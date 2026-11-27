//! Direct native byte oracle for the independent MCP story fixture.
use rocx::{
    Component, HeaderFooterType, Hyperlink, HyperlinkTarget, Paragraph, ParagraphFormat,
    ResolvedBlock, RunFormat, TextRun,
    editor::{
        BodyBlock, EditBatch, HeaderEditMode, ImportedDocument, ImportedInlineNode, ImportedStory,
        ParagraphInsertion, RunTextEdit, SectionStoryKind, StoryReferenceEdit,
    },
};
use std::{env, fs};

fn story_text(doc: &ImportedDocument, source: &ImportedStory) -> Vec<u8> {
    let ImportedInlineNode::Run(direct) = &source.inline_structure(0).unwrap()[0] else {
        panic!("expected a direct run at the start of the story paragraph")
    };
    let edits = [RunTextEdit {
        target: direct.anchor.clone(),
        text: " Direct 😀 ".into(),
    }];
    doc.edit_story_text(&source.part_name, &edits).unwrap()
}

fn first_footer(doc: &ImportedDocument) -> ImportedStory {
    doc.stories()
        .unwrap()
        .into_iter()
        .find(|story| {
            story.section_index == 1
                && story.kind == SectionStoryKind::Footer
                && story.variant == HeaderFooterType::First
                && story.inherited
        })
        .unwrap()
}

fn rich_paragraph() -> rocx::ResolvedParagraph {
    futures_executor::block_on(
        Paragraph::from_children(vec![
            TextRun::new("Styled 😀")
                .with_format(RunFormat {
                    bold: Some(true),
                    italic: Some(false),
                    size: Some(14.0),
                    ..RunFormat::default()
                })
                .into(),
            TextRun::new(" tail").into(),
            Hyperlink::new(
                HyperlinkTarget::external("https://example.org/rich").unwrap(),
                vec![TextRun::new(" source").into()],
            )
            .into(),
        ])
        .with_format(ParagraphFormat {
            spacing_after: Some(9.0),
            ..ParagraphFormat::default()
        })
        .resolve(),
    )
    .unwrap()
    .unwrap()
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
        "text" => {
            let story = doc
                .stories()
                .unwrap()
                .into_iter()
                .find(|story| {
                    story.section_index == 0 && story.kind == Header && story.variant == Default
                })
                .unwrap();
            story_text(&doc, &story)
        }
        "first-footer-text" => story_text(&doc, &first_footer(&doc)),
        "first-footer-blocks" => {
            let target = first_footer(&doc).anchor.unwrap();
            let blocks = [ResolvedBlock::Paragraph(
                futures_executor::block_on(Paragraph::new("New footer 😀").resolve())
                    .unwrap()
                    .unwrap(),
            )];
            let error = doc
                .edit_story_blocks(&target, HeaderEditMode::SectionLocal, 1, 0, &blocks)
                .unwrap_err();
            assert!(error.to_string().contains("ambiguous section structure"));
            doc.edit_story_blocks(&target, HeaderEditMode::Shared, 1, 0, &blocks)
                .unwrap()
        }
        "rich-insert" => {
            let BodyBlock::Paragraph(paragraph) = &doc.body()[0] else {
                panic!("fixture must start with a paragraph");
            };
            doc.preview_edits(EditBatch {
                paragraphs: &[ParagraphInsertion {
                    target: paragraph.anchor.clone(),
                    paragraph: rich_paragraph(),
                }],
                ..EditBatch::default()
            })
            .unwrap()
            .to_docx()
            .unwrap()
        }
        "create-rich" => doc
            .edit_story_reference(
                0,
                Footer,
                Even,
                StoryReferenceEdit::Create(vec![ResolvedBlock::Paragraph(rich_paragraph())]),
            )
            .unwrap(),
        "rich-splice-shared" | "rich-splice-local" => {
            let local = case == "rich-splice-local";
            let story = doc
                .stories()
                .unwrap()
                .into_iter()
                .find(|story| {
                    story.kind == if local { Footer } else { Header }
                        && story.variant == Default
                        && story.section_index == 1
                })
                .unwrap();
            doc.edit_story_blocks(
                &story.anchor.unwrap(),
                if local {
                    HeaderEditMode::SectionLocal
                } else {
                    HeaderEditMode::Shared
                },
                if local { 1 } else { 2 },
                0,
                &[ResolvedBlock::Paragraph(rich_paragraph())],
            )
            .unwrap()
        }
        "rich-splice-footer" => {
            let story = doc
                .stories()
                .unwrap()
                .into_iter()
                .find(|story| {
                    story.kind == Footer && story.variant == Default && story.section_index == 1
                })
                .unwrap();
            doc.edit_story_blocks(
                &story.anchor.unwrap(),
                HeaderEditMode::Shared,
                0,
                1,
                &[ResolvedBlock::Paragraph(rich_paragraph())],
            )
            .unwrap()
        }
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
        "link-default-only" => {
            let part = doc.default_headers().unwrap()[0].part_name.clone();
            doc.edit_story_reference(1, Header, Even, StoryReferenceEdit::Link(part))
                .unwrap()
        }
        "link-first-only" => {
            use HeaderFooterType::First;

            let part = "word/header1.xml";
            let guarded = doc
                .edit_story_reference(0, Header, First, StoryReferenceEdit::Link(part.into()))
                .unwrap_err();
            assert!(
                guarded
                    .to_string()
                    .contains("cannot change a revised section or opaque story reference")
            );
            let late = doc
                .edit_story_reference(
                    1,
                    Header,
                    Even,
                    StoryReferenceEdit::Link("word/custom/late-header.xml".into()),
                )
                .unwrap_err();
            assert!(
                late.to_string()
                    .contains("cannot have an external relationship")
            );
            doc.edit_story_reference(1, Header, Even, StoryReferenceEdit::Link(part.into()))
                .unwrap()
        }
        "relink" => doc
            .edit_story_reference(
                1,
                Footer,
                Default,
                StoryReferenceEdit::Link("word/footer2.xml".into()),
            )
            .unwrap(),
        _ => panic!("unknown story oracle case: {case:?}"),
    };
    fs::write(output, bytes).unwrap();
}
