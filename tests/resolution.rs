mod support;

use std::{
    future::Future,
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use cmi_docx::{Block, Condition, Document, Error, Paragraph, Section, Value};
use futures::{channel::oneshot, executor::block_on, future, task::noop_waker};
use support::texts;

fn bytes(document: Document) -> Vec<u8> {
    block_on(document.resolve()).unwrap().to_docx().unwrap()
}
fn wrap(paragraph: Paragraph) -> Document {
    Document::new(vec![Section::new(vec![paragraph.into()]).into()])
}

#[test]
fn immediate_lazy_async_and_nested_values_have_equivalent_results() {
    let paragraphs = [
        Paragraph::new("same"),
        Paragraph::new(Value::lazy(|| "same".to_owned())),
        Paragraph::new(Value::future(async { "same".to_owned() })),
        Paragraph::new(Value::Lazy(Box::new(|| {
            Ok(Value::future(async { "same".into() }))
        }))),
    ];
    let expected = bytes(wrap(Paragraph::new("same")));
    for paragraph in paragraphs {
        assert_eq!(bytes(wrap(paragraph)), expected);
    }
}

#[test]
fn deferred_sections_child_lists_and_individual_blocks_stay_in_the_tree() {
    let document = Document::new(vec![Value::future(async {
        Section::default().with_children(Value::lazy(|| {
            vec![
                Value::lazy(|| Block::from(Paragraph::new("lazy block"))),
                Value::future(async { Paragraph::new("async block").into() }),
            ]
        }))
    })]);
    assert_eq!(texts(&bytes(document)), ["lazy block", "async block"]);
}

#[test]
fn false_conditions_never_invoke_lazy_producers_or_poll_async_fields() {
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::new(Value::lazy(|| panic!("hidden lazy text")))
                .when(false)
                .into(),
            Paragraph::new(Value::future(async { panic!("hidden async text") }))
                .when(false)
                .into(),
            Paragraph::new("visible").into(),
        ])
        .into(),
        Section::default()
            .with_children(Value::lazy(|| panic!("hidden lazy children")))
            .when(false)
            .into(),
        Section::default()
            .with_children(Value::future(async { panic!("hidden async children") }))
            .when(false)
            .into(),
    ]);
    let output = bytes(document);
    assert_eq!(texts(&output), ["visible"]);
    assert_eq!(
        support::body(&support::part(&output, "word/document.xml")).section_count,
        1
    );
}

#[test]
fn ancestor_conditions_skip_descendant_conditions_and_block_producers() {
    let document = Document::new(vec![
        Section::new(vec![
            Value::lazy(|| panic!("hidden block")),
            Paragraph::new("hidden")
                .when(Condition::lazy(|| panic!("hidden predicate")))
                .into(),
        ])
        .when(false)
        .into(),
    ]);
    let resolved = block_on(document.resolve()).unwrap();
    assert!(resolved.sections().is_empty());
}

#[test]
fn conditions_and_producers_run_once_even_with_repeated_rendering() {
    let predicates = Arc::new(AtomicUsize::new(0));
    let producers = Arc::new(AtomicUsize::new(0));
    let p = Arc::clone(&predicates);
    let t = Arc::clone(&producers);
    let s = Arc::clone(&predicates);
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::new(Value::lazy(move || {
                t.fetch_add(1, Ordering::SeqCst);
                "once".into()
            }))
            .when(Condition::lazy(move || {
                p.fetch_add(1, Ordering::SeqCst);
                true
            }))
            .into(),
        ])
        .when(Condition::lazy(move || {
            s.fetch_add(1, Ordering::SeqCst);
            true
        }))
        .into(),
    ]);
    let resolved = block_on(document.resolve()).unwrap();
    for _ in 0..3 {
        assert_eq!(texts(&resolved.to_docx().unwrap()), ["once"]);
    }
    assert_eq!(predicates.load(Ordering::SeqCst), 2);
    assert_eq!(producers.load(Ordering::SeqCst), 1);
}

