//! Cross-language readback tests: Rust writes files; python-docx reads them.
#![cfg(feature = "python-tests")]

#[path = "support/cell_layout.rs"]
mod cell_layout_fixture;
#[path = "support/character_styles.rs"]
mod character_fixture;
#[path = "support/conditional_paragraph_formatting.rs"]
mod conditional_paragraph_fixture;
#[path = "support/conditional_script_formatting.rs"]
mod conditional_script_fixture;
#[path = "support/document_defaults.rs"]
mod defaults_fixture;
#[path = "support/features.rs"]
mod features;
#[path = "support/fields.rs"]
mod field_fixture;
#[path = "support/linked_styles.rs"]
mod linked_styles_fixture;
#[path = "support/notes.rs"]
mod note_fixture;
#[path = "support/numbering_alignment.rs"]
mod numbering_alignment_fixture;
#[path = "support/numbering_overrides.rs"]
mod numbering_override_fixture;
#[path = "support/numbering_replacements.rs"]
mod numbering_replacement_fixture;
#[path = "support/numbering_restart.rs"]
mod numbering_restart_fixture;
#[path = "support/numbering_suffix.rs"]
mod numbering_suffix_fixture;
#[path = "support/ranges.rs"]
mod range_fixture;
#[path = "support/row_properties.rs"]
mod row_fixture;
#[path = "support/script_formatting.rs"]
mod script_fixture;
#[path = "support/section_direction.rs"]
mod section_direction_fixture;
#[path = "support/section_gutter.rs"]
mod section_gutter_fixture;
#[path = "support/tab_stops.rs"]
mod tab_fixture;
#[path = "support/table_alignment.rs"]
mod table_alignment_fixture;
#[path = "support/table_look.rs"]
mod table_look_fixture;

