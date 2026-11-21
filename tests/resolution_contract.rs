//! R1 shared scenarios; see docs/DEFERRED-FIELDS.md for ownership differences.
mod support;

use futures::{channel::oneshot, executor::block_on, future::try_join, task::noop_waker};
use rocx::*;
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::Context,
};

#[test]
fn image_fields_accept_references_before_the_source_and_snapshots_do_not_rerun_them() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let data = Property::new(
        "source.data",
        Value::lazy(move || {
            count.fetch_add(1, Ordering::SeqCst);
            ImageData::Bytes(include_bytes!("fixtures/images/sample.png").to_vec())
        }),
    );
    let transformation = Property::new(
        "source.transformation",
        Value::future(async {
            Some(ImageTransformation {
                width: Some(36.),
                height: Some(18.),
            })
        }),
    );
    let alt_text = Property::new(
        "source.alt_text",
        Value::future(async {
            Some(AltText {
                title: Some("λ picture".into()),
                ..Default::default()
            })
        }),
    );
    let mut copy = ImageRun::deferred(data.reference().into());
    copy.transformation = transformation.reference().into();
    copy.alt_text = alt_text.reference().into();
    let mut source = ImageRun::deferred(data.into());
    source.transformation = transformation.into();
    source.alt_text = alt_text.into();
    let snapshot = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph::from_children(vec![copy.into(), source.into()]).into(),
            ])
            .into(),
        ])
        .resolve(),
    )
    .unwrap();
    for run in snapshot.sections()[0].paragraphs()[0].runs() {
        assert_eq!(run.image().unwrap().dimensions_emu(), (457200, 228600));
    }
    let first = snapshot.to_docx().unwrap();
    assert_eq!(first, snapshot.to_docx().unwrap());
    let xml = support::part(&first, "word/document.xml");
    let tree = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        tree.descendants()
            .filter(|node| node.attribute("title") == Some("λ picture"))
            .count(),
        2
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn image_fields_start_concurrently_and_hidden_images_do_not_poll_options() {
    let started = Arc::new(AtomicUsize::new(0));
    let (data_tx, data_rx) = oneshot::channel();
    let (transform_tx, transform_rx) = oneshot::channel();
    let (alt_tx, alt_rx) = oneshot::channel();
    let count = started.clone();
    let mut image = ImageRun::deferred(Value::future(async move {
        count.fetch_or(1, Ordering::SeqCst);
        data_rx.await.unwrap()
    }));
    let count = started.clone();
    image.transformation = Value::future(async move {
        count.fetch_or(2, Ordering::SeqCst);
        transform_rx.await.unwrap()
    });
    let count = started.clone();
    image.alt_text = Value::future(async move {
        count.fetch_or(4, Ordering::SeqCst);
        alt_rx.await.unwrap()
    });
    let mut work = Box::pin(Component::resolve(image));
    let waker = noop_waker();
    assert!(
        work.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    assert_eq!(started.load(Ordering::SeqCst), 7);
    data_tx
        .send(ImageData::Bytes(
            include_bytes!("fixtures/images/sample.png").to_vec(),
        ))
        .unwrap();
    transform_tx.send(None).unwrap();
    alt_tx.send(None).unwrap();
    assert!(block_on(work).unwrap().is_some());

    let mut hidden = ImageRun::deferred(Value::lazy(|| panic!("hidden data ran"))).when(false);
    hidden.transformation = Value::lazy(|| panic!("hidden transformation ran"));
    hidden.alt_text = Value::lazy(|| panic!("hidden alt text ran"));
    assert!(block_on(Component::resolve(hidden)).unwrap().is_none());
}

#[test]
fn image_option_producer_errors_have_the_field_path_and_original_cause() {
    use std::error::Error as _;
    for field in ["transformation", "alt_text"] {
        let mut image = ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec());
        let failure =
            || std::io::Error::new(std::io::ErrorKind::PermissionDenied, "option sentinel");
        if field == "transformation" {
            image.transformation =
                Value::try_lazy(move || Err::<Option<ImageTransformation>, _>(failure()));
        } else {
            image.alt_text = Value::try_lazy(move || Err::<Option<AltText>, _>(failure()));
        }
        let error = block_on(Component::resolve(image)).unwrap_err();
        assert_eq!(error.path(), Some(format!("image_run.{field}").as_str()));
        let cause = error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap();
        assert_eq!(cause.kind(), std::io::ErrorKind::PermissionDenied);
    }
}

