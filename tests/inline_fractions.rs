use futures::executor::block_on;
use rocx::{
    Document, Inline, InlineFraction, Paragraph, ResolvedBlock, ResolvedInline, Rule, Section,
    Value,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn document(children: Vec<Value<Inline>>) -> Document {
    Document::new(vec![
        Section::new(vec![Paragraph::from_children(children).into()]).into(),
    ])
}

#[test]
fn deferred_operands_resolve_once_and_hidden_operands_are_not_evaluated() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let visible = InlineFraction::new("unused", "unused")
        .with_numerator(Value::future(async move {
            count.fetch_add(1, Ordering::SeqCst);
            " 1 & π ".into()
        }))
        .with_denominator("2");
    let hidden = InlineFraction::new(
        Value::lazy(|| panic!("hidden numerator ran")),
        Value::lazy(|| panic!("hidden denominator ran")),
    )
    .when(false);
    let snapshot = block_on(document(vec![visible.into(), hidden.into()]).resolve()).unwrap();
    let ResolvedBlock::Paragraph(paragraph) = &snapshot.sections()[0].blocks()[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        paragraph.runs(),
        &[ResolvedInline::Fraction {
            numerator: " 1 & π ".into(),
            denominator: "2".into(),
        }]
    );
    assert_eq!(snapshot.to_docx().unwrap(), snapshot.to_docx().unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn invalid_fraction_operand_is_reported_at_its_field() {
    for (operand, rule) in [
        ("", Rule::InvalidContent),
        ("2\n3", Rule::InvalidContent),
        ("2\u{2028}3", Rule::InvalidContent),
        ("2\0", Rule::InvalidXml),
    ] {
        let error = block_on(document(vec![InlineFraction::new("1", operand).into()]).resolve())
            .unwrap_err();
        assert_eq!(error.rule(), Some(rule));
        assert!(error.to_string().contains("denominator"));
    }
}
