//! Fill an independent template with a small synthetic report.
use rocx::application::{DocumentTemplate, RenderOptions};
use rocx::template::EditScope;
use rocx::{
    Document, Paragraph, Result, RunFormat, Section, Table, TableCell, TableCellProperties,
    TableRow, TextRun,
};
use std::path::Path;

const TEMPLATE: &str = "docs/website/branded_report_template.docx";
const DATA: &str = include_str!("branded_report_data.tsv");

fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/branded_report_output.docx".into());
    if Path::new(&output).exists()
        && std::fs::canonicalize(&output)? == std::fs::canonicalize(TEMPLATE)?
    {
        return Err(std::io::Error::other("output must differ from template").into());
    }
    futures::executor::block_on(async {
        let rows = DATA
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let cells = line
                    .split('\t')
                    .map(|text| {
                        let mut cell = TableCell::new(vec![
                            Paragraph::from_children(vec![
                                TextRun::new(text)
                                    .with_format(RunFormat {
                                        bold: Some(index == 0),
                                        ..Default::default()
                                    })
                                    .into(),
                            ])
                            .into(),
                        ]);
                        if index == 0 {
                            cell.properties = TableCellProperties {
                                background_color: Some([220, 234, 245]),
                                ..Default::default()
                            }
                            .into();
                        }
                        cell.into()
                    })
                    .collect();
                TableRow::new(cells).into()
            })
            .collect();
        let document = Document::new(vec![Section::new(vec![Table::new(rows).into()]).into()]);
        let mut template = DocumentTemplate::new(TEMPLATE);
        template.edits.scope = EditScope::Body;
        template.edits.paragraph_index = Some(1);
        template
            .edits
            .replacements
            .push(("{{title}}".into(), "Quarterly service report".into()));
        let options = RenderOptions {
            timestamp: Some("2026-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        document
            .resolve()
            .await?
            .save_with_template_and_options(output, &template, &options)
    })
}
