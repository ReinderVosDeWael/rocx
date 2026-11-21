#[path = "support/features.rs"]
mod features;
mod support;
use futures::executor::block_on;
use rocx::*;
use std::io::Cursor;
use support::{W, part};
fn bytes(document: Document) -> Vec<u8> {
    block_on(document.resolve()).unwrap().to_docx().unwrap()
}
fn doc(blocks: BlockChildren) -> Document {
    Document::new(vec![Section::new(blocks).into()])
}
fn normalized(n: roxmltree::Node<'_, '_>) -> Option<String> {
    if !n.is_element() {
        return None;
    }
    let name = n.tag_name().name();
    // Historical CMI omits explicit false small-caps. The native contract now
    // preserves it; separate native assertions cover the override semantics.
    if name == "smallCaps" && n.attribute((W, "val")) == Some("0") {
        return None;
    }
    if [
        "drawing",
        "headerReference",
        "footerReference",
        "commentRangeStart",
        "commentRangeEnd",
    ]
    .contains(&name)
    {
        return None;
    }
    let mut attrs: Vec<_> = n
        .attributes()
        .filter(|a| {
            !a.name().starts_with("rsid")
                && !(a.name() == "space"
                    && a.namespace() == Some("http://www.w3.org/XML/1998/namespace"))
                && !(name == "pgSz" && a.name() == "orient" && a.value() == "portrait")
                && !(name == "br" && a.name() == "type" && a.value() == "textWrapping")
                && !(name == "vMerge" && a.name() == "val" && a.value() == "continue")
                && !(name == "shd"
                    && ((a.name() == "val" && a.value() == "clear")
                        || (a.name() == "color" && a.value() == "auto")))
                && !(a.name() == "val"
                    && a.value() == "1"
                    && [
                        "b",
                        "i",
                        "caps",
                        "smallCaps",
                        "strike",
                        "keepNext",
                        "keepLines",
                        "widowControl",
                        "pageBreakBefore",
                    ]
                    .contains(&name))
        })
        .map(|a| format!("{}={}", a.name(), a.value()))
        .collect();
    attrs.sort();
    let mut children: Vec<_> = n.children().filter_map(normalized).collect();
    if name.ends_with("Pr") || name == "style" {
        children.sort();
    }
    if children.is_empty() && attrs.is_empty() && ["rPr", "pPr"].contains(&name) {
        return None;
    }
    Some(format!(
        "{}[{}]{}({})",
        name,
        attrs.join(","),
        if name == "t" {
            n.text().unwrap_or("")
        } else {
            ""
        },
        children.join("")
    ))
}
#[test]
fn feature_document_matches_python_structure_and_style_definitions() {
    let bytes = bytes(features::document());
    let expected = include_bytes!("fixtures/features.docx");
    let a = part(&bytes, "word/document.xml");
    let b = part(expected, "word/document.xml");
    let a = roxmltree::Document::parse(&a).unwrap();
    let b = roxmltree::Document::parse(&b).unwrap();
    let body = |n: roxmltree::Node<'_, '_>| {
        normalized(
            n.descendants()
                .find(|n| n.has_tag_name((W, "body")))
                .unwrap(),
        )
    };
    assert_eq!(body(a.root()), body(b.root()));
    let a = part(&bytes, "word/styles.xml");
    let b = part(expected, "word/styles.xml");
    let a = roxmltree::Document::parse(&a).unwrap();
    let b = roxmltree::Document::parse(&b).unwrap();
    for name in ["ReportBody", "ReportTable"] {
        let find = |n: roxmltree::Node<'_, '_>| {
            normalized(
                n.descendants()
                    .find(|n| n.attribute((W, "styleId")) == Some(name))
                    .unwrap(),
            )
        };
        assert_eq!(find(a.root()), find(b.root()), "{name}");
    }
}
#[test]
fn all_generated_relationships_resolve_to_real_package_parts() {
    let mut header = Header::new(vec![
        Paragraph::from_children(vec![
            ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec()).into(),
        ])
        .into(),
    ]);
    header.condition = true.into();
    let mut section = Section::new(vec![Paragraph::new("body").into()]);
    section
        .headers
        .insert(HeaderFooterType::Default, header.into());
    let bytes = bytes(Document::new(vec![section.into()]));
    let archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
    for name in names.iter().filter(|n| n.ends_with(".rels")) {
        let xml = part(&bytes, name);
        let parsed = roxmltree::Document::parse(&xml).unwrap();
        for rel in parsed.root_element().children().filter(|n| n.is_element()) {
            if rel.attribute("TargetMode") == Some("External") {
                continue;
            }
            let target = rel.attribute("Target").unwrap();
            let base = name.rsplit_once("/_rels/").map(|(s, _)| s).unwrap_or("");
            let path = if target.starts_with('/') {
                target.trim_start_matches('/').to_owned()
            } else {
                format!("{base}/{target}")
            };
            let mut parts = Vec::new();
            for p in path.split('/') {
                match p {
                    "" | "." => (),
                    ".." => {
                        parts.pop();
                    }
                    p => parts.push(p),
                }
            }
            let path = parts.join("/");
            assert!(names.contains(&path), "{name}: missing {path}");
        }
    }
}
#[test]
fn every_supported_image_format_and_density_is_embedded_without_decoding() {
    for (extension, data) in [
        (
            "png",
            include_bytes!("fixtures/images/sample.png").as_slice(),
        ),
        (
            "png_zero_dpi",
            include_bytes!("fixtures/images/sample_zero_dpi.png").as_slice(),
        ),
        (
            "jpg",
            include_bytes!("fixtures/images/sample.jpg").as_slice(),
        ),
        (
            "gif",
            include_bytes!("fixtures/images/sample.gif").as_slice(),
        ),
        (
            "bmp",
            include_bytes!("fixtures/images/sample.bmp").as_slice(),
        ),
        (
            "tiff",
            include_bytes!("fixtures/images/sample.tiff").as_slice(),
        ),
    ] {
        let bytes = bytes(doc(vec![
            Paragraph::from_children(vec![
                ImageRun::new(data.to_vec()).into(),
                ImageRun::new(data.to_vec()).into(),
            ])
            .into(),
        ]));
        let archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        assert_eq!(
            archive
                .file_names()
                .filter(|n| n.starts_with("word/media/"))
                .count(),
            1,
            "{extension}"
        );
        let xml = part(&bytes, "word/document.xml");
        let parsed = roxmltree::Document::parse(&xml).unwrap();
        let ext = parsed
            .descendants()
            .find(|n| n.tag_name().name() == "extent")
            .unwrap();
        let cx = ext.attribute("cx").unwrap().parse::<i64>().unwrap();
        assert_eq!(
            cx,
            if extension == "gif" || extension == "png_zero_dpi" {
                304800
            } else {
                152400
            },
            "{extension}"
        );
    }
}
#[test]
fn image_scaling_alt_text_and_conditional_data() {
    let mut image = ImageRun::deferred(Value::future(async {
        ImageData::Bytes(include_bytes!("fixtures/images/sample.png").to_vec())
    }));
    image.transformation = Some(ImageTransformation {
        width: None,
        height: Some(36.),
        ..Default::default()
    })
    .into();
    image.alt_text = Some(AltText {
        title: Some("Title <&>".into()),
        description: Some("Description".into()),
        name: Some("Name".into()),
    })
    .into();
    let bytes = bytes(doc(vec![
        Paragraph::from_children(vec![
            image.into(),
            ImageRun::deferred(Value::lazy(|| panic!("hidden image")))
                .when(false)
                .into(),
        ])
        .into(),
    ]));
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    let ext = parsed
        .descendants()
        .find(|n| n.tag_name().name() == "extent")
        .unwrap();
    assert_eq!(ext.attribute("cx"), Some("914400"));
    assert_eq!(ext.attribute("cy"), Some("457200"));
    let pr = parsed
        .descendants()
        .find(|n| n.tag_name().name() == "docPr")
        .unwrap();
    assert_eq!(pr.attribute("title"), Some("Title <&>"));
    assert_eq!(pr.attribute("descr"), Some("Description"));
    for bad in [vec![], b"<svg/>".to_vec(), vec![0xff, 0xd8]] {
        assert!(
            block_on(
                doc(vec![
                    Paragraph::from_children(vec![ImageRun::new(bad).into()]).into()
                ])
                .resolve()
            )
            .is_err()
        );
    }
}
#[test]
fn sections_support_all_declared_properties_and_inherit_unspecified_values() {
    let mut section = Section::new(vec![Paragraph::new("one").into()]);
    section.properties = Some(SectionProperties {
        line_numbering: None,
        page_borders: None,
        document_grid: None,
        page_size: Some(PageSize {
            width: Some(7772400),
            height: Some(10058400),
        }),
        page_orientation: Some(Orientation::Landscape),
        page_margins: Some(PageMargins {
            top: Some(914400),
            ..PageMargins::default()
        }),
        page_numbering: Some(PageNumbering {
            start: Some(7),
            format: Some("lowerRoman".into()),
        }),
        columns: Some(Columns {
            count: Some(2),
            space: Some(360),
            separator: Some(true),
        }),
        vertical_align: Some(VerticalAlignment::Center),
        title_page: Some(true),
        r#type: Some(SectionType::OddPage),
        bidi: None,
        rtl_gutter: None,
    })
    .into();
    let bytes = bytes(Document::new(vec![
        section.into(),
        Section::new(vec![Paragraph::new("two").into()]).into(),
    ]));
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    let sections: Vec<_> = parsed
        .descendants()
        .filter(|n| n.has_tag_name((W, "sectPr")))
        .collect();
    assert_eq!(sections.len(), 2);
    for s in sections {
        let find = |name| s.children().find(|n| n.has_tag_name((W, name))).unwrap();
        assert_eq!(find("pgSz").attribute((W, "w")), Some("15840"));
        assert_eq!(find("pgSz").attribute((W, "h")), Some("12240"));
        assert_eq!(find("pgNumType").attribute((W, "start")), Some("7"));
        assert_eq!(find("cols").attribute((W, "num")), Some("2"));
        assert_eq!(find("cols").attribute((W, "sep")), Some("1"));
        assert_eq!(find("vAlign").attribute((W, "val")), Some("center"));
        assert_eq!(find("type").attribute((W, "val")), Some("oddPage"));
        find("titlePg");
    }
}
#[test]
fn tables_resolve_all_lazy_levels_and_skip_conditional_rows_and_cells() {
    let table = Table {
        rows: Value::future(async {
            vec![
                TableRow {
                    children: Value::lazy(|| {
                        vec![
                            TableCell {
                                children: Value::future(async {
                                    vec![Paragraph::new("cell").into()]
                                }),
                                ..TableCell::default()
                            }
                            .into(),
                            TableCell {
                                children: Value::lazy(|| panic!("hidden cell")),
                                ..TableCell::default()
                            }
                            .when(false)
                            .into(),
                        ]
                    }),
                    ..TableRow::default()
                }
                .into(),
                TableRow {
                    children: Value::lazy(|| panic!("hidden row")),
                    ..TableRow::default()
                }
                .when(false)
                .into(),
            ]
        }),
        ..Table::default()
    };
    let bytes = bytes(doc(vec![
        table.into(),
        Table {
            rows: Value::lazy(|| panic!("hidden table")),
            ..Table::default()
        }
        .when(false)
        .into(),
        Table::default().into(),
    ]));
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "tbl")))
            .count(),
        1
    );
    assert_eq!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name((W, "tc")))
            .count(),
        1
    );
}
#[test]
fn autofit_ignores_column_widths_and_fixed_widths_are_validated() {
    let make = |layout| Table {
        rows: vec![TableRow::new(vec![TableCell::default().into()]).into()].into(),
        properties: TableProperties {
            layout,
            column_widths: Some(vec![1000, 2000]),
            ..Default::default()
        }
        .into(),
        ..Table::default()
    };
    assert!(block_on(doc(vec![make(Some(TableLayout::Fixed)).into()]).resolve()).is_err());
    let bytes = bytes(doc(vec![make(Some(TableLayout::Autofit)).into()]));
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        parsed
            .descendants()
            .find(|n| n.has_tag_name((W, "tblLayout")))
            .unwrap()
            .attribute((W, "type")),
        Some("autofit")
    );
}
#[test]
fn every_table_style_region_is_written_and_duplicate_style_names_fail() {
    let f = Some(TableSectionFormat {
        background: Some([1, 2, 3]),
        bold: Some(true),
        ..TableSectionFormat::default()
    });
    let style = TableStyleDefinition {
        name: "All Regions".into(),
        first_row: f.clone(),
        last_row: f.clone(),
        first_column: f.clone(),
        last_column: f.clone(),
        banding_1_row: f.clone(),
        banding_2_row: f.clone(),
        banding_1_column: f.clone(),
        banding_2_column: f.clone(),
        top_left_cell: f.clone(),
        top_right_cell: f.clone(),
        bottom_left_cell: f.clone(),
        bottom_right_cell: f,
        ..TableStyleDefinition::default()
    };
    let mut document = doc(vec![]);
    document.styles = vec![style.clone().into()].into();
    let bytes = bytes(document);
    let xml = part(&bytes, "word/styles.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    let s = parsed
        .descendants()
        .find(|n| n.attribute((W, "styleId")) == Some("AllRegions"))
        .unwrap();
    assert_eq!(
        s.children()
            .filter(|n| n.has_tag_name((W, "tblStylePr")))
            .count(),
        12
    );
    let mut document = doc(vec![]);
    document.styles = vec![style.clone().into(), style.into()].into();
    assert!(block_on(document.resolve()).unwrap().to_docx().is_err());
}
#[test]
fn custom_numbering_is_linked_to_paragraphs_and_ids_cannot_collide() {
    let mut p = Paragraph::new("numbered");
    p.numbering = Some(NumberingReference { id: 42, level: 0 }).into();
    let mut document = doc(vec![p.into()]);
    document.numbering = vec![NumberingDefinition {
        abstract_id: None,
        inherit_style: None,
        id: 42,
        start_overrides: Vec::new(),
        level_overrides: Vec::new(),
        levels: vec![NumberingLevel {
            label_format: None,
            paragraph_style: None,
            paragraph_format: None,
            level: 0,
            start: 1,
            format: "decimal".into(),
            text: "%1.".into(),
            is_legal: None,
            suffix: None,
            restart: None,
            alignment: None,
        }],
    }]
    .into();
    let bytes = bytes(document);
    let xml = part(&bytes, "word/numbering.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert!(
        parsed
            .descendants()
            .any(|n| n.attribute((W, "numId")) == Some("42"))
    );
    let xml = part(&bytes, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        parsed
            .descendants()
            .find(|n| n.has_tag_name((W, "numId")))
            .unwrap()
            .attribute((W, "val")),
        Some("42")
    );
}
#[test]
fn invalid_formatting_and_undefined_styles_fail_without_a_package() {
    for paragraph in [
        Paragraph {
            format: ParagraphFormat {
                heading: Some(10),
                ..ParagraphFormat::default()
            }
            .into(),
            ..Paragraph::default()
        },
        Paragraph {
            format: ParagraphFormat {
                line_spacing: Some(f64::NAN),
                ..ParagraphFormat::default()
            }
            .into(),
            ..Paragraph::default()
        },
        Paragraph {
            format: ParagraphFormat {
                style: Some("Not a style".into()),
                ..ParagraphFormat::default()
            }
            .into(),
            ..Paragraph::default()
        },
    ] {
        assert!(
            block_on(doc(vec![paragraph.into()]).resolve())
                .and_then(|snapshot| snapshot.to_docx())
                .is_err()
        );
    }
}

#[test]
fn comment_ranges_cover_the_same_runs_as_the_python_reference() {
    fn ranges(xml: &str) -> std::collections::BTreeMap<String, (usize, usize)> {
        let doc = roxmltree::Document::parse(xml).unwrap();
        let mut count = 0;
        let mut ranges = std::collections::BTreeMap::new();
        for n in doc.descendants().filter(|n| n.is_element()) {
            if n.has_tag_name((W, "r")) {
                count += 1;
            } else if n.has_tag_name((W, "commentRangeStart")) {
                ranges.insert(n.attribute((W, "id")).unwrap().to_owned(), (count, 0));
            } else if n.has_tag_name((W, "commentRangeEnd")) {
                ranges.get_mut(n.attribute((W, "id")).unwrap()).unwrap().1 = count;
            }
        }
        ranges
    }
    let bytes = bytes(features::document());
    let expected = include_bytes!("fixtures/features.docx");
    assert_eq!(
        ranges(&part(&bytes, "word/document.xml")),
        ranges(&part(expected, "word/document.xml"))
    );
}
