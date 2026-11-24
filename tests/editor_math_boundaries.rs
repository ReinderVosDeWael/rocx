use rocx::editor::{
    BodyBlock, EditBatch, ImportedDocument, ImportedInlineNode, TextRangeEdit, TrackedText,
};
use std::io::{Cursor, Write};

fn fixture() -> ImportedDocument {
    // Extend the python-docx fixture with the audit's math-to-run bookmark,
    // spanning a second, otherwise entirely plain paragraph.
    let base = include_bytes!("fixtures/imported-editor.docx");
    let mut source = zip::ZipArchive::new(Cursor::new(base)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..source.len() {
        let entry = source.by_index(i).unwrap();
        if entry.name().unwrap() == "word/document.xml" {
            out.start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            out.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:eq="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body>
<w:p><eq:oMath><eq:r><eq:t>x</eq:t></eq:r></eq:oMath></w:p>
<w:p><w:r><w:t>before</w:t></w:r></w:p>
<w:p><eq:oMath><w:bookmarkStart w:name="Protected" w:id="31"/><eq:r><eq:t>x</eq:t></eq:r></eq:oMath><w:r><w:t>protected</w:t></w:r></w:p>
<w:p><w:r><w:t>also protected</w:t></w:r></w:p>
<w:p><w:bookmarkEnd w:id="31"/><w:r><w:t>outside</w:t></w:r></w:p>
<w:p><w:r><w:t>after</w:t></w:r></w:p>
</w:body></w:document>"#).unwrap();
        } else {
            out.raw_copy_file(entry).unwrap();
        }
    }
    ImportedDocument::open(&out.finish().unwrap().into_inner()).unwrap()
}

#[test]
fn tracked_run_uses_bookmark_state_at_its_inline_anchor() {
    let doc = fixture();
    let track = |index: usize| {
        let BodyBlock::Paragraph(p) = &doc.body()[index] else {
            panic!()
        };
        let anchor = p
            .inline_structure
            .iter()
            .find_map(|inline| match inline {
                ImportedInlineNode::Run(run) => Some(&run.anchor),
                _ => None,
            })
            .unwrap();
        doc.preview_tracked_text(anchor, TrackedText::Delete, "Reviewer", None)
    };
    track(1).unwrap();
    assert!(track(2).is_err());
    track(4).unwrap();
}

#[test]
fn text_range_uses_bookmark_state_across_paragraphs() {
    let doc = fixture();
    let edit = |index: usize| {
        let BodyBlock::Paragraph(p) = &doc.body()[index] else {
            panic!()
        };
        doc.preview_edits(EditBatch {
            ranges: &[TextRangeEdit {
                target: p.anchor.clone(),
                start: 0,
                end: 1,
                text: "Z".into(),
            }],
            ..Default::default()
        })
    };
    edit(1).unwrap();
    assert!(edit(3).is_err());
    assert_eq!(edit(5).unwrap().run_text_changes()[0].after, "Zfter");
}
