//! ECMA-376 Part 1 (2016): P1 §17.4, table grid and cells.
use crate::ooxml::Compiler;
use crate::xml::{Node, val};
use crate::{ResolvedTable, Result, TableLayout, Twips};
impl Compiler<'_> {
    pub(crate) fn table(
        &mut self,
        t: &ResolvedTable,
        context: &crate::ooxml::PartContext,
    ) -> Result<Node> {
        let width = context.width.get();
        let widths: Vec<i64> = if t.layout != Some(TableLayout::Autofit) {
            t.column_widths
                .as_ref()
                .map(|w| w.iter().map(|n| *n as i64).collect())
                .unwrap_or_else(|| {
                    vec![
                        if t.columns == 0 {
                            0
                        } else {
                            width / t.columns as i64
                        };
                        t.columns
                    ]
                })
        } else {
            vec![
                if t.columns == 0 {
                    0
                } else {
                    width / t.columns as i64
                };
                t.columns
            ]
        };
        let mut props = Node::new("w:tblPr");
        if let Some(style) = &t.style {
            props.push(val(
                "tblStyle",
                self.styles
                    .get(style)
                    .map_err(|error| error.at(context.source.as_str()))?,
            ));
        }
        props.push(
            Node::new("w:tblW")
                .attr(
                    "w:w",
                    if t.column_widths.is_some() && t.layout != Some(TableLayout::Autofit) {
                        widths.iter().sum()
                    } else {
                        0
                    },
                )
                .attr(
                    "w:type",
                    if t.column_widths.is_some() && t.layout != Some(TableLayout::Autofit) {
                        "dxa"
                    } else {
                        "auto"
                    },
                ),
        );
        if let Some(alignment) = t.alignment {
            props.push(val("jc", alignment.xml()));
        }
        if t.layout.is_some() || t.column_widths.is_some() {
            props.push(
                Node::new("w:tblLayout")
                    .attr("w:type", t.layout.unwrap_or(TableLayout::Fixed).xml()),
            );
        }
        if !t.borders.is_empty() {
            let mut b = Node::new("w:tblBorders");
            for border in &t.borders {
                b.push(
                    Node::new(format!("w:{}", border.side.xml()))
                        .attr("w:val", border.val.xml())
                        .attr("w:sz", border.sz)
                        .attr("w:color", border.hex_color()),
                );
            }
            props.push(b);
        }
        let look = t.look.unwrap_or_default();
        let mut node = Node::new("w:tblLook");
        let mut mask = 0_u16;
        for (name, enabled, bit) in [
            ("firstColumn", look.first_column.unwrap_or(true), 0x80),
            ("firstRow", look.first_row.unwrap_or(true), 0x20),
            ("lastColumn", look.last_column.unwrap_or(false), 0x100),
            ("lastRow", look.last_row.unwrap_or(false), 0x40),
            ("noHBand", !look.horizontal_banding.unwrap_or(true), 0x200),
            ("noVBand", !look.vertical_banding.unwrap_or(false), 0x400),
        ] {
            node.set(format!("w:{name}"), u8::from(enabled));
            if enabled {
                mask |= bit;
            }
        }
        node.set("w:val", format!("{mask:04X}"));
        props.push(node);
        let mut grid = Node::new("w:tblGrid");
        for w in &widths {
            grid.push(Node::new("w:gridCol").attr("w:w", w));
        }
        let mut node = Node::new("w:tbl").with(props).with(grid);
        for row in &t.rows {
            let mut tr = Node::new("w:tr");
            row.properties.validate()?;
            let mut row_properties = Node::new("w:trPr");
            for (name, value) in [
                ("cantSplit", row.properties.cant_split),
                ("tblHeader", row.properties.repeat_header),
            ] {
                if let Some(value) = value {
                    row_properties.push(val(name, if value { "1" } else { "0" }));
                }
            }
            if let Some(height) = row.properties.height {
                let (points, rule) = match height {
                    crate::RowHeight::Exact(points) => (points, "exact"),
                    crate::RowHeight::AtLeast(points) => (points, "atLeast"),
                };
                row_properties.push(
                    Node::new("w:trHeight")
                        .attr("w:val", crate::Points::new(points)?.twips()?.get())
                        .attr("w:hRule", rule),
                );
            }
            if !row_properties.children.is_empty() {
                tr.push(row_properties);
            }
            let mut column = 0;
            for cell in &row.cells {
                let span = cell.grid_span as usize;
                let cw = widths[column..column + span].iter().sum::<i64>();
                column += span;
                let mut pr = Node::new("w:tcPr")
                    .with(Node::new("w:tcW").attr("w:w", cw).attr("w:type", "dxa"));
                if span > 1 {
                    pr.push(val("gridSpan", span));
                }
                if let Some(merge) = cell.vmerge {
                    pr.push(val("vMerge", merge.xml()));
                }
                if let Some(color) = cell.background_color {
                    pr.push(crate::ooxml::wordprocessing::format::shading(color));
                }
                if !cell.borders.is_empty() {
                    let mut b = Node::new("w:tcBorders");
                    for border in &cell.borders {
                        b.push(
                            Node::new(format!("w:{}", border.side.xml()))
                                .attr("w:val", border.val.xml())
                                .attr("w:sz", border.sz)
                                .attr("w:color", border.hex_color()),
                        );
                    }
                    pr.push(b);
                }
                if let Some(v) = cell.vertical_alignment {
                    pr.push(val("vAlign", v.xml()));
                }
                if let Some(margins) = &cell.margins {
                    margins.validate()?;
                    let mut node = Node::new("w:tcMar");
                    for (name, points) in [
                        ("top", margins.top),
                        ("start", margins.start),
                        ("bottom", margins.bottom),
                        ("end", margins.end),
                    ] {
                        if let Some(points) = points {
                            node.push(
                                Node::new(format!("w:{name}"))
                                    .attr("w:w", crate::Points::new(points)?.twips()?.get())
                                    .attr("w:type", "dxa"),
                            );
                        }
                    }
                    if !node.children.is_empty() {
                        pr.push(node);
                    }
                }
                if let Some(direction) = cell.text_direction {
                    pr.push(val("textDirection", direction.xml()));
                }
                let mut tc = Node::new("w:tc").with(pr);
                let cell_context = crate::ooxml::PartContext {
                    story: context.story,
                    kind: context.kind,
                    source: context.source.clone(),
                    content: context.content,
                    width: Twips::new(if t.columns == 0 {
                        cw
                    } else {
                        (width as f64 / t.columns as f64).round_ties_even() as i64
                    }),
                };
                let blocks = self.blocks(&cell.blocks, &cell_context)?;
                if blocks.first().is_none_or(|n| n.name != "w:p") {
                    tc.push(Node::new("w:p"));
                }
                for block in blocks {
                    let table = block.name == "w:tbl";
                    tc.push(block);
                    if table {
                        tc.push(Node::new("w:p"));
                    }
                }
                if tc.elements().last().is_none_or(|n| n.name != "w:p") {
                    tc.push(Node::new("w:p"));
                }
                tr.push(self.table_content_control(tc, cell.content_control.as_ref())?);
            }
            while column < t.columns {
                tr.push(
                    Node::new("w:tc")
                        .with(
                            Node::new("w:tcPr").with(
                                Node::new("w:tcW")
                                    .attr("w:w", widths[column])
                                    .attr("w:type", "dxa"),
                            ),
                        )
                        .with(Node::new("w:p")),
                );
                column += 1;
            }
            node.push(self.table_content_control(tr, row.properties.content_control.as_ref())?);
        }
        Ok(node)
    }
}
