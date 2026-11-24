//! Edit the complete cached text of a supported plain-text control by its tag.
//! P1 (2016) §17.5.2 and Annex A.1 CT_SdtText. Edit restrictions are library policy.
use crate::xml::{Content, Node, W, validate_text};
use crate::{Diagnostic, Error, Result, Rule};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct ContentControlEdits<'a> {
    replacements: &'a BTreeMap<String, String>,
    matched: BTreeSet<&'a str>,
}

fn unsupported(message: impl Into<String>) -> Error {
    Diagnostic::new(Rule::UnsupportedEdit, message).into()
}

impl<'a> ContentControlEdits<'a> {
    pub(crate) fn new(replacements: &'a BTreeMap<String, String>) -> Result<Self> {
        for (tag, text) in replacements {
            if tag.is_empty() {
                return Err(unsupported(
                    "content control replacement tags cannot be empty",
                ));
            }
            validate_text(tag)?;
            validate_text(text)?;
        }
        Ok(Self {
            replacements,
            matched: BTreeSet::new(),
        })
    }

    pub(crate) fn apply(&mut self, story: &mut Node) -> Result<bool> {
        let mut dirty = false;
        if !self.replacements.is_empty() {
            self.visit(story, false, &mut 0, &mut dirty)?;
        }
        Ok(dirty)
    }

    pub(crate) fn finish(self) -> Result<()> {
        if let Some(tag) = self
            .replacements
            .keys()
            .find(|tag| !self.matched.contains(tag.as_str()))
        {
            return Err(unsupported(format!(
                "content control tag {tag:?} was not found in the selected stories"
            )));
        }
        Ok(())
    }

    fn visit(
        &mut self,
        node: &mut Node,
        protected: bool,
        field_depth: &mut usize,
        dirty: &mut bool,
    ) -> Result<()> {
        if node.name.namespace() != Some(W) {
            return Ok(());
        }
        let mut local_depth = 0;
        let field_depth = if matches!(node.name.as_str(), "w:comment" | "w:footnote" | "w:endnote")
        {
            &mut local_depth
        } else {
            field_depth
        };
        if node.name == "w:fldChar" {
            match node.get("w:fldCharType") {
                Some("begin") => *field_depth += 1,
                Some("end") => *field_depth = field_depth.saturating_sub(1),
                _ => (),
            }
            return Ok(());
        }
        if node.name == "w:sdt"
            && let Some(tag) = node
                .child("w:sdtPr")
                .and_then(|props| props.child("w:tag"))
                .and_then(|tag| tag.get("w:val"))
            && let Some((tag, replacement)) = self.replacements.get_key_value(tag)
        {
            if !self.matched.insert(tag) {
                return Err(unsupported(format!(
                    "content control tag {tag:?} matches more than one control"
                )));
            }
            if protected || *field_depth != 0 {
                return Err(unsupported(format!(
                    "content control tag {tag:?} is inside a protected structure"
                )));
            }
            replace_plain_text(node, replacement)
                .map_err(|error| error.at(format!("content control tag {tag:?}")))?;
            *dirty = true;
        }
        let protected = protected
            || matches!(
                node.name.as_str(),
                "w:sdt"
                    | "w:fldSimple"
                    | "w:hyperlink"
                    | "w:ins"
                    | "w:del"
                    | "w:moveFrom"
                    | "w:moveTo"
            );
        for child in node.elements_mut() {
            self.visit(child, protected, field_depth, dirty)?;
        }
        Ok(())
    }
}

fn replace_plain_text(control: &mut Node, replacement: &str) -> Result<()> {
    fn contains_binding(node: &Node) -> bool {
        node.name.local() == "dataBinding" || node.elements().any(contains_binding)
    }
    let properties = control
        .child("w:sdtPr")
        .ok_or_else(|| unsupported("control has no properties"))?;
    let text = properties
        .child("w:text")
        .ok_or_else(|| unsupported("only plain-text controls support tag replacements"))?;
    if control
        .elements()
        .filter(|node| node.name == "w:sdtPr")
        .any(contains_binding)
        || properties.child("w:showingPlcHdr").is_some()
        || matches!(
            properties
                .child("w:lock")
                .and_then(|lock| lock.get("w:val")),
            Some("contentLocked" | "sdtContentLocked")
        )
    {
        return Err(unsupported(
            "locked, bound and placeholder controls cannot be replaced",
        ));
    }
    let multiple_lines = matches!(text.get("w:multiLine"), Some("1" | "true" | "on"));
    if replacement.contains('\t') || (!multiple_lines && replacement.contains(['\r', '\n'])) {
        return Err(unsupported(
            "plain-text control replacements cannot contain tabs, and line breaks require multiLine",
        ));
    }

    let content = control
        .child_mut("w:sdtContent")
        .ok_or_else(|| unsupported("control has no cached content"))?;
    let mut run_owner = content;
    if run_owner.elements().any(|node| node.name == "w:p") {
        if run_owner.elements().count() != 1 {
            return Err(unsupported("plain-text control must contain one paragraph"));
        }
        run_owner = run_owner.child_mut("w:p").unwrap();
    }
    let mut runs = run_owner
        .elements_mut()
        .filter(|child| child.name != "w:pPr");
    let run = runs
        .next()
        .ok_or_else(|| unsupported("plain-text control must contain one existing run"))?;
    if run.name != "w:r"
        || runs.next().is_some()
        || run.elements().any(|child| {
            !matches!(child.name.as_str(), "w:rPr" | "w:t" | "w:br" | "w:cr")
                || (child.name == "w:br"
                    && (child.get("w:clear").is_some()
                        || child
                            .get("w:type")
                            .is_some_and(|kind| kind != "textWrapping")))
        })
    {
        return Err(unsupported(
            "plain-text control must contain one editable text run",
        ));
    }
    run.children.retain(|child| match child {
        Content::Element(node) => node.name == "w:rPr",
        _ => true,
    });
    run.children
        .extend(crate::ooxml::wordprocessing::text::text_run(replacement).children);
    Ok(())
}