use futures::executor::block_on;
use rocx::*;
use std::{
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

#[test]
fn python_reads_table_alignment_and_rejects_a_changed_table_request() {
    let temp = Temp::new();
    let path = temp.path("table-alignment.docx");
    save(table_alignment_fixture::document(), &path);
    verify("table_alignment", &path, None);
    let changed = temp.path("wrong-table-alignment.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "<w:jc w:val=\"end\"", "<w:jc w:val=\"start\"")
    });
    let output = python("table_alignment", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_numbering_alignment_and_rejects_a_changed_label_alignment() {
    let temp = Temp::new();
    let path = temp.path("numbering-alignment.docx");
    save(numbering_alignment_fixture::document(), &path);
    verify("numbering_alignment", &path, None);
    let changed = temp.path("wrong-numbering-alignment.docx");
    rewrite_part(&path, &changed, "word/numbering.xml", |xml| {
        replace_once(xml, "<w:lvlJc w:val=\"end\"", "<w:lvlJc w:val=\"start\"")
    });
    let output = python("numbering_alignment", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_numbering_restart_requests_and_rejects_a_changed_trigger() {
    let temp = Temp::new();
    let path = temp.path("restart.docx");
    save(numbering_restart_fixture::document(), &path);
    verify("numbering_restart", &path, None);
    let changed = temp.path("wrong-restart.docx");
    rewrite_part(&path, &changed, "word/numbering.xml", |xml| {
        replace_once(
            xml,
            "<w:lvlRestart w:val=\"0\"",
            "<w:lvlRestart w:val=\"1\"",
        )
    });
    let output = python("numbering_restart", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_numbering_suffixes_and_rejects_a_changed_separator() {
    let temp = Temp::new();
    let path = temp.path("suffix.docx");
    save(numbering_suffix_fixture::document(), &path);
    verify("numbering_suffix", &path, None);
    let changed = temp.path("wrong-suffix.docx");
    rewrite_part(&path, &changed, "word/numbering.xml", |xml| {
        replace_once(xml, "<w:suff w:val=\"space\"", "<w:suff w:val=\"nothing\"")
    });
    let output = python("numbering_suffix", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_table_look_and_rejects_a_changed_activation_flag() {
    let temp = Temp::new();
    let path = temp.path("look.docx");
    save(table_look_fixture::document(), &path);
    verify("table_look", &path, None);
    let changed = temp.path("wrong-look.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "w:lastRow=\"1\"", "w:lastRow=\"0\"")
    });
    let output = python("table_look", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_concrete_numbering_replacements_and_rejects_changed_level_text() {
    let temp = Temp::new();
    let path = temp.path("replacements.docx");
    save(numbering_replacement_fixture::document(), &path);
    verify("numbering_replacements", &path, None);
    let changed = temp.path("wrong-replacement.docx");
    rewrite_part(&path, &changed, "word/numbering.xml", |xml| {
        replace_once(xml, "w:val=\"Replacement-%1\"", "w:val=\"Wrong-%1\"")
    });
    let output = python("numbering_replacements", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_section_gutter_and_rejects_a_changed_placement_flag() {
    let temp = Temp::new();
    let path = temp.path("gutter.docx");
    save(section_gutter_fixture::document(), &path);
    verify("section_gutter", &path, None);
    let changed = temp.path("wrong-gutter.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "<w:rtlGutter w:val=\"1\"", "<w:rtlGutter w:val=\"0\"")
    });
    let output = python("section_gutter", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_linked_styles_and_rejects_a_changed_link() {
    let temp = Temp::new();
    let path = temp.path("linked.docx");
    save(linked_styles_fixture::document(), &path);
    verify("linked_styles", &path, None);
    let changed = temp.path("wrong-link.docx");
    rewrite_part(&path, &changed, "word/styles.xml", |xml| {
        replace_once(
            xml,
            "<w:link w:val=\"PairCharacter\"",
            "<w:link w:val=\"PairParagraph\"",
        )
    });
    let output = python("linked_styles", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_document_defaults_and_rejects_a_changed_default_size() {
    let temp = Temp::new();
    let path = temp.path("defaults.docx");
    save(defaults_fixture::document(), &path);
    verify("document_defaults", &path, None);
    let changed = temp.path("wrong-default.docx");
    rewrite_part(&path, &changed, "word/styles.xml", |xml| {
        replace_once(xml, "<w:sz w:val=\"27\"", "<w:sz w:val=\"26\"")
    });
    let output = python("document_defaults", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_conditional_script_formatting_and_rejects_changed_rtl() {
    let temp = Temp::new();
    let path = temp.path("scripts.docx");
    save(conditional_script_fixture::document(), &path);
    verify("conditional_script_formatting", &path, None);
    let changed = temp.path("wrong-rtl.docx");
    rewrite_part(&path, &changed, "word/styles.xml", |xml| {
        replace_once(xml, "<w:rtl w:val=\"0\"", "<w:rtl w:val=\"1\"")
    });
    let output = python("conditional_script_formatting", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_conditional_paragraph_formatting_and_rejects_changed_line_height() {
    let temp = Temp::new();
    let path = temp.path("conditional.docx");
    save(conditional_paragraph_fixture::document(), &path);
    verify("conditional_paragraph_formatting", &path, None);
    let changed = temp.path("wrong-height.docx");
    rewrite_part(&path, &changed, "word/styles.xml", |xml| {
        replace_once(xml, "w:line=\"290\"", "w:line=\"20\"")
    });
    let output = python("conditional_paragraph_formatting", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_section_direction_and_rejects_a_changed_section_flag() {
    let temp = Temp::new();
    let path = temp.path("section-direction.docx");
    save(section_direction_fixture::document(), &path);
    verify("section_direction", &path, None);
    let changed = temp.path("wrong-direction.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "<w:bidi w:val=\"1\"", "<w:bidi w:val=\"0\"")
    });
    let output = python("section_direction", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rocx-python-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn save(document: Document, path: &Path) {
    block_on(document.resolve()).unwrap().save(path).unwrap();
    assert!(path.is_file(), "Rust did not create {}", path.display());
}

fn python(case: &str, document: &Path, source: Option<&Path>) -> Output {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = project.join("tests/python_docx/verify.py");
    let mut command = Command::new("uv");
    command
        .args(["run", "--locked", "--group", "test", "--project"])
        .arg(project)
        .arg("python")
        .arg(script)
        .arg(case)
        .arg(document);
    if let Some(source) = source {
        command.arg("--source").arg(source);
    }
    command.output().unwrap_or_else(|error| {
        panic!(
            "Could not run uv: {error}. Install uv and run `uv sync --locked --group test` \
             from the repository root."
        )
    })
}

fn verify(case: &str, document: &Path, source: Option<&Path>) {
    let output = python(case, document, source);
    assert!(
        output.status.success(),
        "python-docx readback failed for {case} ({}).\n\
         Run `uv sync --locked --group test` from the repository root.\n\
         stdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn python_opens_an_empty_rust_document() {
    let temp = Temp::new();
    let path = temp.path("empty.docx");
    save(Document::default(), &path);
    verify("empty", &path, None);
}

#[test]
fn python_reads_field_instructions_and_cached_results_and_rejects_mutations() {
    let temp = Temp::new();
    let path = temp.path("fields.docx");
    save(field_fixture::document(), &path);
    verify("fields", &path, None);
    for (name, before, after) in [
        ("wrong-cache", "Contents pending", "Contents changed"),
        ("wrong-code", " NUMPAGES ", " PAGE "),
        (
            "unbalanced",
            "w:fldCharType=\"end\"",
            "w:fldCharType=\"separate\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/document.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("fields", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_typed_line_heights_and_rejects_changed_units_or_rules() {
    let temp = Temp::new();
    let path = temp.path("line-heights.docx");
    let mut document = Document::new(vec![
        Section::new(vec![
            Paragraph::new("multiple")
                .with_format(ParagraphFormat {
                    line_height: Some(LineHeight::Multiple(1.5)),
                    ..Default::default()
                })
                .into(),
            Paragraph::new("exact")
                .with_format(ParagraphFormat {
                    line_height: Some(LineHeight::Exact(14.25)),
                    ..Default::default()
                })
                .into(),
            Paragraph::new("minimum")
                .with_format(ParagraphFormat {
                    line_height: Some(LineHeight::AtLeast(18.)),
                    ..Default::default()
                })
                .into(),
            Paragraph::new("inherited")
                .with_format(ParagraphFormat {
                    style: Some("Measured".into()),
                    ..Default::default()
                })
                .into(),
        ])
        .into(),
    ]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Measured".into(),
            line_height: Some(LineHeight::AtLeast(20.)),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    save(document, &path);
    verify("line_heights", &path, None);
    for (name, owner, before, after) in [
        (
            "wrong-units",
            "word/document.xml",
            "w:line=\"285\"",
            "w:line=\"14\"",
        ),
        (
            "wrong-rule",
            "word/document.xml",
            "w:lineRule=\"exact\"",
            "w:lineRule=\"atLeast\"",
        ),
        (
            "wrong-style",
            "word/styles.xml",
            "w:line=\"400\"",
            "w:line=\"20\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, owner, |xml| {
            replace_once(xml, before, after)
        });
        let output = python("line_heights", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_custom_tab_stops_and_rejects_changed_position_alignment_or_leader() {
    let temp = Temp::new();
    let path = temp.path("tabs.docx");
    save(tab_fixture::document(), &path);
    verify("tab_stops", &path, None);
    for (name, before, after) in [
        ("position", "w:pos=\"2160\"", "w:pos=\"108\""),
        ("alignment", "w:val=\"right\"", "w:val=\"left\""),
        ("leader", "w:leader=\"dot\"", "w:leader=\"heavy\""),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/document.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("tab_stops", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_row_properties_and_rejects_changed_height_rule_or_flags() {
    let temp = Temp::new();
    let path = temp.path("row-properties.docx");
    save(row_fixture::document(), &path);
    verify("row_properties", &path, None);
    for (name, before, after) in [
        ("height", "w:val=\"285\"", "w:val=\"14\""),
        ("rule", "w:hRule=\"exact\"", "w:hRule=\"auto\""),
        (
            "header",
            "<w:tblHeader w:val=\"1\"",
            "<w:tblHeader w:val=\"0\"",
        ),
        (
            "split",
            "<w:cantSplit w:val=\"1\"",
            "<w:cantSplit w:val=\"0\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/document.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("row_properties", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_cell_layout_and_rejects_changed_margin_or_direction() {
    let temp = Temp::new();
    let path = temp.path("cell-layout.docx");
    save(cell_layout_fixture::document(), &path);
    verify("cell_layout", &path, None);
    for (name, before, after) in [
        ("margin", "w:w=\"63\"", "w:w=\"3\""),
        ("direction", "w:val=\"tbRl\"", "w:val=\"lrTb\""),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/document.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("cell_layout", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_script_formatting_and_rejects_changed_language_flag_or_size() {
    let temp = Temp::new();
    let path = temp.path("scripts.docx");
    save(script_fixture::document(), &path);
    verify("script_formatting", &path, None);
    for (name, before, after) in [
        ("language", "ar-SA", "en-US"),
        ("font", "Arabic", "Other"),
        ("size", "w:val=\"29\"", "w:val=\"14\""),
        ("bold", "<w:bCs w:val=\"1\"", "<w:bCs w:val=\"0\""),
        ("rtl", "<w:rtl w:val=\"1\"", "<w:rtl w:val=\"0\""),
        (
            "paragraph-bidi",
            "<w:bidi w:val=\"1\"",
            "<w:bidi w:val=\"0\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/document.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("script_formatting", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_forward_style_references_and_rejects_retargeted_links() {
    let temp = Temp::new();
    let path = temp.path("styles.docx");
    let mut document = Document::new(vec![]);
    document.styles = vec![
        ParagraphStyleDefinition {
            name: "Child".into(),
            base_style: Some("Later Base".into()),
            next_paragraph_style: Some("Later Next".into()),
            ..Default::default()
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Later Base".into(),
            ..Default::default()
        }
        .into(),
        ParagraphStyleDefinition {
            name: "Later Next".into(),
            next_paragraph_style: Some("Later Next".into()),
            ..Default::default()
        }
        .into(),
    ]
    .into();
    save(document, &path);
    verify("style_references", &path, None);
    for (name, before, after) in [
        (
            "base",
            "<w:basedOn w:val=\"LaterBase\"",
            "<w:basedOn w:val=\"Normal\"",
        ),
        (
            "next",
            "<w:next w:val=\"LaterNext\"",
            "<w:next w:val=\"Normal\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, "word/styles.xml", |xml| {
            replace_once(xml, before, after)
        });
        let output = python("style_references", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_character_styles_and_rejects_retargeted_run_style() {
    let temp = Temp::new();
    let path = temp.path("character-styles.docx");
    save(character_fixture::document(), &path);
    verify("character_styles", &path, None);
    let changed = temp.path("retargeted.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "ChildCharacter", "LaterCharacter")
    });
    let output = python("character_styles", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_numbering_override_graph_and_rejects_changed_start() {
    let temp = Temp::new();
    let path = temp.path("numbering-overrides.docx");
    save(numbering_override_fixture::document(), &path);
    verify("numbering_overrides", &path, None);
    let changed = temp.path("wrong-start.docx");
    rewrite_part(&path, &changed, "word/numbering.xml", |xml| {
        replace_once(
            xml,
            "<w:startOverride w:val=\"7\"",
            "<w:startOverride w:val=\"8\"",
        )
    });
    let output = python("numbering_overrides", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_note_and_structured_comment_stories_and_rejects_semantic_mutations() {
    let temp = Temp::new();
    let path = temp.path("notes.docx");
    save(note_fixture::document(), &path);
    verify("notes", &path, None);
    for (name, part, before, after) in [
        (
            "wrong-note-text",
            "word/footnotes.xml",
            "Foot λ",
            "Lost foot",
        ),
        (
            "wrong-note-reference",
            "word/document.xml",
            "w:footnoteReference w:id=\"0\"",
            "w:footnoteReference w:id=\"99\"",
        ),
    ] {
        let changed = temp.path(&format!("{name}.docx"));
        rewrite_part(&path, &changed, part, |xml| {
            replace_once(xml, before, after)
        });
        let output = python("notes", &changed, None);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
    }
}

#[test]
fn python_reads_overlapping_ranges_and_rejects_a_changed_bookmark_endpoint() {
    let temp = Temp::new();
    let path = temp.path("ranges.docx");
    save(range_fixture::document(), &path);
    verify("ranges", &path, None);
    let changed = temp.path("ranges-wrong-endpoint.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(
            xml,
            "w:bookmarkEnd w:id=\"0\"",
            "w:bookmarkEnd w:id=\"999\"",
        )
    });
    let output = python("ranges", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_rich_hyperlinks_and_rejects_a_changed_external_target() {
    let temp = Temp::new();
    let path = temp.path("hyperlinks.docx");
    let link = || {
        Hyperlink::new(
            HyperlinkTarget::external("https://example.invalid/λ?q=a%20b&lang=日本#résumé")
                .unwrap(),
            vec![
                TextRun::new("Read ")
                    .with_format(RunFormat {
                        bold: Some(true),
                        ..Default::default()
                    })
                    .into(),
                TextRun::new("more λ")
                    .with_format(RunFormat {
                        italic: Some(true),
                        ..Default::default()
                    })
                    .into(),
            ],
        )
    };
    let mut section = Section::new(vec![
        Paragraph::from_children(vec![link().into()]).into(),
        Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
            ])
            .into(),
        ])
        .into(),
    ]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::from_children(vec![link().into()]).into()]).into(),
    );
    save(Document::new(vec![section.into()]), &path);
    verify("hyperlinks", &path, None);
    let changed = temp.path("hyperlinks-wrong-target.docx");
    rewrite_part(&path, &changed, "word/_rels/document.xml.rels", |xml| {
        replace_once(xml, "example.invalid", "changed.invalid")
    });
    let output = python("hyperlinks", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

#[test]
fn python_reads_deferred_metadata_properties_and_shared_resolved_children() {
    let temp = Temp::new();
    let path = temp.path("r1.docx");
    let children = Property::new(
        "children",
        Value::try_future(async {
            let paragraph = Paragraph::new("shared R1 λ").resolve().await?.unwrap();
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
    let mut cell = TableCell {
        children: content(children.reference()),
        ..Default::default()
    };
    cell.properties = Property::new(
        "cell.properties",
        Value::future(async {
            TableCellProperties {
                background_color: Some([12, 34, 56]),
                ..Default::default()
            }
        }),
    )
    .into();
    let mut table = Table::new(vec![TableRow::new(vec![cell.into()]).into()]);
    table.properties = Property::new(
        "table.properties",
        Value::future(async {
            TableProperties {
                column_widths: Some(vec![1440]),
                layout: Some(TableLayout::Fixed),
                ..Default::default()
            }
        }),
    )
    .into();
    let mut section = Section::new(vec![table.into()]);
    section.properties = Property::new(
        "section.properties",
        Value::future(async {
            Some(SectionProperties {
                page_margins: Some(PageMargins {
                    left: Some(914400),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }),
    )
    .into();
    section.headers.insert(
        HeaderFooterType::Default,
        Header {
            children: content(children.reference()),
            ..Default::default()
        }
        .into(),
    );
    let mut document = Document::new(vec![section.into()]);
    document.metadata = Property::new(
        "metadata",
        Value::future(async {
            Metadata {
                title: Some("R1 λ".into()),
                creator: Some("Ada".into()),
                ..Default::default()
            }
        }),
    )
    .into();
    save(document, &path);
    verify("r1", &path, None);
    let changed = temp.path("r1-wrong-content.docx");
    rewrite_part(&path, &changed, "word/document.xml", |xml| {
        replace_once(xml, "shared R1 λ", "lost content")
    });
    let output = python("r1", &changed, None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Loaded DOCX:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssertionError"));
}

fn save_text_fixture(path: &Path) {
    let paragraph = Paragraph::default().with_children(Value::lazy(|| {
        vec![
            TextRun {
                format: RunFormat {
                    bold: Some(true),
                    ..RunFormat::default()
                }
                .into(),
                ..TextRun::new("First ")
            }
            .into(),
            TextRun::new("").into(),
            TextRun::new(Value::lazy(|| panic!("hidden text must not run")))
                .when(false)
                .into(),
            Value::future(async { Inline::from(TextRun::new("async")) }),
            TextRun::new(Value::lazy(|| " lazy".to_owned())).into(),
            Tab::default().into(),
            Break::default().into(),
        ]
    }));
    save(
        Document::new(vec![
            Section::new(vec![
                Paragraph::new("Unicode: café Ελληνικά 日本語 🌍 <tag> & \"quotes\"").into(),
                Paragraph::new("  leading  middle  trailing  \tend\nnext\r\nCRLF").into(),
                Paragraph::new("").into(),
                Paragraph::from_children(vec![]).into(),
                paragraph.into(),
            ])
            .into(),
            Section::default()
                .with_children(Value::lazy(|| panic!("hidden section")))
                .when(false)
                .into(),
        ]),
        path,
    );
}

#[test]
fn python_reads_rust_text_runs_whitespace_and_async_conditions() {
    let temp = Temp::new();
    let path = temp.path("text.docx");
    save_text_fixture(&path);
    verify("text", &path, None);
}

#[test]
fn python_reads_rust_shared_property_dependencies() {
    let temp = Temp::new();
    let path = temp.path("dependencies.docx");
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let result = Value::future(async move {
        count.fetch_add(1, Ordering::SeqCst);
        "42 λ".to_owned()
    })
    .deferred();
    let text = result.clone();
    let comment = result.clone();
    let summary = Paragraph {
        comment_text: Some(Value::try_future(async move {
            Ok::<_, Error>(format!("Source: {}", comment.await?))
        })),
        comment_author: Some(Value::future(async { "Ada".to_owned() }).deferred().into()),
        ..Paragraph::new(Value::try_future(async move {
            Ok::<_, Error>(format!("The result is {}", text.await?))
        }))
    };
    save(
        Document::new(vec![
            Section::new(vec![summary.into(), Paragraph::new(result).into()]).into(),
        ]),
        &path,
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    verify("dependencies", &path, None);
}

#[test]
fn python_reads_rust_formatting_styles_tables_comments_and_sections() {
    let temp = Temp::new();
    let path = temp.path("rich.docx");
    save(features::document(), &path);
    verify("rich", &path, None);
}

fn save_images_fixture(path: &Path) {
    let mut png = ImageRun::deferred(Value::future(async {
        ImageData::Bytes(include_bytes!("fixtures/images/sample.png").to_vec())
    }));
    png.transformation = Value::future(async {
        Some(ImageTransformation {
            width: Some(36.),
            height: None,
        })
    });
    png.alt_text = Value::future(async {
        Some(AltText {
            title: Some("Title <&>".into()),
            description: Some("Red sample".into()),
            name: Some("Sample PNG".into()),
        })
    });
    let jpeg = ImageRun::from_path(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images/sample.jpg"),
    );
    let mut tiff = ImageRun::new(include_bytes!("fixtures/images/sample.tiff").to_vec());
    tiff.transformation = Some(ImageTransformation {
        width: None,
        height: Some(24.),
    })
    .into();
    let mut repeated_png = ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec());
    repeated_png.transformation = Some(ImageTransformation {
        width: Some(72.),
        height: None,
    })
    .into();
    let images = vec![
        png,
        jpeg,
        ImageRun::new(include_bytes!("fixtures/images/sample.gif").to_vec()),
        ImageRun::new(include_bytes!("fixtures/images/sample.bmp").to_vec()),
        tiff,
        repeated_png,
    ];
    save(
        Document::new(vec![
            Section::new(vec![
                Paragraph::from_children(images.into_iter().map(Into::into).collect()).into(),
            ])
            .into(),
        ]),
        path,
    );
}

#[test]
fn python_reads_rust_images_with_dpi_sizing_and_accessibility_metadata() {
    let temp = Temp::new();
    let path = temp.path("images.docx");
    save_images_fixture(&path);
    verify("images", &path, None);
}

#[test]
fn python_reads_rust_section_layout_and_custom_numbering() {
    let temp = Temp::new();
    let path = temp.path("layout.docx");
    let mut paragraph = Paragraph::new("Numbered item");
    paragraph.numbering = Some(NumberingReference { id: 42, level: 0 }).into();
    let mut section = Section::new(vec![paragraph.into()]);
    section.properties = Some(SectionProperties {
        page_orientation: Some(Orientation::Landscape),
        page_margins: Some(PageMargins {
            top: Some(914400),
            bottom: Some(914400),
            left: Some(457200),
            right: Some(685800),
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
        ..SectionProperties::default()
    })
    .into();
    let mut document = Document::new(vec![section.into()]);
    document.numbering = vec![NumberingDefinition {
        id: 42,
        start_overrides: Vec::new(),
        level_overrides: Vec::new(),
        levels: vec![NumberingLevel {
            level: 0,
            start: 3,
            format: "decimal".into(),
            text: "%1.".into(),
            is_legal: None,
            suffix: None,
            restart: None,
            alignment: None,
        }],
    }]
    .into();
    save(document, &path);
    verify("layout", &path, None);
}

fn save_template_fixture(temp: &Temp) -> (PathBuf, PathBuf) {
    let source = temp.path("source.docx");
    let output = temp.path("edited.docx");
    let mut greeting = Paragraph::from_children(vec![
        TextRun {
            format: RunFormat {
                bold: Some(true),
                ..RunFormat::default()
            }
            .into(),
            comment_text: Some("Original note".into()),
            comment_author: Some("Original Author".into()),
            ..TextRun::new("Hello {{NA")
        }
        .into(),
        TextRun {
            format: RunFormat {
                italic: Some(true),
                ..RunFormat::default()
            }
            .into(),
            ..TextRun::new("ME}}")
        }
        .into(),
    ]);
    greeting.format = ParagraphFormat {
        style: Some("TemplateBody".into()),
        ..ParagraphFormat::default()
    }
    .into();
    let mut image = ImageRun::new(include_bytes!("fixtures/images/sample.png").to_vec());
    image.transformation = Some(ImageTransformation {
        width: Some(48.),
        height: None,
    })
    .into();
    let mut section = Section::new(vec![
        greeting.into(),
        Table::new(vec![
            TableRow::new(vec![
                TableCell::new(vec![Paragraph::new("{{NAME}}").into()]).into(),
            ])
            .into(),
        ])
        .into(),
        Paragraph::from_children(vec![image.into()]).into(),
        Paragraph::new("Tail").into(),
    ]);
    section.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("Header {{NAME}}").into()]).into(),
    );
    section.footers.insert(
        HeaderFooterType::Default,
        Footer::new(vec![Paragraph::new("Footer {{NAME}}").into()]).into(),
    );
    let mut template_document = Document::new(vec![section.into()]);
    template_document.metadata = Metadata {
        title: Some("Template title".into()),
        subject: Some("Retained subject".into()),
        creator: Some("Template Author".into()),
        ..Default::default()
    }
    .into();
    template_document.styles = vec![
        ParagraphStyleDefinition {
            name: "TemplateBody".into(),
            base_style: Some("Normal".into()),
            font: Some("Courier New".into()),
            font_size: Some(11.),
            ..ParagraphStyleDefinition::default()
        }
        .into(),
    ]
    .into();
    save(template_document, &source);

    let mut addition = Section::new(vec![
        Paragraph::new("Inserted {{NAME}}").into(),
        Paragraph::from_children(vec![
            ImageRun::new(include_bytes!("fixtures/images/sample.jpg").to_vec()).into(),
        ])
        .into(),
        Paragraph::from_children(vec![
            TextRun {
                comment_text: Some("New note".into()),
                ..TextRun::new("Additional comment")
            }
            .into(),
        ])
        .into(),
    ]);
    addition.headers.insert(
        HeaderFooterType::Default,
        Header::new(vec![Paragraph::new("Header extension").into()]).into(),
    );
    let mut document = Document::new(vec![addition.into()]);
    document.metadata = Metadata {
        title: Some("Updated title".into()),
        ..Default::default()
    }
    .into();
    document.comment_author = Some("New Author".into()).into();
    let mut template = DocumentTemplate::new(&source);
    template.edits.replacements = vec![("{{NAME}}".into(), "Alice".into())];
    template.edits.paragraph_index = Some(1);
    block_on(document.resolve())
        .unwrap()
        .save_with_template(&output, &template)
        .unwrap();
    (source, output)
}

#[test]
fn python_reads_rust_template_replacements_and_preserved_content() {
    let temp = Temp::new();
    let (source, output) = save_template_fixture(&temp);
    verify("template", &output, Some(&source));
}

#[test]
fn python_verifier_rejects_a_file_with_incorrect_content() {
    let temp = Temp::new();
    let path = temp.path("incorrect.docx");
    save(
        Document::new(vec![
            Section::new(vec![Paragraph::new("Incorrect content").into()]).into(),
        ]),
        &path,
    );
    let output = python("text", &path, None);
    assert!(
        !output.status.success(),
        "Verifier accepted incorrect content"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("AssertionError"),
        "Verifier failed for an unexpected reason:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// These mutations keep the ZIP and XML readable. The Python verifier must reject
// the changed semantic value, rather than merely succeed at opening the file.
fn rewrite_part(
    source: &Path,
    destination: &Path,
    part: &str,
    change: impl FnOnce(&str) -> String,
) {
    let source = std::fs::read(source).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(source)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut change = Some(change);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        if entry.name() == part {
            let original = std::str::from_utf8(&content).unwrap();
            let edited = change.take().expect("duplicate mutation target")(original);
            assert_ne!(original, edited, "mutation did not change {part}");
            // Fail the Rust test if the mutation accidentally breaks XML syntax.
            roxmltree::Document::parse(&edited).unwrap();
            content = edited.into_bytes();
        }
        writer
            .start_file(entry.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    assert!(change.is_none(), "missing mutation target: {part}");
    std::fs::write(destination, writer.finish().unwrap().into_inner()).unwrap();
}

fn replace_once(xml: &str, old: &str, new: &str) -> String {
    assert!(xml.contains(old), "fixture does not contain {old:?}");
    xml.replacen(old, new, 1)
}

fn replace_attribute(
    xml: &str,
    element: &str,
    attribute: &str,
    old: &str,
    new: &str,
    style: Option<&str>,
) -> String {
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let range = parsed
        .descendants()
        .filter(|node| {
            node.tag_name().name() == element
                && style.is_none_or(|style| {
                    node.ancestors().any(|n| {
                        n.attribute((
                            "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                            "styleId",
                        )) == Some(style)
                    })
                })
        })
        .flat_map(|node| node.attributes())
        .find(|a| a.name() == attribute && a.value() == old)
        .unwrap_or_else(|| panic!("missing {element}/@{attribute}={old:?} in fixture"))
        .range_value();
    let mut result = xml.to_owned();
    result.replace_range(range, new);
    result
}

fn extend_first_comment(xml: &str) -> String {
    const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let parsed = roxmltree::Document::parse(xml).unwrap();
    let end = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "commentRangeEnd")) && n.attribute((W, "id")) == Some("0"))
        .unwrap()
        .range();
    let reference = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "commentReference")) && n.attribute((W, "id")) == Some("0"))
        .unwrap()
        .ancestors()
        .find(|n| n.has_tag_name((W, "r")))
        .unwrap()
        .range();
    let target = parsed
        .descendants()
        .find(|n| n.has_tag_name((W, "t")) && n.text() == Some("sup"))
        .unwrap()
        .ancestors()
        .find(|n| n.has_tag_name((W, "r")))
        .unwrap()
        .range();
    assert!(end.end <= reference.start && reference.end <= target.start);
    let moved = &xml[end.start..reference.end];
    let mut result = xml.to_owned();
    result.replace_range(end.start..reference.end, "");
    result.insert_str(target.end - moved.len(), moved);
    result
}

fn verify_rejected(case: &str, document: &Path, source: Option<&Path>, mismatch: &str) {
    let output = python(case, document, source);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("Loaded DOCX:"),
        "mutation was not readable:\n{stderr}"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "verifier accepted {}: expected {mismatch}\n{stderr}",
        document.display()
    );
    assert!(
        stderr.contains("AssertionError")
            && stderr.contains(mismatch)
            && !stderr.contains("ERROR:"),
        "expected an assertion for {mismatch}, not a parser/setup failure:\n{stderr}"
    );
}

type Mutation = (&'static str, &'static str, fn(&str) -> String, &'static str);

#[test]
fn python_assertions_reject_individual_content_and_formatting_regressions() {
    let temp = Temp::new();
    let baseline = temp.path("rich-baseline.docx");
    save(features::document(), &baseline);
    verify("rich", &baseline, None);
    let mutations: &[Mutation] = &[
        (
            "paragraph_text",
            "word/document.xml",
            |xml| replace_once(xml, "Feature parity report", "Wrong report title"),
            "document.paragraphs.text",
        ),
        (
            "run_bold",
            "word/document.xml",
            |xml| replace_attribute(xml, "b", "val", "1", "0", None),
            "run.bold",
        ),
        (
            "paragraph_spacing",
            "word/document.xml",
            |xml| replace_attribute(xml, "spacing", "before", "240", "260", None),
            "paragraph_property='space_before'",
        ),
        (
            "style_font_size",
            "word/styles.xml",
            |xml| replace_attribute(xml, "sz", "val", "22", "24", Some("ReportBody")),
            "style.font.size",
        ),
        (
            "column_width",
            "word/document.xml",
            |xml| replace_attribute(xml, "gridCol", "w", "1000", "1200", None),
            "table.column_widths",
        ),
        (
            "cell_shading",
            "word/document.xml",
            |xml| replace_attribute(xml, "shd", "fill", "FFEEDD", "ABCDEF", None),
            "table.cell.background",
        ),
        (
            "cell_border",
            "word/document.xml",
            |xml| replace_attribute(xml, "top", "sz", "4", "8", None),
            "table.cell.border",
        ),
        (
            "table_style_region",
            "word/styles.xml",
            |xml| replace_attribute(xml, "shd", "fill", "002060", "112233", Some("ReportTable")),
            "table.style.first_row.background",
        ),
        (
            "header_text",
            "word/header1.xml",
            |xml| replace_once(xml, "Default header", "Wrong header"),
            "section.header.text",
        ),
        (
            "comment_text",
            "word/comments.xml",
            |xml| replace_once(xml, "Run note", "Wrong note"),
            "comments.content",
        ),
        (
            "comment_range",
            "word/document.xml",
            extend_first_comment,
            "comment.anchored_text",
        ),
        (
            "metadata_title",
            "docProps/core.xml",
            |xml| replace_once(xml, "Parity report", "Wrong title"),
            "metadata='title'",
        ),
        (
            "page_width",
            "word/document.xml",
            |xml| replace_attribute(xml, "pgSz", "w", "15840", "16000", None),
            "section.dimensions",
        ),
        (
            "image_size",
            "word/document.xml",
            |xml| replace_attribute(xml, "extent", "cx", "914400", "900000", None),
            "image.dimensions",
        ),
    ];
    for (name, part, change, mismatch) in mutations {
        let output = temp.path(&format!("{name}.docx"));
        rewrite_part(&baseline, &output, part, change);
        verify_rejected("rich", &output, None, mismatch);
    }
}

#[test]
fn python_assertions_reject_lost_unicode_whitespace_and_run_boundaries() {
    let temp = Temp::new();
    let baseline = temp.path("text-baseline.docx");
    save_text_fixture(&baseline);
    verify("text", &baseline, None);
    let mutations: &[Mutation] = &[
        (
            "lost_unicode",
            "word/document.xml",
            |xml| replace_once(xml, "🌍", ""),
            "document.paragraphs.text",
        ),
        (
            "collapsed_spaces",
            "word/document.xml",
            |xml| {
                replace_once(
                    xml,
                    "  leading  middle  trailing  ",
                    "leading middle trailing",
                )
            },
            "document.paragraphs.text",
        ),
        (
            "lost_newline",
            "word/document.xml",
            |xml| replace_once(xml, "<w:br/>", ""),
            "document.paragraphs.text",
        ),
        (
            "lost_empty_run",
            "word/document.xml",
            |xml| replace_once(xml, "<w:r/>", ""),
            "paragraph.runs.text",
        ),
    ];
    for (name, part, change, mismatch) in mutations {
        let output = temp.path(&format!("{name}.docx"));
        rewrite_part(&baseline, &output, part, change);
        verify_rejected("text", &output, None, mismatch);
    }
}

#[test]
fn python_assertions_reject_wrong_image_payload_even_when_sizes_and_parts_match() {
    let temp = Temp::new();
    let baseline = temp.path("images-baseline.docx");
    let output = temp.path("wrong-image.docx");
    save_images_fixture(&baseline);
    verify("images", &baseline, None);
    rewrite_part(&baseline, &output, "word/document.xml", |xml| {
        let parsed = roxmltree::Document::parse(xml).unwrap();
        let identifiers: Vec<_> = parsed
            .descendants()
            .filter(|n| n.tag_name().name() == "blip")
            .map(|n| {
                n.attribute((
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
                    "embed",
                ))
                .unwrap()
            })
            .collect();
        assert_ne!(identifiers[0], identifiers[1]);
        replace_attribute(xml, "blip", "embed", identifiers[0], identifiers[1], None)
    });
    verify_rejected("images", &output, None, "image[0].content");
}

#[test]
fn python_assertions_reject_template_content_and_preservation_regressions() {
    let temp = Temp::new();
    let (source, baseline) = save_template_fixture(&temp);
    verify("template", &baseline, Some(&source));
    let mutations: &[Mutation] = &[
        (
            "missed_replacement",
            "word/document.xml",
            |xml| replace_once(xml, "Hello Alice", "Hello {{NAME}}"),
            "template.paragraphs.text",
        ),
        (
            "replaced_new_content",
            "word/document.xml",
            |xml| replace_once(xml, "Inserted {{NAME}}", "Inserted Alice"),
            "template.paragraphs.text",
        ),
        (
            "wrong_table_value",
            "word/document.xml",
            |xml| replace_once(xml, ">Alice<", ">Bob<"),
            "template.table.text",
        ),
        (
            "lost_run_format",
            "word/document.xml",
            |xml| replace_attribute(xml, "b", "val", "1", "0", None),
            "template.run.bold",
        ),
        (
            "lost_header_extension",
            "word/header1.xml",
            |xml| replace_once(xml, "Header extension", "Wrong extension"),
            "template.header.text",
        ),
    ];
    for (name, part, change, mismatch) in mutations {
        let output = temp.path(&format!("{name}.docx"));
        rewrite_part(&baseline, &output, part, change);
        verify_rejected("template", &output, Some(&source), mismatch);
    }
}
