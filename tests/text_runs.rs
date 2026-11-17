mod support;

use std::{
    future::Future,
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Context,
};

use cmi_docx::{Condition, Document, Error, Inline, Paragraph, Section, TextRun, Value};
use futures::{channel::oneshot, executor::block_on, task::noop_waker};
use support::{W, body, part};

fn document(paragraphs: Vec<Paragraph>) -> Document {
    Document::new(vec![
        Section::new(paragraphs.into_iter().map(Into::into).collect()).into(),
    ])
}

fn runs(xml: &str) -> Vec<Vec<String>> {
    let doc = roxmltree::Document::parse(xml).unwrap();
    doc.descendants()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            p.children()
                .filter(|n| n.has_tag_name((W, "r")))
                .map(|run| {
                    let mut text = String::new();
                    for n in run.descendants() {
                        if n.has_tag_name((W, "t")) {
                            text.push_str(n.text().unwrap_or(""));
                        } else if n.has_tag_name((W, "tab")) {
                            text.push('\t');
                        } else if n.has_tag_name((W, "br")) {
                            text.push('\n');
                        }
                    }
                    text
                })
                .collect()
        })
        .collect()
}

#[test]
fn run_boundaries_empty_runs_and_text_match_the_python_reference() {
    let declaration = document(vec![
        Paragraph::from_children(vec![
            TextRun::new("First ").into(),
            TextRun::new("").into(),
            TextRun::new("hidden").when(false).into(),
            TextRun::new("café & <world>\t\n").into(),
        ]),
        Paragraph::default().with_children(Value::lazy(|| {
            vec![
                TextRun::new(Value::lazy(|| "Lazy ".into())).into(),
                TextRun::new(Value::future(async { "async".into() })).into(),
            ]
        })),
        Paragraph::from_children(vec![]),
        Paragraph::new(""),
    ]);
    let resolved = block_on(declaration.resolve()).unwrap();
    let first = &resolved.sections()[0].paragraphs()[0];
    assert_eq!(first.text(), "First café & <world>\t\n");
    assert_eq!(
        first.runs().iter().map(|r| r.text()).collect::<Vec<_>>(),
        ["First ", "", "café & <world>\t\n"]
    );
    let bytes = resolved.to_docx().unwrap();
    let xml = part(&bytes, "word/document.xml");
    let reference = include_str!("fixtures/runs.xml");
    assert_eq!(body(&xml), body(reference));
    assert_eq!(runs(&xml), runs(reference));
    assert_eq!(bytes, resolved.to_docx().unwrap());
}

#[test]
fn inline_producers_and_the_children_list_can_be_deferred() {
    let paragraph = Paragraph::default().with_children(Value::future(async {
        vec![
            Value::lazy(|| Inline::from(TextRun::new("lazy"))),
            Value::future(async { Inline::from(TextRun::new("async")) }),
        ]
    }));
    let resolved = block_on(document(vec![paragraph]).resolve()).unwrap();
    assert_eq!(resolved.sections()[0].paragraphs()[0].text(), "lazyasync");
    assert_eq!(resolved.sections()[0].paragraphs()[0].runs().len(), 2);
}

#[test]
fn false_conditions_skip_run_text_and_paragraph_descendants() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&calls);
    let resolved = block_on(
        document(vec![
            Paragraph::from_children(vec![
                TextRun::new(Value::lazy(|| panic!("hidden lazy text")))
                    .when(false)
                    .into(),
                TextRun::new(Value::future(async { panic!("hidden future text") }))
                    .when(false)
                    .into(),
                TextRun::new("visible")
                    .when(Condition::lazy(move || {
                        count.fetch_add(1, Ordering::SeqCst);
                        true
                    }))
                    .into(),
            ]),
            Paragraph::default()
                .with_children(Value::lazy(|| panic!("hidden run list")))
                .when(false),
            Paragraph::from_children(vec![Value::future(async { panic!("hidden run producer") })])
                .when(false),
        ])
        .resolve(),
    )
    .unwrap();
    assert_eq!(support::texts(&resolved.to_docx().unwrap()), ["visible"]);
    resolved.to_docx().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn run_text_and_inline_futures_resolve_concurrently_in_declaration_order() {
    for deferred_inline in [false, true] {
        let started = Arc::new(AtomicUsize::new(0));
        let mut senders = Vec::new();
        let children = (0..3)
            .map(|index| {
                let (tx, rx) = oneshot::channel::<()>();
                senders.push(tx);
                let started = Arc::clone(&started);
                let text = async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    rx.await.unwrap();
                    index.to_string()
                };
                if deferred_inline {
                    Value::future(async { Inline::from(TextRun::new(text.await)) })
                } else {
                    TextRun::new(Value::future(text)).into()
                }
            })
            .collect();
        let mut resolution = Box::pin(document(vec![Paragraph::from_children(children)]).resolve());
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
        assert_eq!(
            resolved.sections()[0].paragraphs()[0]
                .runs()
                .iter()
                .map(|r| r.text())
                .collect::<Vec<_>>(),
            ["0", "1", "2"]
        );
    }
}

#[test]
fn run_errors_report_the_exact_field() {
    let cases = [
        (
            Paragraph::default().with_children(Value::try_lazy(|| {
                Err::<Vec<Value<Inline>>, _>(io::Error::other("list failed"))
            })),
            "sections[0].children[0].children",
        ),
        (
            Paragraph::from_children(vec![Value::try_future(async {
                Err::<Inline, _>(io::Error::other("run failed"))
            })]),
            "sections[0].children[0].children[0]",
        ),
        (
            Paragraph::from_children(vec![
                TextRun::new(Value::try_future(async {
                    Err::<String, _>(io::Error::other("text failed"))
                }))
                .into(),
            ]),
            "sections[0].children[0].children[0].text",
        ),
        (
            Paragraph::from_children(vec![TextRun::new("bad\0").into()]),
            "sections[0].children[0].children[0].text",
        ),
    ];
    for (paragraph, path) in cases {
        let error = block_on(document(vec![paragraph]).resolve()).unwrap_err();
        assert_eq!(error.path(), Some(path));
    }
}

#[test]
fn conflicting_or_missing_paragraph_content_is_rejected_before_producers_run() {
    for text in [
        None,
        Some(Value::lazy(|| panic!("conflicting text must not resolve"))),
    ] {
        let both = text.is_some();
        let paragraph = Paragraph {
            text,
            children: if both {
                Some(Value::lazy(|| panic!("conflicting list must not resolve")))
            } else {
                None
            },
            condition: true.into(),
            ..Paragraph::default()
        };
        let error = block_on(document(vec![paragraph]).resolve()).unwrap_err();
        assert_eq!(error.path(), Some("sections[0].children[0]"));
        assert!(
            matches!(error,Error::At {source,..} if matches!(*source,Error::InvalidParagraphContent))
        );
    }
}

#[test]
fn paragraph_setters_switch_content_modes() {
    let a = Paragraph::new("replaced").with_children(vec![TextRun::new("runs").into()]);
    let b = Paragraph::from_children(vec![TextRun::new("replaced").into()]).with_text("text");
    let resolved = block_on(document(vec![a, b]).resolve()).unwrap();
    assert_eq!(
        support::texts(&resolved.to_docx().unwrap()),
        ["runs", "text"]
    );
}
