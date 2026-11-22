//! Builder assignments must equal public field assignments, including all other fields.
use futures::executor::block_on;
use rocx::*;

macro_rules! check {
    ($test:ident, $owner:ident, $base:expr; $($method:ident => $field:ident: $kind:ident($value:expr)),* $(,)?) => {
        #[test]
        fn $test() {
            $(let expected = $owner { $field: check!(@expected $kind, $value), ..$base };
            assert_eq!(($base).$method($value), expected, stringify!($method));)*
            let expected = $owner { $($field: check!(@expected $kind, $value)),* };
            assert_eq!(($base)$(.$method($value))*, expected);
        }
    };
    (@expected optional, $value:expr) => { Some($value.into()) };
    (@expected value, $value:expr) => { $value.into() };
}

check!(content_control_list_item, ContentControlListItem, ContentControlListItem { display_text: String::new(), value: String::new() };
    with_display_text => display_text: value("Accepted"),
    with_value => value: value("accepted"),
);
check!(content_control_properties, ContentControlProperties, ContentControlProperties::default();
    with_id => id: optional(42u32),
    with_text => text: value(ContentControlText::Plain { multi_line: Some(true) }),
    with_alias => alias: optional("Title"),
    with_tag => tag: optional("review"),
    with_lock => lock: optional(ContentControlLock::ControlAndContentLocked),
);
check!(text_revision, TextRevision, TextRevision { kind: RevisionKind::Insertion, author: "Initial".into(), date: None };
    with_kind => kind: value(RevisionKind::Deletion), with_author => author: value("Review"),
    with_date => date: optional("2026-11-20T00:00:00Z"),
);
check!(note_numbering, NoteNumbering, NoteNumbering::default();
    with_format => format: optional(NoteNumberFormat::UpperRoman),
    with_start => start: optional(3u32),
    with_restart => restart: optional(NoteNumberRestart::EachSection),
);
check!(defaults, DocumentDefaults, DocumentDefaults::default();
    with_mirror_margins => mirror_margins: optional(true),
    with_gutter_at_top => gutter_at_top: optional(false),
    with_track_revisions => track_revisions: optional(false),
    with_footnote_position => footnote_position: optional(FootnotePosition::BeneathText),
    with_endnote_position => endnote_position: optional(EndnotePosition::SectionEnd),
    with_footnote_numbering => footnote_numbering: optional(NoteNumbering::default().with_start(3)),
    with_endnote_numbering => endnote_numbering: optional(NoteNumbering::default().with_restart(NoteNumberRestart::Continuous)),
    with_run => run: optional(RunFormat::default().with_bold(false)),
    with_paragraph => paragraph: optional(ParagraphFormat::default().with_bidi(true)),
);
check!(language, RunLanguage, RunLanguage::default();
    with_latin => latin: optional("en-US"),
    with_east_asia => east_asia: optional("ja-JP"),
    with_bidi => bidi: optional("ar-SA"),
);
check!(tabs, TabStop, TabStop::default();
    with_position => position: value(-12.),
    with_alignment => alignment: value(TabAlignment::End),
    with_leader => leader: optional(TabLeader::Dot),
);
check!(paragraph, ParagraphFormat, ParagraphFormat::default();
    with_bidi => bidi: optional(true),
    with_heading => heading: optional(2u8),
    with_style => style: optional("Normal"),
    with_alignment => alignment: optional(Alignment::Right),
    with_spacing_before => spacing_before: optional(6.),
    with_spacing_after => spacing_after: optional(8.),
    with_line_spacing => line_spacing: optional(1.5),
    with_line_height => line_height: optional(LineHeight::Exact(14.)),
    with_left_indent => left_indent: optional(12.),
    with_right_indent => right_indent: optional(18.),
    with_first_line_indent => first_line_indent: optional(-6.),
    with_keep_together => keep_together: optional(false),
    with_keep_with_next => keep_with_next: optional(true),
    with_page_break_before => page_break_before: optional(false),
    with_widow_control => widow_control: optional(true),
    with_tab_stops => tab_stops: value(vec![TabStop::default().with_position(36.)]),
    with_suppress_line_numbers => suppress_line_numbers: optional(false),
    with_snap_to_grid => snap_to_grid: optional(false),
    with_contextual_spacing => contextual_spacing: optional(true), with_mirror_indents => mirror_indents: optional(false),
);
check!(page_size, PageSize, PageSize::default();
    with_width => width: optional(12000i64), with_height => height: optional(16000i64),
);
check!(page_margins, PageMargins, PageMargins::default();
    with_top => top: optional(720i64), with_bottom => bottom: optional(800i64),
    with_left => left: optional(900i64), with_right => right: optional(1000i64),
    with_header => header: optional(240i64), with_footer => footer: optional(360i64),
);
check!(line_numbering, LineNumbering, LineNumbering::default();
    with_clear => clear: value(true),
    with_count_by => count_by: optional(5u32), with_start => start: optional(0u32),
    with_distance => distance: optional(240u32), with_restart => restart: optional(LineNumberRestart::NewSection),
);
check!(page_numbering, PageNumbering, PageNumbering::default();
    with_start => start: optional(4u32), with_format => format: optional("decimal"),
);
check!(columns, Columns, Columns::default();
    with_count => count: optional(2u32), with_space => space: optional(720u32),
    with_separator => separator: optional(false),
);
check!(page_border, PageBorder, PageBorder { style: BorderLine::Single, size: None, space: None, color: None };
    with_style => style: value(BorderLine::Dashed), with_size => size: optional(8u32),
    with_space => space: optional(4u32), with_color => color: optional([32, 64, 96]),
);
check!(page_borders, PageBorders, PageBorders::default();
    with_offset_from => offset_from: optional(PageBorderOffset::Page),
    with_display => display: optional(PageBorderDisplay::FirstPage),
    with_z_order => z_order: optional(PageBorderZOrder::Back),
    with_top => top: optional(PageBorder { style: BorderLine::Single, size: None, space: None, color: None }),
    with_left => left: optional(PageBorder { style: BorderLine::Single, size: None, space: None, color: None }),
    with_bottom => bottom: optional(PageBorder { style: BorderLine::Single, size: None, space: None, color: None }),
    with_right => right: optional(PageBorder { style: BorderLine::Single, size: None, space: None, color: None }),
);
check!(document_grid, DocumentGrid, DocumentGrid::default();
    with_mode => mode: optional(DocumentGridMode::LinesAndChars),
    with_line_pitch => line_pitch: optional(360u32),
    with_character_pitch_delta => character_pitch_delta: optional(-4096i32),
);
check!(section_properties, SectionProperties, SectionProperties::default();
    with_page_size => page_size: optional(PageSize::default().with_width(12000)),
    with_page_orientation => page_orientation: optional(Orientation::Landscape),
    with_page_margins => page_margins: optional(PageMargins::default().with_left(720)),
    with_page_borders => page_borders: optional(PageBorders::default()),
    with_document_grid => document_grid: optional(DocumentGrid::default()),
    with_page_numbering => page_numbering: optional(PageNumbering::default().with_start(3)),
    with_line_numbering => line_numbering: optional(LineNumbering::default().with_count_by(5)),
    with_columns => columns: optional(Columns::default().with_count(2)),
    with_vertical_align => vertical_align: optional(VerticalAlignment::Center),
    with_title_page => title_page: optional(false),
    with_type => r#type: optional(SectionType::Continuous),
    with_bidi => bidi: optional(true), with_rtl_gutter => rtl_gutter: optional(false),
);
check!(table_border, TableBorder, TableBorder::default();
    with_side => side: value(BorderSide::Bottom), with_sz => sz: value(8u32),
    with_color => color: value([20u8, 40, 60]), with_val => val: value(BorderLine::Dashed),
);
check!(cell_border, CellBorder, CellBorder::default();
    with_side => side: value(CellBorderSide::End), with_sz => sz: value(12u32),
    with_color => color: value([60u8, 40, 20]), with_val => val: value(BorderLine::Dashed),
);
check!(paragraph_style, ParagraphStyleDefinition, ParagraphStyleDefinition::default();
    with_name => name: value("Builder style"), with_bidi => bidi: optional(true),
    with_base_style => base_style: optional("Normal"),
    with_next_paragraph_style => next_paragraph_style: optional("Normal"),
    with_bold => bold: optional(true), with_italic => italic: optional(false),
    with_underline => underline: optional(false), with_font => font: optional("Arial"),
    with_font_size => font_size: optional(12.), with_color => color: optional([20u8, 40, 60]),
    with_alignment => alignment: optional(Alignment::Center),
    with_spacing_before => spacing_before: optional(6.), with_spacing_after => spacing_after: optional(8.),
    with_line_spacing => line_spacing: optional(1.5),
    with_line_height => line_height: optional(LineHeight::AtLeast(14.)),
    with_left_indent => left_indent: optional(12.), with_right_indent => right_indent: optional(18.),
    with_first_line_indent => first_line_indent: optional(-6.),
    with_keep_together => keep_together: optional(true), with_keep_with_next => keep_with_next: optional(false),
    with_page_break_before => page_break_before: optional(true), with_widow_control => widow_control: optional(false),
    with_tab_stops => tab_stops: value(vec![TabStop::default().with_position(36.)]),
    with_language => language: optional(RunLanguage::default().with_latin("en-US")),
    with_bold_complex_script => bold_complex_script: optional(true),
    with_italic_complex_script => italic_complex_script: optional(false),
    with_size_complex_script => size_complex_script: optional(14.),
    with_rtl => rtl: optional(true), with_complex_script => complex_script: optional(false),
    with_font_east_asia => font_east_asia: optional("Yu Mincho"),
    with_font_complex_script => font_complex_script: optional("Arial"),
    with_linked_style => linked_style: optional("Builder character"),
    with_suppress_line_numbers => suppress_line_numbers: optional(false),
    with_snap_to_grid => snap_to_grid: optional(false),
    with_contextual_spacing => contextual_spacing: optional(true), with_mirror_indents => mirror_indents: optional(false),
);
check!(conditional_format, TableSectionFormat, TableSectionFormat::default();
    with_bold => bold: optional(true), with_italic => italic: optional(false),
    with_underline => underline: optional(true), with_font => font: optional("Arial"),
    with_font_size => font_size: optional(12.), with_color => color: optional([20u8, 40, 60]),
    with_background => background: optional([200u8, 220, 240]),
    with_alignment => alignment: optional(Alignment::Right),
    with_spacing_before => spacing_before: optional(6.), with_spacing_after => spacing_after: optional(8.),
    with_line_height => line_height: optional(LineHeight::Exact(14.)),
    with_tab_stops => tab_stops: value(vec![TabStop::default().with_position(36.)]),
    with_bidi => bidi: optional(false),
    with_language => language: optional(RunLanguage::default().with_latin("en-US")),
    with_bold_complex_script => bold_complex_script: optional(true),
    with_italic_complex_script => italic_complex_script: optional(false),
    with_size_complex_script => size_complex_script: optional(14.),
    with_rtl => rtl: optional(true), with_complex_script => complex_script: optional(false),
    with_font_east_asia => font_east_asia: optional("Yu Mincho"),
    with_font_complex_script => font_complex_script: optional("Arial"),
);
check!(table_style, TableStyleDefinition, TableStyleDefinition::default();
    with_name => name: value("Builder table"), with_base_style => base_style: optional("Table Grid"),
    with_whole_table => whole_table: optional(TableSectionFormat::default().with_bold(true)),
    with_first_row => first_row: optional(TableSectionFormat::default().with_bold(true)),
    with_last_row => last_row: optional(TableSectionFormat::default().with_bold(false)),
    with_first_column => first_column: optional(TableSectionFormat::default().with_italic(true)),
    with_last_column => last_column: optional(TableSectionFormat::default().with_italic(false)),
    with_banding_1_row => banding_1_row: optional(TableSectionFormat::default().with_bold(true)),
    with_banding_2_row => banding_2_row: optional(TableSectionFormat::default().with_bold(false)),
    with_banding_1_column => banding_1_column: optional(TableSectionFormat::default().with_italic(true)),
    with_banding_2_column => banding_2_column: optional(TableSectionFormat::default().with_italic(false)),
    with_top_left_cell => top_left_cell: optional(TableSectionFormat::default().with_bold(true)),
    with_top_right_cell => top_right_cell: optional(TableSectionFormat::default().with_bold(false)),
    with_bottom_left_cell => bottom_left_cell: optional(TableSectionFormat::default().with_italic(true)),
    with_bottom_right_cell => bottom_right_cell: optional(TableSectionFormat::default().with_italic(false)),
);
check!(character_style, CharacterStyleDefinition, CharacterStyleDefinition::default();
    with_name => name: value("Builder character"), with_base_style => base_style: optional("Default Paragraph Font"),
    with_format => format: value(RunFormat::default().with_bold(false)),
    with_linked_style => linked_style: optional("Builder style"),
);
check!(image_crop, ImageCrop, ImageCrop::default();
    with_left => left: value(10.), with_top => top: value(5.),
    with_right => right: value(20.), with_bottom => bottom: value(15.),
);
check!(shape_outline, ShapeOutline, ShapeOutline { width: 1., color: [0; 3] };
    with_width => width: value(2.), with_color => color: value([10u8, 20, 30]),
);
check!(shape_properties, ShapeProperties, ShapeProperties::default();
    with_width => width: value(40.), with_height => height: value(20.),
    with_geometry => geometry: value(ShapeGeometry::Ellipse), with_fill => fill: optional([1u8, 2, 3]),
    with_text => text: optional("Label"),
    with_outline => outline: optional(ShapeOutline { width: 1., color: [0; 3] }),
    with_text_format => text_format: optional(ShapeTextFormat::default().with_bold(false)),
    with_anchor => anchor: optional(ImageAnchor::default().with_x(12.)),
);
check!(shape_text_format, ShapeTextFormat, ShapeTextFormat::default();
    with_font_size => font_size: optional(14.),
    with_bold => bold: optional(false),
    with_color => color: optional([1u8, 2, 3]),
    with_alignment => alignment: optional(ShapeTextAlignment::Center),
    with_vertical_alignment => vertical_alignment: optional(ShapeTextVerticalAlignment::Bottom),
    with_italic => italic: optional(false),
    with_font => font: optional("Arial"),
);
check!(image_anchor, ImageAnchor, ImageAnchor::default();
    with_horizontal_reference => horizontal_reference: value(ImageAnchorReference::Margin),
    with_vertical_reference => vertical_reference: value(ImageAnchorReference::Margin),
    with_horizontal_alignment => horizontal_alignment: optional(ImageAnchorHorizontalAlignment::Center),
    with_vertical_alignment => vertical_alignment: optional(ImageAnchorVerticalAlignment::Bottom),
    with_x => x: value(-5.), with_y => y: value(18.),
    with_wrap => wrap: value(ImageWrap::Square),
    with_wrap_side => wrap_side: optional(ImageWrapSide::Largest),
    with_wrap_polygon => wrap_polygon: value(vec![(0, 0), (0, 21600), (21600, 0)]),
    with_distance_top => distance_top: optional(0.),
    with_distance_bottom => distance_bottom: optional(2.),
    with_distance_left => distance_left: optional(3.),
    with_distance_right => distance_right: optional(4.),
    with_relative_height => relative_height: value(4u32),
    with_behind_text => behind_text: value(true),
    with_locked => locked: optional(true),
    with_layout_in_cell => layout_in_cell: optional(false),
    with_allow_overlap => allow_overlap: optional(false),
);
check!(image_transformation, ImageTransformation, ImageTransformation::default();
    with_anchor => anchor: optional(ImageAnchor::default()),
    with_crop => crop: optional(ImageCrop::default().with_left(10.)),
    with_rotation => rotation: optional(45.),
    with_flip_horizontal => flip_horizontal: optional(true),
    with_flip_vertical => flip_vertical: optional(false),
    with_width => width: optional(72.), with_height => height: optional(36.),
);
check!(alt_text, AltText, AltText::default();
    with_title => title: optional("Title"), with_description => description: optional("Description"),
    with_name => name: optional("Name"),
);
check!(numbering_level, NumberingLevel, NumberingLevel::default();
    with_level => level: value(2u8), with_start => start: value(3u32),
    with_format => format: value("decimal"), with_text => text: value("%1."),
    with_is_legal => is_legal: optional(false), with_suffix => suffix: optional(NumberingSuffix::Space),
    with_restart => restart: optional(0u32), with_alignment => alignment: optional(NumberingAlignment::End),
    with_label_format => label_format: optional(RunFormat::default().with_bold(false)),
    with_paragraph_format => paragraph_format: optional(ParagraphFormat::default().with_left_indent(24.)),
    with_paragraph_style => paragraph_style: optional("Heading 1"),
);
check!(numbering_style, NumberingStyleDefinition, NumberingStyleDefinition { name: String::new(), numbering_id: 1 };
    with_name => name: value("Catalog List"), with_numbering_id => numbering_id: value(42u32),
);
check!(numbering_definition, NumberingDefinition, NumberingDefinition::default();
    with_id => id: value(42u32),
    with_abstract_id => abstract_id: optional(100u32),
    with_inherit_style => inherit_style: optional("Catalog List"),
    with_levels => levels: value(vec![NumberingLevel::default().with_start(1)]),
    with_start_overrides => start_overrides: value(vec![NumberingStartOverride { level: 0, start: 3 }]),
    with_level_overrides => level_overrides: value(vec![NumberingLevel::default().with_start(4)]),
);
check!(numbering_start, NumberingStartOverride, NumberingStartOverride { level: 0, start: 1 };
    with_level => level: value(2u8), with_start => start: value(5u32),
);
check!(numbering_reference, NumberingReference, NumberingReference::default();
    with_id => id: value(42u32), with_level => level: value(2u8),
);
check!(cell_margins, CellMargins, CellMargins::default();
    with_top => top: optional(3.), with_bottom => bottom: optional(4.),
    with_start => start: optional(0.), with_end => end: optional(6.),
);
check!(row_properties, TableRowProperties, TableRowProperties::default();
    with_height => height: optional(RowHeight::AtLeast(18.)),
    with_repeat_header => repeat_header: optional(false), with_cant_split => cant_split: optional(true),
    with_content_control => content_control: optional(ContentControlProperties::default().with_tag("body")),
);
check!(table_look, TableLook, TableLook::default();
    with_first_row => first_row: optional(true), with_last_row => last_row: optional(false),
    with_first_column => first_column: optional(true), with_last_column => last_column: optional(false),
    with_horizontal_banding => horizontal_banding: optional(true), with_vertical_banding => vertical_banding: optional(false),
);
check!(table_properties, TableProperties, TableProperties::default();
    with_column_widths => column_widths: optional(vec![1440u32, 2880]),
    with_layout => layout: optional(TableLayout::Fixed), with_style => style: optional("Table Grid"),
    with_borders => borders: value(vec![TableBorder::default().with_sz(8)]),
    with_look => look: optional(TableLook::default().with_first_row(false)),
    with_alignment => alignment: optional(TableAlignment::Center),
);
check!(cell_properties, TableCellProperties, TableCellProperties::default();
    with_grid_span => grid_span: optional(2u32), with_vmerge => vmerge: optional(VerticalMerge::Restart),
    with_borders => borders: value(vec![CellBorder::default().with_sz(8)]),
    with_background_color => background_color: optional([200u8, 220, 240]),
    with_vertical_alignment => vertical_alignment: optional(VerticalAlignment::Bottom),
    with_margins => margins: optional(CellMargins::default().with_top(3.)),
    with_text_direction => text_direction: optional(CellTextDirection::TopToBottomRightToLeft),
    with_content_control => content_control: optional(ContentControlProperties::default().with_tag("body")),
);
check!(hyperlink_properties, HyperlinkProperties, HyperlinkProperties::default();
    with_tooltip => tooltip: optional("Visit site"), with_history => history: optional(false),
);
check!(field_properties, FieldProperties, FieldProperties::default();
    with_form => form: value(FieldForm::Complex), with_dirty => dirty: optional(true),
    with_locked => locked: optional(false),
);
check!(comment_metadata, CommentMetadata, CommentMetadata::default();
    with_author => author: optional("Author"), with_date => date: optional("2026-11-20T00:00:00Z"),
    with_initials => initials: optional("AB"),
);
check!(metadata, Metadata, Metadata::default();
    with_creator => creator: optional("Creator"), with_title => title: optional("Title"),
    with_subject => subject: optional("Subject"), with_description => description: optional("Description"),
    with_keywords => keywords: optional("Keywords"), with_category => category: optional("Category"),
    with_comments => comments: optional("Comments"), with_version => version: optional("1.0"),
);
check!(note_target, NoteTarget, NoteTarget { kind: NoteKind::Footnote, id: NoteId::new(1) };
    with_kind => kind: value(NoteKind::Endnote), with_id => id: value(NoteId::new(2)),
);

