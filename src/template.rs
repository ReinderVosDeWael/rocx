use crate::xml::{Content, Node};
use crate::*;
/// Replace spans in paragraph run text without flattening unrelated formatting.
pub(crate) fn replacements(root: &mut Node, replacements: &[(String, String)]) -> Result<()> {
    for (needle, replacement) in replacements {
        if needle.is_empty() {
            return Err(Error::Validation(
                "template replacement needle cannot be empty".into(),
            ));
        }
        crate::render::validate_text(needle)?;
        crate::render::validate_text(replacement)?;
        fn visit(n: &mut Node, needle: &str, replacement: &str) {
            if n.name == "w:p" {
                // Text tokens include tab/break elements, just as python-docx run.text does.
                fn collect(n: &Node, out: &mut Vec<String>) {
                    if n.name == "w:t" {
                        out.push(
                            n.children
                                .iter()
                                .filter_map(|c| {
                                    if let Content::Text(t) = c {
                                        Some(t.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        );
                    } else if n.name == "w:tab" {
                        out.push("\t".into());
                    } else if n.name == "w:br" || n.name == "w:cr" {
                        out.push("\n".into());
                    } else {
                        for c in n.elements() {
                            collect(c, out);
                        }
                    }
                }
                let mut tokens = Vec::new();
                collect(n, &mut tokens);
                let joined = tokens.concat();
                let matches: Vec<_> = joined.match_indices(needle).map(|(i, _)| i).collect();
                for start in matches.into_iter().rev() {
                    let end = start + needle.len();
                    let mut offset = 0;
                    let mut inserted = false;
                    for token in &mut tokens {
                        let oldlen = token.len();
                        let tstart = offset;
                        let tend = offset + oldlen;
                        if start < tend && end > tstart {
                            let a = start.saturating_sub(tstart);
                            let b = (end - tstart).min(oldlen);
                            token.replace_range(
                                a..b,
                                if inserted {
                                    ""
                                } else {
                                    inserted = true;
                                    replacement
                                },
                            );
                        }
                        offset = tend;
                    }
                }
                fn write(n: &mut Node, tokens: &[String], index: &mut usize) {
                    let old = std::mem::take(&mut n.children);
                    for child in old {
                        match child {
                            Content::Element(child)
                                if ["w:t", "w:tab", "w:br", "w:cr"]
                                    .contains(&child.name.as_str()) =>
                            {
                                let text = &tokens[*index];
                                *index += 1;
                                let replacement = crate::render::text_run(text);
                                n.children.extend(replacement.children);
                            }
                            Content::Element(mut child) => {
                                write(&mut child, tokens, index);
                                n.push(child);
                            }
                            other => n.children.push(other),
                        }
                    }
                }
                if joined.contains(needle) {
                    write(n, &tokens, &mut 0);
                }
            } else {
                for c in n.elements_mut() {
                    visit(c, needle, replacement);
                }
            }
        }
        visit(root, needle, replacement);
    }
    Ok(())
}
