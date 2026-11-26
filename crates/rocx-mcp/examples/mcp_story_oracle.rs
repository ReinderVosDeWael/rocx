//! Direct native byte oracle for the independent MCP story fixture.
use rocx::{
    Component, HeaderFooterType, Paragraph, ResolvedBlock,
    editor::{
        ImportedDocument, ImportedInlineNode, RunTextEdit, SectionStoryKind, StoryReferenceEdit,
    },
};
use std::{env, fs};

fn story_text(doc: &ImportedDocument) -> Vec<u8> {
    let stories = doc.stories().unwrap();
    let source = stories
        .iter()
        .find(|story| {
            story.section_index == 0
                && story.kind == SectionStoryKind::Header
                && story.variant == HeaderFooterType::Default
        })
        .unwrap();
    let ImportedInlineNode::Run(direct) = &source.inline_structure(0).unwrap()[0] else {
        panic!("expected a direct run at the start of the header paragraph")
    };
    let edits = [RunTextEdit {
        target: direct.anchor.clone(),
        text: " Direct 😀 ".into(),
    }];
    doc.edit_story_text(&source.part_name, &edits).unwrap()
}

fn main() {
    let mut args = env::args_os().skip(1);
    let case = args.next().unwrap();
    let output = args.next().unwrap();
    let source = args.next().unwrap();
    assert!(args.next().is_none());
    let doc = ImportedDocument::open(&fs::read(source).unwrap()).unwrap();
    use HeaderFooterType::{Default, Even};
    use SectionStoryKind::{Footer, Header};
    let bytes = match case.to_str().unwrap() {
        "text" => story_text(&doc),
        "create" => {
            let blocks = ["New even footer", "Second paragraph & 😀"]
                .into_iter()
                .map(|text| {
                    ResolvedBlock::Paragraph(
                        futures_executor::block_on(Paragraph::new(text).resolve())
                            .unwrap()
                            .unwrap(),
                    )
                })
                .collect();
            doc.edit_story_reference(0, Footer, Even, StoryReferenceEdit::Create(blocks))
                .unwrap()
        }
        "link" => {
            let part = doc
                .stories()
                .unwrap()
                .into_iter()
                .find(|story| {
                    story.section_index == 0 && story.kind == Header && story.variant == Default
                })
                .unwrap()
                .part_name;
            doc.edit_story_reference(1, Header, Even, StoryReferenceEdit::Link(part))
                .unwrap()
        }
        "remove" => doc
            .edit_story_reference(1, Footer, Default, StoryReferenceEdit::Remove)
            .unwrap(),
        _ => panic!("unknown story oracle case: {case:?}"),
    };
    fs::write(output, bytes).unwrap();
}
