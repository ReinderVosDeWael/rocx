//! Text/whitespace encoding: P1 §§17.3.3 and 17.3.3.31.
use crate::xml::Node;
pub(crate) fn text_run(text: &str) -> Node {
    run_with_text_tag(text, "w:t")
}

pub(super) fn deleted_text_run(text: &str) -> Node {
    run_with_text_tag(text, "w:delText")
}

fn run_with_text_tag(text: &str, tag: &str) -> Node {
    let mut run = Node::new("w:r");
    let mut segment = String::new();
    for c in text.chars() {
        if matches!(c, '\t' | '\r' | '\n') {
            if !segment.is_empty() {
                run.push(
                    Node::new(tag)
                        .attr("xml:space", "preserve")
                        .text(std::mem::take(&mut segment)),
                );
            }
            run.push(Node::new(if c == '\t' { "w:tab" } else { "w:br" }));
        } else {
            segment.push(c);
        }
    }
    if !segment.is_empty() {
        run.push(Node::new(tag).attr("xml:space", "preserve").text(segment));
    }
    run
}
