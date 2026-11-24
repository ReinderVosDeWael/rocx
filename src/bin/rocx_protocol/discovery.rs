use super::*;

pub(super) fn inlines(nodes: &[ImportedInlineNode]) -> Vec<Value> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(i, node)| {
            let value = match node {
                ImportedInlineNode::Run(r) => {
                    let text = match r.items.as_slice() {
                        [ImportedInline::Text(s)] => Some(s),
                        _ => None,
                    };
                    json!({"kind":"run","inline_index":i,"text":text})
                }
                ImportedInlineNode::Hyperlink(_) => json!({"kind":"hyperlink","inline_index":i}),
                ImportedInlineNode::Revision(r) => json!({
                    "kind":"revision","inline_index":i,"revision_kind":r.kind,
                    "author":r.author,"id":r.id,"date":r.date
                }),
                _ => return None,
            };
            Some(value)
        })
        .collect()
}

fn blocks<'a>(
    count: usize,
    nodes: impl Fn(usize) -> Option<&'a [ImportedInlineNode]>,
) -> Vec<Value> {
    (0..count)
        .filter_map(|i| Some(json!({"block_index":i,"inlines":inlines(nodes(i)?)})))
        .collect()
}

pub(super) fn extend(doc: &ImportedDocument, result: &mut Value) {
    fn add(obj: &mut Value, key: &str, value: rocx::Result<Value>) {
        match value {
            Ok(v) => obj[key] = v,
            Err(e) => obj[format!("{key}_error")] = json!(e.to_string()),
        }
    }
    add(result, "sections", doc.sections().map(|sections| Value::Array(sections.iter().map(|s| json!({
        "body_start":s.body_start,"body_end":s.body_end,"section_type_xml":s.section_type_xml,
        "flags":s.flags.iter().map(|(f,v)|(f.python_name(),v)).collect::<std::collections::BTreeMap<_,_>>()
    })).collect())));
    match doc.stories() {
        Ok(stories) => {
            result["headers"] = stories
                .iter()
                .filter(|s| {
                    s.kind == SectionStoryKind::Header
                        && s.variant != HeaderFooterType::Even
                        && s.anchor.is_some()
                })
                .map(|s| {
                    json!({
                        "target":{"section_index":s.section_index,"variant":variant(s.variant)},
                        "part":s.part_name,"inherited":s.inherited
                    })
                })
                .collect();
            result["stories"] = stories.iter().map(|s|json!({
                "target":{"kind":s.kind.as_str(),"section_index":s.section_index,"variant":variant(s.variant)},
                "part":s.part_name,"inherited":s.inherited,
                "blocks":blocks(s.blocks.len(),|i|s.inline_structure(i))
            })).collect();
        }
        Err(e) => {
            result["headers_error"] = json!(e.to_string());
            result["stories_error"] = json!(e.to_string());
        }
    }
    add(
        result,
        "entries",
        doc.entry_stories().map(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    json!({
                        "entry_index":i,"kind":e.kind.as_str(),"part":e.part_name,
                        "id":e.id,"note_type":e.note_type,
                        "blocks":blocks(e.blocks().len(),|i|e.inline_structure(i))
                    })
                })
                .collect()
        }),
    );
    add(
        result,
        "styles",
        doc.styles().map(|catalog| {
            json!(catalog.map(|s| {
                let definitions: Vec<_> = s
                    .styles
                    .iter()
                    .map(|s| {
                        json!({
                            "id":s.id,"kind":s.kind.as_str(),"name":s.name,"based_on":s.based_on
                        })
                    })
                    .collect();
                json!({"part":s.part_name,"definitions":definitions})
            }))
        }),
    );
    add(result, "core_title", doc.core_title().map(|v| json!(v)));
    add(result, "core_subject", doc.core_subject().map(|v| json!(v)));
    add(
        result,
        "default_alignment",
        doc.default_alignment().map(|v| {
            json!(v.map(|a| match a {
                Alignment::Left => "left",
                Alignment::Center => "center",
                Alignment::Right => "right",
                Alignment::Justify => "justify",
                Alignment::Distribute => "distribute",
                Alignment::JustifyMedium => "justify-medium",
                Alignment::JustifyHigh => "justify-high",
                Alignment::JustifyLow => "justify-low",
                Alignment::ThaiJustify => "thai-justify",
            }))
        }),
    );
    add(
        result,
        "abstract_starts",
        doc.abstract_starts().map(|v| {
            v.iter()
                .map(|n| json!({"abstract_id":n.abstract_id,"level":n.level,"value":n.start}))
                .collect()
        }),
    );
    result["controls"] = doc
        .controls()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({"target":i,"context":c.context.as_str(),"id":c.id,
               "tag":c.tag,"alias":c.alias,"lock":c.lock,"bound":c.bound})
        })
        .collect();
}