#[test]
fn document_builders_replace_ready_and_deferred_configuration() {
    let document = Document::default()
        .with_notes(Vec::<Value<Note>>::new())
        .with_comment_author(Some("Author".into()))
        .with_timestamp(Value::future(async { Some("2026-11-20T00:00:00Z".into()) }))
        .with_styles(vec![
            ParagraphStyleDefinition::default()
                .with_name("Built")
                .into(),
        ])
        .with_numbering(vec![NumberingDefinition::default().with_id(42)])
        .with_defaults(Some(
            DocumentDefaults::default().with_run(RunFormat::default().with_bold(false)),
        ));
    assert!(block_on(document.notes.resolve()).unwrap().is_empty());
    assert_eq!(
        block_on(document.comment_author.resolve())
            .unwrap()
            .as_deref(),
        Some("Author")
    );
    assert_eq!(
        block_on(document.timestamp.resolve()).unwrap().as_deref(),
        Some("2026-11-20T00:00:00Z")
    );
    assert_eq!(block_on(document.styles.resolve()).unwrap().len(), 1);
    assert_eq!(block_on(document.numbering.resolve()).unwrap()[0].id, 42);
    assert_eq!(
        block_on(document.defaults.resolve())
            .unwrap()
            .unwrap()
            .run
            .unwrap()
            .bold,
        Some(false)
    );
}

