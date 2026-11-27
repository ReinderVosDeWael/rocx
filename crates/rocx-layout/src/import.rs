use crate::{FlowAlign, FlowStyle, PageSpec};
use rocx::editor::{
    BodyBlock, ImportedDocument, ImportedInline, ImportedInlineNode, ImportedParagraph,
    ImportedSection, LayoutField, LayoutValue,
    layout_bridge::{verify_direct_flow, verify_plain_flow},
};
use rocx::{BreakType, RunFormat};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    Plain,
    Direct,
}

type SectionFlow = (Vec<String>, PageSpec, Vec<FlowStyle>);

/// Project a plain, single-section, single-column DOCX body into prototype
/// layout input. The returned paragraph indices are also indices into
/// [`ImportedDocument::body`]; no blocks are skipped.
///
/// Requires explicit page width, height, and all four body margins. Twips
/// round to the nearest 1/64 point. The layout session supplies the only font
/// and size and uses its own line metrics. This profile does not carry
/// direct paragraph spacing or indents.
/// Only ordinary unformatted runs, empty paragraphs, and inline line breaks
/// are supported. A line break occupies one scalar (`\n`) in the projection.
/// Boundary spaces in a text element require `xml:space="preserve"`.
/// Drawing a caret does not guarantee eligibility for a `rocx` text edit.
///
/// This is not a Word layout or conformance check. The main body's direct
/// markup is checked even where the typed views omit it; for example,
/// individual column definitions, alternate-unit indents and spacing, and
/// nontrivial line-break clearing are refused. An absent column count means
/// one column. Styles (including Normal), package settings (such as mirrored
/// margins and compatibility mode), and other package-level formatting are
/// not resolved. Callers must supply known left-to-right input whose effective
/// flow does not depend on those features. The layout session separately
/// checks character and font coverage.
pub fn plain_section(doc: &ImportedDocument) -> Result<(Vec<String>, PageSpec), String> {
    let (text, page, _) = project_section(doc, Profile::Plain)?;
    Ok((text, page))
}

/// The bound DOCX profile adds direct `left`, `center`, and `right` alignment
/// and direct `before`/`after` spacing in nonnegative twips. It also accepts
/// nonnegative `left`/`right` indents and either `firstLine` or `hanging`.
/// Only the first row uses the `firstLine` or `hanging` adjustment, even if
/// the paragraph spans pages.
/// Hanging subtracts from the left indent and cannot cross the page content's
/// left edge. Missing values mean left alignment and zero spacing/indents;
/// no inheritance or defaults are read.
/// Each wrapped row is aligned separately, and between paragraphs on the same
/// page the larger spacing wins. Other paragraph properties retain the
/// [`plain_section`] restrictions.
pub(crate) fn section_flow(doc: &ImportedDocument) -> Result<SectionFlow, String> {
    project_section(doc, Profile::Direct)
}

fn project_section(doc: &ImportedDocument, profile: Profile) -> Result<SectionFlow, String> {
    let sections = doc.sections().map_err(|e| format!("section layout: {e}"))?;
    let [section] = sections.as_slice() else {
        return Err("expected exactly one section".into());
    };
    let page = page_spec(section)?;
    if !doc
        .stories()
        .map_err(|e| format!("section 0: {e}"))?
        .is_empty()
    {
        return Err("section 0: headers and footers are unsupported".into());
    }
    let body = doc.body();
    let mut paragraphs = Vec::with_capacity(body.len());
    let mut styles = match profile {
        Profile::Plain => Vec::new(),
        Profile::Direct => Vec::with_capacity(body.len()),
    };
    for (index, block) in body.iter().enumerate() {
        let paragraph = match block {
            BodyBlock::Paragraph(p) => p,
            BodyBlock::Table(_) => return Err(format!("body block {index}: unsupported table")),
            BodyBlock::ContentControl(_) => {
                return Err(format!("body block {index}: unsupported content control"));
            }
            BodyBlock::Opaque(name) => {
                return Err(format!("body block {index}: unsupported element {name}"));
            }
            _ => return Err(format!("body block {index}: unsupported block")),
        };
        let contextualize = |e| format!("body block {index}: {e}");
        let style = paragraph_style(paragraph, profile).map_err(contextualize)?;
        paragraphs.push(paragraph_text(doc, paragraph).map_err(contextualize)?);
        if profile == Profile::Direct {
            styles.push(style);
        }
    }
    match profile {
        Profile::Plain => verify_plain_flow(doc),
        Profile::Direct => verify_direct_flow(doc),
    }
    .map_err(|e| e.to_string())?;
    Ok((paragraphs, page, styles))
}

fn layout_measure(twips: i32, name: &str) -> Result<i32, String> {
    if twips < 0 {
        return Err(format!("negative {name}"));
    }
    signed_layout_measure(twips, name)
}

fn signed_layout_measure(twips: i32, name: &str) -> Result<i32, String> {
    // 64/20 = 16/5; round to nearest symmetrically for hanging indents.
    let scaled = i64::from(twips) * 16;
    i32::try_from((scaled + scaled.signum() * 2) / 5)
        .map_err(|_| format!("{name} exceeds layout range"))
}

