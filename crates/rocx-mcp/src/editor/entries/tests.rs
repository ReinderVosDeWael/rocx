use super::*;
use rocx::editor::ImportedDocument;

const SAMPLE: &[u8] = include_bytes!("../../../../../examples/all_features-rust.entry-text.docx");

#[test]
fn pages_preserve_source_order_lexical_ids_and_distinct_owners() {
    let snapshot = Snapshot::open(SAMPLE).unwrap();
    let (footnotes, next) = snapshot.entry_page(0, 3).unwrap();
    assert_eq!(next, Some(3));
    assert_eq!(
        footnotes[0],
        json!({
            "kind": "footnote", "part_name": "word/footnotes.xml", "id": "-1",
            "note_type": "separator", "author": null, "date": null, "initials": null,
            "blocks": [{"index": 0, "type": "paragraph", "text": "", "has_opaque_content": true}],
        })
    );
    assert!(footnotes[1].get("anchor").is_none());
    assert_eq!(
        footnotes[2]["blocks"],
        json!([
            {"index": 0, "type": "paragraph", "text": "Edited footnote entry", "has_opaque_content": true},
            {"index": 1, "type": "opaque", "name": "w:tbl"},
            {"index": 2, "type": "paragraph", "text": "Footnote external link", "has_opaque_content": true},
        ])
    );
    let (remaining, next) = snapshot.entry_page(next.unwrap(), usize::MAX).unwrap();
    assert_eq!(next, None);
    assert_eq!(remaining[2]["kind"], "endnote");
    assert_eq!(remaining[3]["kind"], "comment");
    assert_eq!(remaining[3]["author"], "Range reviewer");
    assert_eq!(remaining[3]["initials"], "");
    for other in &remaining[2..=3] {
        assert_eq!(footnotes[2]["id"], other["id"]);
        assert_ne!(footnotes[2]["anchor"], other["anchor"]);
    }
    assert_ne!(remaining[2]["anchor"], remaining[3]["anchor"]);
    assert_eq!(
        snapshot.entry_page(2, 2).unwrap(),
        (vec![footnotes[2].clone(), remaining[0].clone()], Some(4))
    );
    assert_eq!(snapshot.entry_page(usize::MAX, 1).unwrap(), (vec![], None));
    assert!(snapshot.entry_page(0, 0).is_err());
    let blank = Snapshot::open(&Snapshot::create(vec![]).unwrap()).unwrap();
    assert_eq!(blank.entry_page(0, 10).unwrap(), (vec![], None));
}

#[test]
fn edits_match_native_bytes_and_errors_without_mutating_the_snapshot() {
    let snapshot = Snapshot::open(SAMPLE).unwrap();
    let compact = snapshot.inspect().unwrap();
    let page = snapshot.entry_page(6, 1).unwrap().0;
    let local = page[0]["anchor"].as_str().unwrap();
    let native = ImportedDocument::open(SAMPLE).unwrap();
    let target = &native.entry_stories().unwrap()[6];
    let native_edit = |edit| native.edit_entry(&target.part_name, &target.id, edit);
    let invalid = EntryEdit::RemoveParagraph(usize::MAX);
    assert_eq!(
        snapshot.edit_entry(local, invalid.clone()).unwrap_err(),
        native_edit(invalid).unwrap_err().to_string()
    );
    for forged in ["paragraph:6", "entry:0", "entry:+6", "entry:06", "entry:10"] {
        assert_eq!(
            snapshot
                .edit_entry(forged, EntryEdit::CommentInitials(None))
                .unwrap_err(),
            "unknown local entry anchor"
        );
    }
    let edit = EntryEdit::CommentInitials(Some("NEW".into()));
    let edited = snapshot.edit_entry(local, edit.clone()).unwrap();
    assert_eq!(edited, native_edit(edit).unwrap());
    assert_eq!(snapshot.inspect().unwrap(), compact);
    assert_eq!(snapshot.entry_page(6, 1).unwrap().0, page);
    let reopened = Snapshot::open(&edited).unwrap();
    assert_eq!(reopened.entry_page(6, 1).unwrap().0[0]["initials"], "NEW");
}

#[test]
fn candidate_eligibility_does_not_parse_ids_as_bounded_integers() {
    for id in [
        "  +0007 ",
        " -0000\t",
        "9223372036854775808000",
        "malformed",
        "-x",
    ] {
        assert!(is_candidate(EntryKind::Footnote, None, id), "{id:?}");
    }
    assert!(!is_candidate(
        EntryKind::Endnote,
        Some("normal"),
        " \t-9223372036854775808000"
    ));
    assert!(!is_candidate(EntryKind::Footnote, Some("separator"), "7"));
    assert!(is_candidate(EntryKind::Comment, None, "malformed"));
}