#[test]
fn section_builders_replace_headers_and_footers() {
    let section = Section::default()
        .with_headers(
            [(
                HeaderFooterType::Default,
                Header::default().with_children(vec![]).into(),
            )]
            .into(),
        )
        .with_footers(
            [(
                HeaderFooterType::Default,
                Footer::default()
                    .with_children(Value::future(async { vec![] }))
                    .into(),
            )]
            .into(),
        );
    let header = block_on(section.headers.into_values().next().unwrap().resolve()).unwrap();
    let footer = block_on(section.footers.into_values().next().unwrap().resolve()).unwrap();
    assert!(block_on(header.children.resolve()).unwrap().is_empty());
    assert!(block_on(footer.children.resolve()).unwrap().is_empty());
}

#[test]
fn paragraph_and_run_builders_wrap_comment_values_without_evaluating_them() {
    let paragraph = Paragraph::default()
        .with_numbering(Some(NumberingReference::default().with_id(42)))
        .with_comment_text("comment")
        .with_comment_author(Value::future(async { "Author".into() }));
    assert_eq!(
        block_on(paragraph.numbering.resolve()).unwrap().unwrap().id,
        42
    );
    assert_eq!(
        block_on(paragraph.comment_text.unwrap().resolve()).unwrap(),
        "comment"
    );
    assert_eq!(
        block_on(paragraph.comment_author.unwrap().resolve()).unwrap(),
        "Author"
    );
    let run = TextRun::new("text")
        .with_comment_text(Value::future(async { "run comment".into() }))
        .with_comment_author("Other author");
    assert_eq!(
        block_on(run.comment_text.unwrap().resolve()).unwrap(),
        "run comment"
    );
    assert_eq!(
        block_on(run.comment_author.unwrap().resolve()).unwrap(),
        "Other author"
    );
}

