//! Cross-check public template output using an independent XML parser.
mod support;
use futures::executor::block_on;
use rv_docx_rs::*;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(parts: BTreeMap<String, Vec<u8>>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rv-docx-preserve-{}-{}.docx",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in parts {
            archive
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(&data).unwrap();
        }
        std::fs::write(&path, archive.finish().unwrap().into_inner()).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn parts(data: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (entry.name().into(), data)
        })
        .collect()
}
fn resolve(document: Document) -> ResolvedDocument {
    block_on(document.resolve()).unwrap()
}
#[test]
fn relocated_main_headers_and_media_are_discovered_by_relationships() {
    let image = include_bytes!("fixtures/images/sample.png").to_vec();
    let mut section = Section::new(vec![Paragraph::new("before").into()]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![ImageRun::new(image.clone()).into()]).into(),
        ])
        .into(),
    );
    let original = resolve(Document::new(vec![section.into()]))
        .to_docx()
        .unwrap();
    let mut relocated = BTreeMap::new();
    for (name, data) in parts(&original) {
        let target = match name.as_str() {
            "word/document.xml" => "content/body.xml",
            "word/_rels/document.xml.rels" => "content/_rels/body.xml.rels",
            "word/header1.xml" => "stories/top.xml",
            "word/_rels/header1.xml.rels" => "stories/_rels/top.xml.rels",
            "word/media/image1.png" => "assets/picture.png",
            other => other,
        }
        .to_owned();
        let data = if name.ends_with(".rels") || name == "[Content_Types].xml" {
            let mut xml = String::from_utf8(data).unwrap();
            if name == "_rels/.rels" || name == "[Content_Types].xml" {
                xml = xml
                    .replace("word/document.xml", "content/body.xml")
                    .replace("word/header1.xml", "stories/top.xml")
                    .replace("word/media/image1.png", "assets/picture.png");
            } else if name == "word/_rels/document.xml.rels" {
                for file in [
                    "styles.xml",
                    "stylesWithEffects.xml",
                    "settings.xml",
                    "webSettings.xml",
                    "fontTable.xml",
                    "numbering.xml",
                    "theme/theme1.xml",
                ] {
                    xml = xml.replace(
                        &format!("Target=\"{file}\""),
                        &format!("Target=\"../word/{file}\""),
                    );
                }
                xml = xml.replace("Target=\"header1.xml\"", "Target=\"../stories/top.xml\"");
            } else if name == "word/_rels/header1.xml.rels" {
                xml = xml.replace(
                    "Target=\"media/image1.png\"",
                    "Target=\"../assets/picture.png\"",
                );
            }
            xml.into_bytes()
        } else {
            data
        };
        relocated.insert(target, data);
    }
    let fixture = Fixture::new(relocated);
    let hyperlink = |text: &str| {
        Hyperlink::new(
            HyperlinkTarget::external("https://example.invalid/relocated?q=1&x=2#frag").unwrap(),
            vec![TextRun::new(text).into()],
        )
    };
    let mut added = Section::new(vec![
        Paragraph::from_children(vec![hyperlink("after").into()]).into(),
    ]);
    added.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![
            Paragraph::from_children(vec![
                ImageRun::new(image.clone()).into(),
                hyperlink("header link").into(),
            ])
            .into(),
        ])
        .into(),
    );
    let output = resolve(Document::new(vec![added.into()]))
        .to_docx_with_template(&DocumentTemplate::new(&fixture.0))
        .unwrap();
    let content = support::part(&output, "content/body.xml");
    let parsed = roxmltree::Document::parse(&content).unwrap();
    let texts: Vec<_> = parsed
        .descendants()
        .filter(|n| n.has_tag_name((support::W, "t")))
        .map(|n| n.text().unwrap_or(""))
        .collect();
    assert_eq!(texts, ["before", "after"]);
    let header = support::part(&output, "stories/top.xml");
    assert_eq!(
        roxmltree::Document::parse(&header)
            .unwrap()
            .descendants()
            .filter(|n| n.tag_name().name() == "inline")
            .count(),
        2
    );
    assert_eq!(parts(&output)["assets/picture.png"], image);
    assert!(support::part(&output, "stories/_rels/top.xml.rels").contains("../"));
    assert!(!parts(&output).contains_key("word/document.xml"));
    for (owner, rels) in [
        ("content/body.xml", "content/_rels/body.xml.rels"),
        ("stories/top.xml", "stories/_rels/top.xml.rels"),
    ] {
        let xml = support::part(&output, owner);
        let tree = roxmltree::Document::parse(&xml).unwrap();
        let link = tree
            .descendants()
            .find(|n| n.has_tag_name((support::W, "hyperlink")))
            .unwrap();
        let id = link
            .attribute((
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                "id",
            ))
            .unwrap();
        let xml = support::part(&output, rels);
        let tree = roxmltree::Document::parse(&xml).unwrap();
        let rel = tree
            .descendants()
            .find(|n| n.attribute("Id") == Some(id))
            .unwrap();
        assert_eq!(rel.attribute("TargetMode"), Some("External"));
        assert_eq!(
            rel.attribute("Target"),
            Some("https://example.invalid/relocated?q=1&x=2#frag")
        );
    }
}
#[test]
fn shadowed_namespaces_and_opaque_properties_survive_template_rendering() {
    let mut package = parts(include_bytes!("../assets/default.docx"));
    let source=String::from_utf8(package["word/document.xml"].clone()).unwrap()
        .replace("mc:Ignorable=\"","xmlns:ext=\"urn:extension\" mc:Ignorable=\"ext ")
        .replace("<w:sectPr", &format!(r#"<ext:container xmlns:w="urn:shadow" xmlns:main="{}"><main:p><main:r><main:t>retain</main:t></main:r></main:p><w:p/></ext:container><w:p><w:pPr><w:jc w:val="left"/><ext:property/><w:pStyle w:val="Normal"/></w:pPr></w:p><w:sectPr"#,support::W));
    package.insert("word/document.xml".into(), source.into_bytes());
    let fixture = Fixture::new(package);
    let output = resolve(Document::default())
        .to_docx_with_template(&DocumentTemplate::new(&fixture.0))
        .unwrap();
    let xml = support::part(&output, "word/document.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    let opaque = parsed
        .descendants()
        .find(|n| n.has_tag_name(("urn:extension", "container")))
        .unwrap();
    assert_eq!(
        opaque
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().namespace())
            .collect::<Vec<_>>(),
        [Some(support::W), Some("urn:shadow")]
    );
    let props = parsed
        .descendants()
        .find(|n| n.has_tag_name((support::W, "pPr")))
        .unwrap();
    assert_eq!(
        props
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name())
            .collect::<Vec<_>>(),
        ["jc", "property", "pStyle"]
    );
}

#[test]
fn generated_metadata_qnames_do_not_rebind_opaque_template_prefixes() {
    let mut package = parts(include_bytes!("../assets/default.docx"));
    let core = String::from_utf8(package["docProps/core.xml"].clone())
        .unwrap()
        .replace("dcterms:", "actualDate:")
        .replace("xmlns:dcterms=", "xmlns:actualDate=")
        .replace(
            "<cp:coreProperties",
            "<cp:coreProperties xmlns:dcterms=\"urn:opaque\"",
        );
    package.insert("docProps/core.xml".into(), core.into_bytes());
    let fixture = Fixture::new(package);
    let output = resolve(Document::default())
        .to_docx_with_template(&DocumentTemplate::new(&fixture.0))
        .unwrap();
    let xml = support::part(&output, "docProps/core.xml");
    let parsed = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        parsed.root_element().lookup_namespace_uri(Some("dcterms")),
        Some("urn:opaque")
    );
    let created = parsed
        .descendants()
        .find(|n| n.has_tag_name(("http://purl.org/dc/terms/", "created")))
        .unwrap();
    let value = created
        .attribute(("http://www.w3.org/2001/XMLSchema-instance", "type"))
        .unwrap();
    let (prefix, local) = value.split_once(':').unwrap();
    assert_eq!(local, "W3CDTF");
    assert_eq!(
        created.lookup_namespace_uri(Some(prefix)),
        Some("http://purl.org/dc/terms/")
    );
}
