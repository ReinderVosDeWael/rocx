use std::io::{Cursor, Write};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{Error, ResolvedDocument, Result};

const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const CONTENT_TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
const RELATIONSHIPS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

pub(crate) fn validate_text(text: &str) -> Result<()> {
    for (byte_offset, character) in text.char_indices() {
        if !matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
        {
            return Err(Error::InvalidXmlCharacter {
                character,
                byte_offset,
            });
        }
    }
    Ok(())
}

fn text_run(xml: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    xml.push_str("<w:r><w:t xml:space=\"preserve\">");
    for character in text.chars() {
        match character {
            '&' => xml.push_str("&amp;"),
            '<' => xml.push_str("&lt;"),
            '>' => xml.push_str("&gt;"),
            '\t' => xml.push_str("</w:t><w:tab/><w:t xml:space=\"preserve\">"),
            // python-docx maps each CR and each LF to a break, including CRLF pairs.
            '\n' | '\r' => xml.push_str("</w:t><w:br/><w:t xml:space=\"preserve\">"),
            _ => xml.push(character),
        }
    }
    xml.push_str("</w:t></w:r>");
}

fn document_xml(document: &ResolvedDocument) -> String {
    let mut xml = format!(
        "{XML}<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>"
    );
    for (index, section) in document.sections.iter().enumerate() {
        for paragraph in &section.paragraphs {
            xml.push_str("<w:p>");
            text_run(&mut xml, &paragraph.text);
            xml.push_str("</w:p>");
        }
        if index + 1 < document.sections.len() {
            // The default section start is nextPage. The break belongs to the
            // previous section's terminal paragraph, as in python-docx.
            xml.push_str("<w:p><w:pPr><w:sectPr/></w:pPr></w:p>");
        }
    }
    // A final body-level sectPr is required even for an empty document.
    xml.push_str("<w:sectPr/></w:body></w:document>");
    xml
}

pub(crate) fn render(document: &ResolvedDocument) -> Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    // Stored ZIP entries are valid OPC parts and avoid compression dependencies.
    // Fixed timestamps keep repeated rendering deterministic.
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    for (path, xml) in [
        ("[Content_Types].xml", format!("{XML}{CONTENT_TYPES}")),
        ("_rels/.rels", format!("{XML}{RELATIONSHIPS}")),
        ("word/document.xml", document_xml(document)),
    ] {
        zip.start_file(path, options)?;
        zip.write_all(xml.as_bytes())?;
    }
    Ok(zip.finish()?.into_inner())
}
