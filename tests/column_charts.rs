use futures::executor::block_on;
use rocx::{
    ColumnChartProperties, ColumnChartRun, Document, Inline, Paragraph, ResolvedBlock,
    ResolvedInline, Rule, Section, Value,
};

fn properties() -> ColumnChartProperties {
    ColumnChartProperties {
        width: 144.,
        height: 72.,
        categories: vec!["North".into(), "South".into()],
        values: vec![8., -13.5],
    }
}

fn document(children: Vec<Value<Inline>>) -> Document {
    Document::new(vec![
        Section::new(vec![Paragraph::from_children(children).into()]).into(),
    ])
}

#[test]
fn deferred_chart_resolves_to_a_chart_run_without_paragraph_text() {
    let visible = ColumnChartRun::new(properties())
        .with_properties(Value::future(async { properties().with_width(150.) }));
    let hidden =
        ColumnChartRun::new(Value::lazy(|| panic!("hidden chart properties ran"))).when(false);
    let snapshot = block_on(document(vec![visible.into(), hidden.into()]).resolve()).unwrap();
    let ResolvedBlock::Paragraph(paragraph) = &snapshot.sections()[0].blocks()[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        paragraph.runs(),
        &[ResolvedInline::ColumnChart(properties().with_width(150.))]
    );
    assert_eq!(paragraph.text(), "");
}

#[test]
fn invalid_chart_diagnostic_identifies_the_value() {
    let chart = ColumnChartRun::new(properties().with_values(vec![8., f64::NAN]));
    let error = block_on(document(vec![chart.into()]).resolve()).unwrap_err();
    assert_eq!(error.rule(), Some(Rule::InvalidContent));
    assert!(error.to_string().contains("properties.values[1]"));
}
