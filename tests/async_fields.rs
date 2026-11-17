use cmi_docx::*;
use futures::{channel::oneshot, executor::block_on, task::noop_waker};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Context,
};
fn pending(
    started: &Arc<AtomicUsize>,
    senders: &mut Vec<oneshot::Sender<()>>,
    text: &str,
) -> Value<String> {
    let (tx, rx) = oneshot::channel();
    senders.push(tx);
    let count = Arc::clone(started);
    let text = text.to_owned();
    Value::future(async move {
        count.fetch_add(1, Ordering::SeqCst);
        rx.await.unwrap();
        text
    })
}
#[test]
fn run_and_paragraph_text_and_comment_fields_resolve_concurrently() {
    for paragraph_comment in [false, true] {
        let started = Arc::new(AtomicUsize::new(0));
        let mut senders = Vec::new();
        let text = pending(&started, &mut senders, "text");
        let comment = pending(&started, &mut senders, "comment");
        let author = pending(&started, &mut senders, "author");
        let paragraph = if paragraph_comment {
            Paragraph {
                comment_text: Some(comment),
                comment_author: Some(author),
                ..Paragraph::new(text)
            }
        } else {
            Paragraph::from_children(vec![
                TextRun {
                    comment_text: Some(comment),
                    comment_author: Some(author),
                    ..TextRun::new(text)
                }
                .into(),
            ])
        };
        let mut resolution =
            Box::pin(Document::new(vec![Section::new(vec![paragraph.into()]).into()]).resolve());
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(resolution.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 3);
        for sender in senders {
            sender.send(()).unwrap();
        }
        let resolved = block_on(resolution).unwrap();
        assert_eq!(resolved.sections()[0].paragraphs()[0].text(), "text");
        assert!(!resolved.to_docx().unwrap().is_empty());
    }
}
#[test]
fn section_content_headers_and_footers_resolve_concurrently() {
    let started = Arc::new(AtomicUsize::new(0));
    let mut senders = Vec::new();
    let body = pending(&started, &mut senders, "body");
    let header = pending(&started, &mut senders, "header");
    let footer = pending(&started, &mut senders, "footer");
    let mut section = Section::new(vec![Paragraph::new(body).into()]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new(header).into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::new(footer).into()]).into(),
    );
    let mut resolution = Box::pin(Document::new(vec![section.into()]).resolve());
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(resolution.as_mut().poll(&mut cx).is_pending());
    assert_eq!(started.load(Ordering::SeqCst), 3);
    for sender in senders {
        sender.send(()).unwrap();
    }
    assert_eq!(
        block_on(resolution).unwrap().sections()[0].paragraphs()[0].text(),
        "body"
    );
}
#[test]
fn standalone_components_and_documents_can_be_awaited() {
    block_on(async {
        assert_eq!(TextRun::new("run").await.unwrap().unwrap().text(), "run");
        assert!(
            TextRun::new(Value::lazy(|| panic!("hidden")))
                .when(false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(Tab::default().await.unwrap().unwrap().text(), "\t");
        assert_eq!(Break::default().await.unwrap().unwrap().text(), "\n");
        assert!(Table::default().await.unwrap().is_none());
        assert!(
            Header {
                children: Value::lazy(|| panic!("hidden header")),
                ..Header::default()
            }
            .when(false)
            .await
            .unwrap()
            .is_none()
        );
        assert!(
            Footer {
                children: Value::lazy(|| panic!("hidden footer")),
                ..Footer::default()
            }
            .when(false)
            .await
            .unwrap()
            .is_none()
        );
        assert_eq!(
            Paragraph::new("paragraph")
                .resolve()
                .await
                .unwrap()
                .unwrap()
                .text(),
            "paragraph"
        );
        assert_eq!(Document::default().await.unwrap().sections().len(), 0);
    });
}