#[test]
fn image_and_break_builders_replace_properties() {
    let image = ImageRun::new(vec![1u8])
        .with_data(Value::future(async { ImageData::Bytes(vec![2u8]) }))
        .with_type("png")
        .with_transformation(Some(ImageTransformation::default().with_width(72.)))
        .with_alt_text(Some(AltText::default().with_description("Picture")));
    assert_eq!(
        block_on(image.data.resolve()).unwrap(),
        ImageData::Bytes(vec![2u8])
    );
    assert_eq!(image.r#type.as_deref(), Some("png"));
    assert_eq!(
        block_on(image.transformation.resolve())
            .unwrap()
            .unwrap()
            .width,
        Some(72.)
    );
    assert_eq!(
        block_on(image.alt_text.resolve())
            .unwrap()
            .unwrap()
            .description
            .as_deref(),
        Some("Picture")
    );
    assert_eq!(
        Break::default().with_type(BreakType::Page).r#type,
        BreakType::Page
    );
}

#[test]
fn hyperlink_and_field_builders_replace_targets_instructions_and_children() {
    let target = HyperlinkTarget::external("https://example.com").unwrap();
    let link = Hyperlink::new(target.clone(), vec![TextRun::new("old").into()])
        .with_target(Value::future(async move { target }))
        .with_children(Vec::<Value<Inline>>::new());
    assert_eq!(
        block_on(link.target.resolve()).unwrap().as_str(),
        "https://example.com"
    );
    assert!(block_on(link.children.resolve()).unwrap().is_empty());
    let field = Field::new(FieldInstruction::Page, vec![TextRun::new("old").into()])
        .with_instruction(Value::future(async { FieldInstruction::NumPages }))
        .with_children(Vec::<Value<Inline>>::new());
    assert_eq!(
        block_on(field.instruction.resolve()).unwrap(),
        FieldInstruction::NumPages
    );
    assert!(block_on(field.children.resolve()).unwrap().is_empty());
}

#[test]
fn note_comment_and_range_builders_preserve_typed_targets() {
    let target = NoteTarget {
        kind: NoteKind::Endnote,
        id: NoteId::new(2),
    };
    let note = Note::footnote(NoteId::new(1), vec![])
        .with_target(target)
        .with_children(Value::future(async { vec![] }));
    assert_eq!(block_on(note.target.resolve()).unwrap(), target);
    assert!(block_on(note.children.resolve()).unwrap().is_empty());
    assert_eq!(
        block_on(
            NoteReference::footnote(NoteId::new(1))
                .with_target(target)
                .target
                .resolve()
        )
        .unwrap(),
        target
    );
    let comment = Comment::default().with_children(vec![Paragraph::new("comment").into()]);
    assert_eq!(block_on(comment.children.resolve()).unwrap().len(), 1);
    let name = BookmarkName::new("Built").unwrap();
    let id = RangeId::new("new identity").unwrap();
    let start = RangeStart::bookmark(name.clone())
        .with_id(id.clone())
        .with_kind(RangeKind::Bookmark(name.clone()));
    assert_eq!(block_on(start.id.resolve()).unwrap(), id);
    assert_eq!(
        block_on(start.kind.resolve()).unwrap(),
        RangeKind::Bookmark(name.clone())
    );
    let end = RangeEnd::bookmark(&name).with_id(id.clone());
    assert_eq!(block_on(end.id.resolve()).unwrap(), id);
}

#[test]
fn collection_setters_replace_instead_of_appending() {
    assert_eq!(
        ParagraphFormat::default()
            .with_tab_stops(vec![TabStop::default()])
            .with_tab_stops(vec![])
            .tab_stops,
        vec![]
    );
    assert_eq!(
        TableProperties::default()
            .with_column_widths(vec![1440])
            .with_column_widths(vec![2880])
            .column_widths,
        Some(vec![2880])
    );
    assert!(
        NumberingDefinition::default()
            .with_levels(vec![NumberingLevel::default()])
            .with_levels(vec![])
            .levels
            .is_empty()
    );
}

#[test]
fn builders_do_not_bypass_existing_validation() {
    let invalid = ParagraphFormat::default()
        .with_line_spacing(1.5)
        .with_line_height(LineHeight::Exact(14.));
    assert!(
        block_on(
            Document::new(vec![
                Section::new(vec![Paragraph::new("invalid").with_format(invalid).into()]).into()
            ])
            .resolve()
        )
        .is_err()
    );
}

#[test]
fn component_builders_preserve_owned_deferred_values_and_hidden_producers() {
    let document = Document::default()
        .with_metadata(Value::future(async {
            Metadata::default().with_title("Built")
        }))
        .with_sections(vec![
            Section::default()
                .with_properties(Some(SectionProperties::default().with_bidi(false)))
                .with_children(Value::future(async {
                    vec![
                        Paragraph::default()
                            .with_text("deferred")
                            .with_format(Value::future(async {
                                ParagraphFormat::default().with_keep_with_next(false)
                            }))
                            .into(),
                        Table::default()
                            .with_properties(Value::future(async {
                                TableProperties::default()
                                    .with_column_widths(vec![1440])
                                    .with_alignment(TableAlignment::Center)
                            }))
                            .with_rows(vec![
                                TableRow::default()
                                    .with_properties(
                                        TableRowProperties::default().with_cant_split(false),
                                    )
                                    .with_children(vec![
                                        TableCell::default()
                                            .with_properties(
                                                TableCellProperties::default().with_grid_span(1),
                                            )
                                            .with_children(vec![Paragraph::new("cell").into()])
                                            .into(),
                                    ])
                                    .into(),
                            ])
                            .into(),
                        Paragraph::new("hidden")
                            .with_format(Value::future(async { panic!("hidden builder ran") }))
                            .when(false)
                            .into(),
                    ]
                }))
                .into(),
        ]);
    let snapshot = block_on(document.resolve()).unwrap();
    assert_eq!(snapshot.sections()[0].blocks().len(), 2);
    assert_eq!(snapshot.to_docx().unwrap(), snapshot.to_docx().unwrap());
}