#[test]
fn sibling_text_futures_start_concurrently_and_retain_declaration_order() {
    let started = Arc::new(AtomicUsize::new(0));
    let mut senders = Vec::new();
    let children = (0..3)
        .map(|i| {
            let (tx, rx) = oneshot::channel();
            senders.push(tx);
            let started = Arc::clone(&started);
            Paragraph::new(Value::future(async move {
                started.fetch_add(1, Ordering::SeqCst);
                rx.await.unwrap();
                format!("paragraph {i}")
            }))
            .into()
        })
        .collect();
    let mut resolution = Box::pin(Document::new(vec![Section::new(children).into()]).resolve());
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(matches!(resolution.as_mut().poll(&mut cx), Poll::Pending));
    assert_eq!(started.load(Ordering::SeqCst), 3);
    for (index, tx) in senders.into_iter().rev().enumerate() {
        tx.send(()).unwrap();
        if index < 2 {
            // Let later declarations complete while earlier ones are still pending.
            assert!(resolution.as_mut().poll(&mut cx).is_pending());
        }
    }
    let resolved = block_on(resolution).unwrap();
    assert_eq!(
        texts(&resolved.to_docx().unwrap()),
        ["paragraph 0", "paragraph 1", "paragraph 2"]
    );
}

#[test]
fn sibling_section_and_block_futures_start_concurrently() {
    for deferred_sections in [false, true] {
        let started = Arc::new(AtomicUsize::new(0));
        let mut senders = Vec::new();
        let mut section_values = Vec::new();
        let mut block_values = Vec::new();
        for i in 0..3 {
            let (tx, rx) = oneshot::channel::<()>();
            senders.push(tx);
            let started = Arc::clone(&started);
            let content = async move {
                started.fetch_add(1, Ordering::SeqCst);
                rx.await.unwrap();
                Paragraph::new(format!("{i}"))
            };
            if deferred_sections {
                section_values.push(Value::future(async move {
                    Section::new(vec![content.await.into()])
                }));
            } else {
                block_values.push(Value::future(async move { Block::from(content.await) }));
            }
        }
        let document = if deferred_sections {
            Document::new(section_values)
        } else {
            Document::new(vec![Section::new(block_values).into()])
        };
        let mut resolution = Box::pin(document.resolve());
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(resolution.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 3);
        for (index, tx) in senders.into_iter().rev().enumerate() {
            tx.send(()).unwrap();
            if index < 2 {
                assert!(resolution.as_mut().poll(&mut cx).is_pending());
            }
        }
        let resolved = block_on(resolution).unwrap();
        assert_eq!(texts(&resolved.to_docx().unwrap()), ["0", "1", "2"]);
    }
}

fn failure() -> io::Error {
    io::Error::other("source unavailable")
}

#[test]
fn producer_errors_report_the_exact_tree_field_and_retain_the_cause() {
    let cases = [
        (
            Document::new(vec![Value::try_future(async {
                Err::<Section, _>(failure())
            })]),
            "sections[0]",
        ),
        (
            Document::new(vec![
                Section::default()
                    .with_children(Value::try_lazy(|| Err::<Vec<Value<Block>>, _>(failure())))
                    .into(),
            ]),
            "sections[0].children",
        ),
        (
            Document::new(vec![
                Section::new(vec![Value::try_future(async {
                    Err::<Block, _>(failure())
                })])
                .into(),
            ]),
            "sections[0].children[0]",
        ),
        (
            wrap(Paragraph::new(Value::try_lazy(|| {
                Err::<String, _>(failure())
            }))),
            "sections[0].children[0].text",
        ),
        (
            wrap(Paragraph::new(Value::try_future(async {
                Err::<String, _>(failure())
            }))),
            "sections[0].children[0].text",
        ),
    ];
    for (document, path) in cases {
        let error = block_on(document.resolve()).unwrap_err();
        assert_eq!(error.path(), Some(path));
        assert!(error.to_string().contains("source unavailable"));
        let Error::At { source, .. } = error else {
            panic!("Missing field context");
        };
        let Error::Producer(source) = *source else {
            panic!("Missing producer cause");
        };
        assert!(source.downcast_ref::<io::Error>().is_some());
    }
}

#[test]
fn an_error_drops_pending_sibling_futures() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicUsize::new(0));
    let guard = Guard(Arc::clone(&dropped));
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::new(Value::future(async move {
                let _guard = guard;
                future::pending::<String>().await
            }))
            .into(),
            Paragraph::new(Value::try_future(async { Err::<String, _>(failure()) })).into(),
        ])
        .into(),
    ]);
    assert!(block_on(document.resolve()).is_err());
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn the_tree_and_resolution_future_can_move_between_threads() {
    fn assert_send<T: Send>(_: T) {}
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send(wrap(Paragraph::new(Value::future(async { "text".into() }))));
    assert_send(Document::default().resolve());
    assert_send_sync::<cmi_docx::ResolvedDocument>();
}
