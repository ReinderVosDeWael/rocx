mod support;

use futures::{channel::oneshot, executor::block_on, future::try_join_all, task::noop_waker};
use rocx::{Deferred, Document, Error, Metadata, Paragraph, Section, Value};
use std::{
    collections::BTreeMap,
    error::Error as _,
    future::Future,
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Context,
};

#[test]
fn deferred_metadata_writes_string_and_boolean_custom_properties() {
    let metadata = Value::future(async {
        Metadata::default()
            .with_custom_properties(BTreeMap::from([("Middle".into(), "text".into())]))
            .with_custom_boolean_properties(BTreeMap::from([
                ("Alpha".into(), true),
                ("Zulu".into(), false),
            ]))
    });
    let document = block_on(Document::default().with_metadata(metadata).resolve()).unwrap();
    let xml = support::part(&document.to_docx().unwrap(), "docProps/custom.xml");
    assert!(xml.contains(">true</vt:bool>"));
    assert!(xml.contains(">false</vt:bool>"));
    assert!(xml.contains(">text</vt:lpwstr>"));
    assert!(xml.find("Alpha") < xml.find("Middle"));
    assert!(xml.find("Middle") < xml.find("Zulu"));
}

#[test]
fn deferred_metadata_rejects_duplicate_names_across_variants() {
    let metadata = Value::future(async {
        Metadata::default()
            .with_custom_properties(BTreeMap::from([("Same".into(), "text".into())]))
            .with_custom_boolean_properties(BTreeMap::from([("Same".into(), true)]))
    });
    let error = block_on(Document::default().with_metadata(metadata).resolve()).unwrap_err();
    assert!(error.to_string().contains("both a string and a boolean"));
}

#[test]
fn paragraph_dependencies_share_one_producer_in_any_declaration_order() {
    for source_first in [true, false] {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let source = Value::future(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            "42 λ".to_owned()
        })
        .deferred();
        let input = source.clone();
        let summary =
            Value::try_future(async move { Ok::<_, Error>(format!("Result: {}", input.await?)) })
                .deferred();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let source_paragraph = Paragraph::new(source.clone());
        let consumer = Paragraph::new(summary.clone());
        let blocks = if source_first {
            vec![source_paragraph.into(), consumer.into()]
        } else {
            vec![consumer.into(), source_paragraph.into()]
        };
        let resolved =
            block_on(Document::new(vec![Section::new(blocks).into()]).resolve()).unwrap();
        let expected = if source_first {
            vec!["42 λ", "Result: 42 λ"]
        } else {
            vec!["Result: 42 λ", "42 λ"]
        };
        assert_eq!(support::texts(&resolved.to_docx().unwrap()), expected);
        assert_eq!(block_on(source.resolve()).unwrap(), "42 λ");
        assert_eq!(block_on(summary.resolve()).unwrap(), "Result: 42 λ");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn any_cloneable_value_type_can_be_shared_and_composed() {
    let numbers = Deferred::new(Value::future(async { vec![2_u64, 3, 5] }));
    let input = numbers.clone();
    let sum = Value::try_future(async move { Ok::<_, Error>(input.await?.iter().sum::<u64>()) })
        .deferred();
    let input = sum.clone();
    let text = Value::try_future(async move { Ok::<_, Error>(format!("sum={}", input.await?)) });
    let resolved = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph {
                    comment_text: Some(
                        Value::future(async { "numeric dependency".to_owned() })
                            .deferred()
                            .into(),
                    ),
                    ..Paragraph::new(text)
                }
                .into(),
            ])
            .into(),
        ])
        .resolve(),
    )
    .unwrap();
    let data = resolved.to_docx().unwrap();
    assert_eq!(support::texts(&data), ["sum=10"]);
    assert!(support::part(&data, "word/comments.xml").contains("numeric dependency"));
    assert_eq!(block_on(sum.resolve()).unwrap(), 10);
    assert_eq!(block_on(numbers.resolve()).unwrap(), [2, 3, 5]);
}