fn page_spec(section: &ImportedSection) -> Result<PageSpec, String> {
    use LayoutField::*;
    use LayoutValue::{Boolean, Integer, Token};
    if section.section_type_xml.is_some()
        && section.section_type != Some(rocx::SectionType::NextPage)
    {
        return Err("section 0: unsupported section type".into());
    }
    for (&field, value) in &section.layout {
        let supported = match (field, value) {
            (PageWidth | PageHeight | MarginTop | MarginBottom | MarginLeft | MarginRight, _) => {
                true
            }
            // No active grid mode is the default; some producers still record a pitch.
            (
                Orientation | Margins | MarginHeader | MarginFooter | ColumnSpace | GridPitch
                | SectionType,
                _,
            ) => true,
            (ColumnCount, Integer(1)) | (MarginGutter | GridCharSpace, Integer(0)) => true,
            (ColumnSeparator | TitlePage | Bidi | RtlGutter, Boolean(false)) => true,
            (VerticalAlign, Token(token)) => token == "top",
            (GridMode, Token(token)) => token == "default",
            _ => false,
        };
        if !supported {
            return Err(format!("section 0: unsupported {}", field.name()));
        }
    }

    let measure = |field: LayoutField| {
        let Some(Integer(twips)) = section.layout.get(&field) else {
            return Err(format!("section 0: expected explicit {}", field.name()));
        };
        layout_measure(*twips, &field.name()).map_err(|e| format!("section 0: {e}"))
    };
    let page = PageSpec {
        width: measure(PageWidth)?,
        height: measure(PageHeight)?,
        left: measure(MarginLeft)?,
        right: measure(MarginRight)?,
        top: measure(MarginTop)?,
        bottom: measure(MarginBottom)?,
        space_before: 0,
        space_after: 0,
    };
    if i64::from(page.left) + i64::from(page.right) >= i64::from(page.width)
        || i64::from(page.top) + i64::from(page.bottom) >= i64::from(page.height)
    {
        return Err("section 0: margins leave no body area".into());
    }
    Ok(page)
}

fn paragraph_style(paragraph: &ImportedParagraph, profile: Profile) -> Result<FlowStyle, String> {
    use LayoutField::*;
    use LayoutValue::{Boolean, Integer, Token};
    if paragraph.mark_revision.is_some() {
        return Err("unsupported paragraph mark revision".into());
    }
    let align = match (profile, paragraph.alignment_xml.as_deref()) {
        (_, None | Some("left")) => FlowAlign::Left,
        (Profile::Direct, Some("center")) => FlowAlign::Center,
        (Profile::Direct, Some("right")) => FlowAlign::Right,
        (Profile::Plain, _) => return Err("unsupported paragraph alignment (expected left)".into()),
        (Profile::Direct, _) => {
            return Err("unsupported paragraph alignment (expected left, center, or right)".into());
        }
    };
    let mut style = FlowStyle {
        align,
        ..FlowStyle::default()
    };
    for (&field, value) in &paragraph.layout {
        match (field, value) {
            (Alignment, Token(token)) if paragraph.alignment_xml.as_deref() == Some(token) => (),
            (SpaceBefore, Integer(twips)) if profile == Profile::Direct => {
                style.before = layout_measure(*twips, "paragraph spaceBefore")?;
            }
            (SpaceAfter, Integer(twips)) if profile == Profile::Direct => {
                style.after = layout_measure(*twips, "paragraph spaceAfter")?;
            }
            (LeftIndent, Integer(twips)) if profile == Profile::Direct => {
                style.left = layout_measure(*twips, "paragraph leftIndent")?;
            }
            (RightIndent, Integer(twips)) if profile == Profile::Direct => {
                style.right = layout_measure(*twips, "paragraph rightIndent")?;
            }
            (FirstLineIndent, Integer(twips)) if profile == Profile::Direct => {
                style.first_line = signed_layout_measure(*twips, "paragraph firstLineIndent")?;
            }
            (
                KeepWithNext | KeepTogether | PageBreakBefore | WidowControl | SuppressLineNumbers
                | ParagraphBidi | SnapToGrid | ContextualSpacing | MirrorIndents,
                Boolean(false),
            ) => (),
            _ => return Err(format!("unsupported paragraph {}", field.name())),
        }
    }
    Ok(style)
}

fn paragraph_text(doc: &ImportedDocument, paragraph: &ImportedParagraph) -> Result<String, String> {
    let mut text = String::new();
    for (index, node) in paragraph.inline_structure.iter().enumerate() {
        let ImportedInlineNode::Run(run) = node else {
            let kind = match node {
                ImportedInlineNode::Hyperlink(_) => "hyperlink",
                ImportedInlineNode::Revision(_) => "revision",
                ImportedInlineNode::Fraction { .. } => "fraction",
                ImportedInlineNode::Opaque(name) => name,
                _ => "inline node",
            };
            return Err(format!("inline {index}: unsupported {kind}"));
        };
        if run.has_opaque_formatting
            || run.property_revision.is_some()
            || run.hidden.is_some()
            || run.format != RunFormat::default()
        {
            return Err(format!("run {index}: unsupported run formatting"));
        }
        for item in &run.items {
            match item {
                ImportedInline::Text(s) if !s.chars().any(char::is_control) => text.push_str(s),
                ImportedInline::Break(BreakType::Line | BreakType::TextWrapping) => text.push('\n'),
                _ => return Err(format!("run {index}: unsupported run content")),
            }
        }
    }
    if doc
        .paragraph_numbering(&paragraph.anchor)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("unsupported paragraph numbering".into());
    }
    Ok(text)
}
