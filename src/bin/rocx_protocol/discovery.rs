use super::*;

pub(super) fn revision_metadata(r: &ImportedPropertyRevision) -> Value {
    json!({"kind":r.kind,"id":r.id,"author":r.author,"date":r.date})
}

pub(super) fn inlines(nodes: &[ImportedInlineNode]) -> Vec<Value> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(i, node)| {
            Some(match node {
                ImportedInlineNode::Run(r) => {
                    let text = match r.items.as_slice() {
                        [ImportedInline::Text(s)] => Some(s),
                        _ => None,
                    };
                    let mut value = json!({"kind":"run","inline_index":i,"text":text});
                    if let Some(r) = &r.property_revision {
                        value["property_revision"] = revision_metadata(r);
                    }
                    if let Some(d) = &r.drawing {
                        value["drawing"] = json!({"kind":d.kind,"description":d.description,
                            "placement":d.placement,"relationship_id":d.relationship_id});
                    }
                    if r.items.iter().any(|n| matches!(n, ImportedInline::Opaque(name) if name == "w:fldChar")) {
                        value["has_field_marker"] = json!(true);
                    }
                    value
                }
                ImportedInlineNode::Hyperlink(_) => json!({"kind":"hyperlink","inline_index":i}),
                ImportedInlineNode::Fraction { numerator, denominator, .. } =>
                    json!({"kind":"fraction","inline_index":i,"numerator":numerator,"denominator":denominator}),
                ImportedInlineNode::Opaque(name) if name == "w:fldSimple" =>
                    json!({"kind":"field","inline_index":i}),
                ImportedInlineNode::Revision(r) => json!({
                    "kind":"revision","inline_index":i,"revision_kind":r.kind,
                    "author":r.author,"id":r.id,"date":r.date
                }),
                _ => return None,
            })
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
    add(result, "sections", doc.sections().map(|sections| Value::Array(sections.iter().enumerate().map(|(i,s)| json!({
        "target":i,"layout":structure::layout_view(&s.layout),
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
                "part":s.part_name,"inherited":s.inherited,"block_count":s.blocks.len(),
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
                            "id":s.id,"kind":s.kind.as_str(),"name":s.name,"based_on":s.based_on,
                            "next_style":s.next_style,"linked_style":s.linked_style,"property_xml":s.property_xml
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
        "document_defaults",
        doc.document_defaults().map(|v| {
            json!(
                v.into_iter()
                    .map(|(k, v)| (k.key(), DefaultScalar::from(v)))
                    .collect::<std::collections::BTreeMap<_, _>>()
            )
        }),
    );
    add(
        result,
        "numbering_catalog",
        doc.numbering_catalog().map(|catalog| json!(catalog)),
    );
    add(result,"numbering_instances",doc.numbering_instances().map(|v|v.into_iter().map(|i|json!({
        "num_id":i.num_id,"abstract_id":i.abstract_id,"replacement_levels":i.replacement_levels,
        "start_overrides":i.start_overrides.iter().map(|o|json!({"level":o.level,"start":o.start})).collect::<Vec<_>>()
    })).collect()));
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
               "tag":c.tag,"alias":c.alias,"lock":c.lock,"bound":c.bound,
               "kind":c.kind,"selected":c.selected,
               "choices":c.choices.iter().map(|c|json!({"display_text":c.display_text,"value":c.value})).collect::<Vec<_>>()})
        })
        .collect();
    result["control_sites"] = doc
        .control_sites()
        .iter()
        .enumerate()
        .map(|(i, s)| json!({"target":i,"context":s.context.as_str(),"text":s.text}))
        .collect();
}
