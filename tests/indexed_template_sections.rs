mod support;

use futures::executor::block_on;
use rocx::*;
use support::{W, part};

fn section(text: Option<&str>, left: i64) -> Section {
    let mut section = Section::new(text.into_iter().map(|s| Paragraph::new(s).into()).collect());
    section.properties = Some(SectionProperties {
        page_margins: Some(PageMargins {
            left: Some(left),
            ..Default::default()
        }),
        ..Default::default()
    })
    .into();
    section
}

#[test]
fn indexed_boundaries_follow_blocks_and_keep_section_geometry() {
    let path =
        std::env::temp_dir().join(format!("rocx-indexed-sections-{}.docx", std::process::id()));
    let template = block_on(
        Document::new(vec![
            Section::new(vec![
                Paragraph::new("Before").into(),
                Paragraph::new("Original").into(),
            ])
            .into(),
        ])
        .resolve(),
    )
    .unwrap();
    template.save(&path).unwrap();
    for (index, first, expected) in [
        (
            0,
            Some("A"),
            vec![vec!["A"], vec!["B", "Before", "Original"]],
        ),
        (
            1,
            Some("A"),
            vec![vec!["Before", "A"], vec!["B", "Original"]],
        ),
        (
            99,
            Some("A"),
            vec![vec!["Before", "Original", "A"], vec!["B"]],
        ),
        (1, None, vec![vec!["Before"], vec!["B", "Original"]]),
    ] {
        let doc = block_on(
            Document::new(vec![
                section(first, 914400).into(),
                section(Some("B"), 1828800).into(),
            ])
            .resolve(),
        )
        .unwrap();
        let mut template = DocumentTemplate::new(&path);
        template.edits.paragraph_index = Some(index);
        let bytes = doc.to_docx_with_template(&template).unwrap();
        let xml = part(&bytes, "word/document.xml");
        let parsed = roxmltree::Document::parse(&xml).unwrap();
        let body = parsed
            .descendants()
            .find(|n| n.has_tag_name((W, "body")))
            .unwrap();
        let mut groups = vec![Vec::new()];
        let mut margins = Vec::new();
        for node in body.children().filter(|n| n.is_element()) {
            if node.has_tag_name((W, "p")) {
                let text: String = node
                    .descendants()
                    .filter(|n| n.has_tag_name((W, "t")))
                    .filter_map(|n| n.text())
                    .collect();
                if !text.is_empty() {
                    groups.last_mut().unwrap().push(text);
                }
            }
            let props = if node.has_tag_name((W, "sectPr")) {
                Some(node)
            } else {
                node.children()
                    .find(|n| n.has_tag_name((W, "pPr")))
                    .and_then(|p| p.children().find(|n| n.has_tag_name((W, "sectPr"))))
            };
            if let Some(props) = props {
                let margin = props
                    .children()
                    .find(|n| n.has_tag_name((W, "pgMar")))
                    .unwrap();
                margins.push(margin.attribute((W, "left")).unwrap().to_string());
                if !node.has_tag_name((W, "sectPr")) {
                    groups.push(Vec::new());
                }
            }
        }
        assert_eq!(groups, expected, "index {index}, first {first:?}");
        assert_eq!(margins, ["1440", "2880"]);
    }
    std::fs::remove_file(path).unwrap();
}
