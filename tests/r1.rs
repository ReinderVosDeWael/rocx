//! End-to-end R1 contracts beyond the initial image/lifetime checkpoint.
mod support;
use futures::executor::block_on;
use rv_docx_rs::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn metadata_section_table_and_cell_properties_use_the_same_reference_mechanism() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let metadata = Property::new(
        "metadata",
        Value::future(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Metadata {
                title: Some("R1 λ".into()),
                creator: Some("Ada".into()),
                ..Default::default()
            }
        }),
    );
    let title = metadata.reference();
    let paragraph = Paragraph::new(Value::try_future(async move {
        Ok::<_, Error>(title.await?.title.unwrap())
    }));
    let section_props = Property::new(
        "section.properties",
        Value::future(async {
            Some(SectionProperties {
                columns: Some(Columns {
                    count: Some(2),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }),
    );
    let table_props = Property::new(
        "table.properties",
        Value::future(async {
            TableProperties {
                column_widths: Some(vec![1440]),
                layout: Some(TableLayout::Fixed),
                ..Default::default()
            }
        }),
    );
    let cell_props = Property::new(
        "cell.properties",
        Value::future(async {
            TableCellProperties {
                background_color: Some([12, 34, 56]),
                grid_span: Some(1),
                ..Default::default()
            }
        }),
    );
    let mut cell = TableCell::new(vec![Paragraph::new("cell").into()]);
    cell.properties = cell_props.reference().into();
    let mut table = Table::new(vec![TableRow::new(vec![cell.into()]).into()]);
    table.properties = table_props.reference().into();
    let mut section = Section::new(vec![paragraph.into(), table.into()]);
    section.properties = section_props.reference().into();
    let mut document = Document::new(vec![section.into()]);
    document.metadata = metadata.into();
    document.timestamp = Value::future(async { Some("2026-11-18T00:00:00Z".into()) });
    document.comment_author = Value::future(async { Some("Grace".into()) });
    document.styles = Property::new("styles", Value::ready(Vec::new())).into();
    document.numbering = Property::new("numbering", Value::ready(Vec::new())).into();
    let snapshot = block_on(document.resolve()).unwrap();
    assert_eq!(snapshot.metadata().title.as_deref(), Some("R1 λ"));
    assert_eq!(snapshot.timestamp(), "2026-11-18T00:00:00Z");
    assert_eq!(snapshot.comment_author(), Some("Grace"));
    assert_eq!(
        snapshot.sections()[0]
            .properties()
            .unwrap()
            .columns
            .as_ref()
            .unwrap()
            .count,
        Some(2)
    );
    let ResolvedBlock::Table(table) = &snapshot.sections()[0].blocks()[1] else {
        panic!("missing table")
    };
    assert_eq!(table.column_widths(), Some([1440].as_slice()));
    assert_eq!(
        table.rows()[0].cells()[0].background_color(),
        Some([12, 34, 56])
    );
    let bytes = snapshot.to_docx().unwrap();
    assert!(support::part(&bytes, "docProps/core.xml").contains("R1 λ"));
    assert_eq!(support::texts(&bytes), ["R1 λ"]);
    assert_eq!(bytes, snapshot.to_docx().unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn immutable_resolved_children_can_be_shared_without_cloning_futures() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let children = Property::new(
        "children",
        Value::try_future(async move {
            count.fetch_add(1, Ordering::SeqCst);
            let paragraph = Paragraph::new("shared children λ")
                .resolve()
                .await?
                .unwrap();
            Ok::<_, Error>(vec![ResolvedBlock::Paragraph(paragraph)])
        }),
    );
    let content = |reference| {
        Value::from(reference).map(|values: Vec<ResolvedBlock>| {
            values
                .into_iter()
                .map(Into::into)
                .collect::<BlockChildren>()
        })
    };
    let cell = TableCell {
        children: content(children.reference()),
        ..Default::default()
    };
    let header = Header {
        children: content(children.reference()),
        ..Default::default()
    };
    let mut section = Section::new(vec![
        Table::new(vec![TableRow::new(vec![cell.into()]).into()]).into(),
    ]);
    section
        .headers
        .insert(HeaderFooterType::Default, header.into());
    let body = Section::default().with_children(content(children.reference()));
    let snapshot = block_on(Document::new(vec![section.into(), body.into()]).resolve()).unwrap();
    assert_eq!(
        snapshot.sections()[1].paragraphs()[0].text(),
        "shared children λ"
    );
    assert_eq!(
        snapshot.sections()[0].headers()[&HeaderFooterType::Default]
            .blocks()
            .len(),
        1
    );
    let bytes = snapshot.to_docx().unwrap();
    assert!(support::part(&bytes, "word/header1.xml").contains("shared children λ"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let run = block_on(TextRun::new("shared inline").resolve())
        .unwrap()
        .unwrap();
    let runs = Property::new("inlines", Value::ready(vec![run]));
    let snapshot = block_on(
        Paragraph::default()
            .with_children(
                Value::from(runs.reference()).map(|values: Vec<ResolvedInline>| {
                    values
                        .into_iter()
                        .map(Into::into)
                        .collect::<InlineChildren>()
                }),
            )
            .resolve(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(snapshot.text(), "shared inline");
}

#[test]
fn pure_invariants_fail_before_serialization_with_structured_locations() {
    for value in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
        let error = RunFormat {
            size: Some(value),
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(error.rule, Rule::InvalidMeasurement);
        assert_eq!(error.location.as_deref(), Some("size"));
    }
    let error = block_on(
        Paragraph::new("invalid")
            .with_format(ParagraphFormat {
                spacing_before: Some(-1.),
                ..Default::default()
            })
            .resolve(),
    )
    .unwrap_err();
    assert_eq!(error.rule().unwrap().code(), "invalid_measurement");
    assert_eq!(error.path(), Some("paragraph.format.spacing_before"));
    let error = TableCellProperties {
        grid_span: Some(0),
        ..Default::default()
    }
    .validate()
    .unwrap_err();
    assert_eq!(error.rule, Rule::InvalidContent);
    assert_eq!(error.location.as_deref(), Some("grid_span"));
    assert!(
        SectionProperties {
            columns: Some(Columns {
                count: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert_eq!(Emu::new(i64::MAX).twips().get(), 14_524_995_333_629_568);
    assert_eq!(Emu::new(-635).twips().get(), -1);
}

#[test]
fn aggregate_property_guards_and_component_conditions_remain_distinct() {
    let property = Property::new(
        "hidden.properties",
        Value::lazy(|| panic!("excluded property ran")),
    )
    .when(false);
    let section = Section {
        properties: property.reference().into(),
        ..Default::default()
    };
    let error = block_on(Document::new(vec![section.into()]).resolve()).unwrap_err();
    assert_eq!(error.rule(), Some(Rule::ExcludedDependency));
    assert_eq!(error.path(), Some("sections[0].properties"));
    let mut section = Section::default().when(false);
    section.properties = Value::lazy(|| panic!("hidden section properties ran"));
    let mut table = Table::default().when(false);
    table.properties = Value::lazy(|| panic!("hidden table properties ran"));
    let mut cell = TableCell::default().when(false);
    cell.properties = Value::lazy(|| panic!("hidden cell properties ran"));
    assert!(block_on(section.resolve()).unwrap().is_none());
    assert!(block_on(table.resolve()).unwrap().is_none());
    assert!(block_on(cell.resolve()).unwrap().is_none());
}

#[test]
fn render_options_override_without_mutating_the_snapshot() {
    let mut document = Document::new(vec![
        Section::new(vec![Paragraph::new("options").into()]).into(),
    ]);
    document.timestamp = Some("2026-11-18T00:00:00Z".into()).into();
    let snapshot = block_on(document.resolve()).unwrap();
    let options = application::RenderOptions {
        timestamp: Some("2026-01-01T00:00:00Z".into()),
        ..Default::default()
    };
    let overridden = snapshot.to_docx_with_options(&options).unwrap();
    assert!(support::part(&overridden, "docProps/core.xml").contains("2026-01-01T00:00:00Z"));
    assert_eq!(snapshot.timestamp(), "2026-11-18T00:00:00Z");
    let strict = application::RenderOptions {
        profile: OutputProfile::Strict,
        ..Default::default()
    };
    assert_eq!(
        snapshot.to_docx_with_options(&strict).unwrap_err().rule(),
        Some(Rule::UnsupportedProfile)
    );
    let limits = application::RenderOptions {
        read_limits: opc::ReadLimits {
            max_parts: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        snapshot.to_docx_with_options(&limits).unwrap_err().rule(),
        Some(Rule::InvalidPackage)
    );
}