#[test]
fn concurrent_fanout_and_chains_do_not_repeat_producers() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let base = Value::lazy(move || {
        count.fetch_add(1, Ordering::SeqCst);
        7
    })
    .deferred();
    let branches: Vec<_> = (0..20)
        .map(|i| {
            let input = base.clone();
            Value::try_future(async move { Ok::<_, Error>(input.await? + i) }).deferred()
        })
        .collect();
    let results = block_on(try_join_all(branches.iter().map(Deferred::resolve))).unwrap();
    assert_eq!(results, (7..27).collect::<Vec<_>>());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn shared_failures_preserve_original_error_and_run_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let source = Value::try_lazy(move || {
        count.fetch_add(1, Ordering::SeqCst);
        Err::<String, _>(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "deferred sentinel",
        ))
    })
    .deferred();
    let first = block_on(source.resolve()).unwrap_err();
    let second = block_on(source.resolve()).unwrap_err();
    let (Error::Shared(a), Error::Shared(b)) = (&first, &second) else {
        panic!("expected shared errors")
    };
    assert!(Arc::ptr_eq(a, b));
    assert_eq!(
        first.to_string(),
        "content producer failed: deferred sentinel"
    );
    let original = first
        .source()
        .unwrap()
        .source()
        .unwrap()
        .downcast_ref::<io::Error>()
        .unwrap();
    assert_eq!(original.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let error = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new(source).into()]).into(),
        ])
        .resolve(),
    )
    .unwrap_err();
    assert_eq!(error.path(), Some("sections[0].children[0].text"));
}

#[test]
fn hidden_fields_leave_shared_producers_unpolled() {
    let source: Deferred<String> = Value::lazy(|| panic!("hidden dependency ran")).deferred();
    let input = source.clone();
    let hidden = Paragraph::new(Value::try_future(async move { input.await })).when(false);
    let resolved =
        block_on(Document::new(vec![Section::new(vec![hidden.into()]).into()]).resolve()).unwrap();
    assert!(support::texts(&resolved.to_docx().unwrap()).is_empty());
    drop(source);
}

#[test]
fn dropping_one_waiter_keeps_other_deferred_consumers_usable() {
    let (sender, receiver) = oneshot::channel();
    let shared = Value::future(async { receiver.await.unwrap() }).deferred();
    let mut first = Box::pin(shared.resolve());
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(first.as_mut().poll(&mut context).is_pending());
    drop(first);
    sender.send("done".to_owned()).unwrap();
    assert_eq!(block_on(shared.resolve()).unwrap(), "done");
}

#[test]
fn shared_property_can_move_to_another_thread() {
    fn send_sync<T: Send + Sync>(_: &T) {}
    let shared = Value::future(async { 123 }).deferred();
    send_sync(&shared);
    assert_eq!(
        std::thread::spawn(move || block_on(shared.resolve()).unwrap())
            .join()
            .unwrap(),
        123
    );
}

#[test]
fn typed_property_references_detect_cycles_and_honor_excluded_sources() {
    use rocx::{Property, PropertyRef, Rule};
    use std::sync::Mutex;
    let dependency = Arc::new(Mutex::new(None::<PropertyRef<String>>));
    let slot = dependency.clone();
    let first = Property::new(
        "first.text",
        Value::try_future(async move {
            let next = slot.lock().unwrap().clone().unwrap();
            next.await
        }),
    );
    let reference = first.reference();
    let second = Property::new(
        "second.text",
        Value::try_future(async move { reference.await }),
    );
    *dependency.lock().unwrap() = Some(second.reference());
    let result = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph::new(first).into(),
                Paragraph::new(second).into(),
            ])
            .into(),
        ])
        .resolve(),
    );
    assert_eq!(result.unwrap_err().rule(), Some(Rule::DependencyCycle));
    let excluded = Property::new(
        "hidden.text",
        Value::lazy(|| panic!("excluded property ran")),
    )
    .when(false);
    let reference = excluded.reference();
    let result = block_on(
        Document::new(vec![
            Section::new(vec![Paragraph::new(reference).into()]).into(),
        ])
        .resolve(),
    );
    assert_eq!(result.unwrap_err().rule(), Some(Rule::ExcludedDependency));
}