#[test]
fn concurrent_root_passes_share_explicit_properties() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let (sender, receiver) = oneshot::channel();
    let property = Property::new(
        "shared",
        Value::future(async move {
            count.fetch_add(1, Ordering::SeqCst);
            receiver.await.unwrap()
        }),
    );
    let mut first = Box::pin(Paragraph::new(property.reference()).resolve());
    let mut second = Box::pin(Paragraph::new(property.reference()).resolve());
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(first.as_mut().poll(&mut context).is_pending());
    assert!(second.as_mut().poll(&mut context).is_pending());
    sender.send("shared λ".to_owned()).unwrap();
    let (first, second) = block_on(try_join(first, second)).unwrap();
    assert_eq!(first.unwrap().text(), "shared λ");
    assert_eq!(second.unwrap().text(), "shared λ");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn diamond_references_share_the_base_producer() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let base = Property::new(
        "base",
        Value::lazy(move || {
            count.fetch_add(1, Ordering::SeqCst);
            "seed".to_owned()
        }),
    );
    let left = Property::new("left", Value::from(base.reference()));
    let right = Property::new("right", Value::from(base.reference()));
    let a = left.reference();
    let b = right.reference();
    let end = Value::try_future(async move {
        let (a, b) = try_join(a.resolve(), b.resolve()).await?;
        Ok::<_, Error>(format!("{a}/{b}"))
    });
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::new(end).into(),
            Paragraph::new(right).into(),
            Paragraph::new(left).into(),
            Paragraph::new(base).into(),
        ])
        .into(),
    ]);
    let snapshot = block_on(document.resolve()).unwrap();
    assert_eq!(
        support::texts(&snapshot.to_docx().unwrap()),
        ["seed/seed", "seed", "seed", "seed"]
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn direct_and_indirect_reference_cycles_fail_without_waiting() {
    for length in 1..=3 {
        let slots: Vec<_> = (0..length)
            .map(|_| Arc::new(Mutex::new(None::<PropertyRef<String>>)))
            .collect();
        let properties: Vec<_> = slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                let slot = slot.clone();
                Property::new(
                    format!("cycle[{index}]"),
                    Value::try_future(async move {
                        let next = slot.lock().unwrap().clone().unwrap();
                        next.await
                    }),
                )
            })
            .collect();
        for (index, slot) in slots.iter().enumerate() {
            *slot.lock().unwrap() = Some(properties[(index + 1) % length].reference());
        }
        let mut pass = Box::pin(Paragraph::new(properties[0].reference()).resolve());
        let waker = noop_waker();
        let result = pass.as_mut().poll(&mut Context::from_waker(&waker));
        assert!(
            matches!(result, std::task::Poll::Ready(Err(error)) if error.rule() == Some(Rule::DependencyCycle))
        );
        // Break the author-supplied Arc cycle rather than leaking the test fixture.
        for slot in &slots {
            slot.lock().unwrap().take();
        }
    }
}

#[test]
fn fresh_programs_rerun_but_explicit_properties_remain_cached() {
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..2 {
        let count = calls.clone();
        let value = Value::lazy(move || {
            count.fetch_add(1, Ordering::SeqCst);
            "fresh".to_owned()
        });
        assert_eq!(
            block_on(Paragraph::new(value).resolve())
                .unwrap()
                .unwrap()
                .text(),
            "fresh"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let count = calls.clone();
    let shared = Property::new(
        "cached",
        Value::lazy(move || {
            count.fetch_add(1, Ordering::SeqCst);
            "cached".to_owned()
        }),
    );
    for _ in 0..2 {
        assert_eq!(
            block_on(Paragraph::new(shared.reference()).resolve())
                .unwrap()
                .unwrap()
                .text(),
            "cached"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn nested_passes_keep_cycle_detection_and_property_guards() {
    let slot = Arc::new(Mutex::new(None::<PropertyRef<String>>));
    let input = slot.clone();
    let property = Property::new(
        "nested",
        Value::try_future(async move {
            let reference = input.lock().unwrap().clone().unwrap();
            let paragraph = Paragraph::new(reference).resolve().await?.unwrap();
            Ok::<_, Error>(paragraph.text().to_owned())
        }),
    );
    *slot.lock().unwrap() = Some(property.reference());
    // Poll once rather than block forever if nested passes lose the dependency graph.
    let mut work = Box::pin(Paragraph::new(property).resolve());
    let waker = noop_waker();
    let result = work.as_mut().poll(&mut Context::from_waker(&waker));
    assert!(
        matches!(result, std::task::Poll::Ready(Err(error)) if error.rule() == Some(Rule::DependencyCycle))
    );

    let property = Property::new("independent", Value::ready("visible".to_owned()));
    let hidden = Paragraph::new(property.reference()).when(false);
    assert!(block_on(hidden.resolve()).unwrap().is_none());
    assert_eq!(block_on(property.reference().resolve()).unwrap(), "visible");
}

#[test]
fn dropping_a_pass_drops_owned_work_but_an_external_property_can_resume() {
    struct Dropped(Arc<AtomicUsize>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let marker = Dropped(drops.clone());
    let value = Value::future(async move {
        let _marker = marker;
        futures::future::pending::<String>().await
    });
    let mut owned = Box::pin(Paragraph::new(value).resolve());
    let waker = noop_waker();
    assert!(
        owned
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    drop(owned);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let (sender, receiver) = oneshot::channel();
    let property = Property::new("external", Value::future(async { receiver.await.unwrap() }));
    let mut pass = Box::pin(Paragraph::new(property.reference()).resolve());
    assert!(
        pass.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    drop(pass);
    sender.send("resumed".to_owned()).unwrap();
    assert_eq!(block_on(property.reference().resolve()).unwrap(), "resumed");
}

#[test]
fn image_snapshots_survive_source_mutation_and_deletion() {
    use std::io::{Cursor, Read};
    let directory = std::env::temp_dir().join(format!("rocx-r1-assets-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("input.png");
    let original = include_bytes!("fixtures/images/sample.png");
    std::fs::write(&path, original).unwrap();
    let document = Document::new(vec![
        Section::new(vec![
            Paragraph::from_children(vec![ImageRun::from_path(&path).into()]).into(),
        ])
        .into(),
    ]);
    let snapshot = block_on(document.resolve()).unwrap();
    let first = snapshot.to_docx().unwrap();
    std::fs::write(&path, b"not an image").unwrap();
    assert_eq!(first, snapshot.to_docx().unwrap());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(first, snapshot.to_docx().unwrap());
    std::fs::remove_dir(directory).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(first)).unwrap();
    let mut payload = Vec::new();
    archive
        .by_name("word/media/image1.png")
        .unwrap()
        .read_to_end(&mut payload)
        .unwrap();
    assert_eq!(payload, original);
}
