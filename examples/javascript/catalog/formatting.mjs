import { EDIT_LABEL, br, heading, lineHeight, p, para, run, tab, tabStop, text } from "./blocks.mjs";

export function formattingBlocks() {
  const blocks = [
    heading("01 Paragraphs and headings"),
    p(EDIT_LABEL, { alignment: "left" }),
  ];
  for (let level = 0; level < 10; level++) {
    blocks.push(p(`This is a Heading ${level}${level === 0 ? " (heading=0: no heading style)" : ""}`, { heading: level }));
  }
  for (const alignment of ["left", "center", "right", "justify", "distribute", "justifyMedium", "justifyHigh", "justifyLow", "thaiJustify"]) {
    blocks.push(p(`Paragraph alignment=${alignment}: The quick brown fox jumps over the lazy dog. Ελληνικά العربية 日本語.`, { alignment }));
  }
  for (const [kind, value, name] of [["multiplier", 1.5, "multiple 1.5"], ["exact", 20, "exact 20 pt"], ["minimum", 18, "at_least 18 pt"]]) {
    blocks.push(para([
      text(`Line height=${name}; before=6 pt; after=8 pt`), br(), text("Second line makes the line-height request visible."),
    ], { lineHeight: lineHeight(kind, value), spacingBefore: 6, spacingAfter: 8 }));
  }
  blocks.push(
    p("Legacy line_spacing=1.25; left=18 pt, right=12 pt, hanging=6 pt; keep lines/next and widow control enabled.",
      { lineSpacing: 1.25, leftIndent: 18, rightIndent: 12, firstLineIndent: -6, keepTogether: true, keepWithNext: true, widowControl: true }),
    p("First-line indent=18 pt; keep lines/next, page-break-before and widow control explicitly false.",
      { firstLineIndent: 18, keepTogether: false, keepWithNext: false, pageBreakBefore: false, widowControl: false, bidi: false }),
  );
  for (const alignment of ["left", "center", "right", "decimal", "bar", "clear", "start", "end"]) {
    blocks.push(para([text(`Tab ${alignment}`), tab, text("123.45")], { tabStops: [tabStop(216, alignment)] }));
  }
  for (const leader of ["none", "dot", "hyphen", "underscore", "heavy", "middleDot"]) {
    blocks.push(para([text(`Tab leader=${leader === "middleDot" ? "middle_dot" : leader}`), tab, text("at 288 pt")],
      { tabStops: [tabStop(288, "right", leader), tabStop(-12, "clear")] }));
  }
  blocks.push(
    heading("02 Run formatting, colors, scripts and whitespace"),
    p("Formatting declarations in Rust, Python and JavaScript: omitted, enabled and explicitly disabled bold."),
    para([
      text("Builder bold unspecified; "), text("Builder bold enabled; ", { bold: true }),
      text("Builder bold disabled, italic enabled.", { bold: false, italic: true }),
    ], { spacingAfter: 6, keepWithNext: false }),
  );
  for (const [name, format] of [
    ["Bold=true", { bold: true }], ["Italic=true", { italic: true }], ["Strike=true", { strike: true }],
    ["ALL CAPS=true", { allCaps: true }], ["Small caps=true", { smallCaps: true }],
    ["Superscript=true", { superscript: true }], ["Subscript=true", { subscript: true }],
    ["Arial 16 pt, RGB 803020", { font: "Arial", size: 16, color: [128, 48, 32] }],
    ["Automatic color", { color: "auto" }],
  ]) blocks.push(run(name, format));
  for (const underline of ["none", "single", "double", "dotted", "dash", "wave", "words"]) {
    blocks.push(run(`Underline=${underline}: words with spaces`, { underline }));
  }
  for (const name of ["dark1", "light1", "dark2", "light2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hyperlink", "followedHyperlink", "background1", "text1", "background2", "text2"]) {
    blocks.push(para([
      text(`Theme ${name} (light colors may be invisible on white): `),
      text("colored text", { color: { kind: "theme", name, tint: 32, shade: 224 } }),
    ]));
  }
  return blocks.concat(
    run("Explicit false overrides: bold, italic, strike, caps, small caps, superscript, subscript and underline disabled.",
      { bold: false, italic: false, strike: false, allCaps: false, smallCaps: false, superscript: false, subscript: false, underline: "none" }),
    para([
      text("  Preserved leading and trailing spaces  "), text(""),
      text("Unicode: café Ελληνικά 日本語 العربية; literal tab\tand newline\nend."),
    ]),
    para([text("Run language/script: en-US / ja-JP / ar-SA; العربية 日本語", {
      language: { latin: "en-US", eastAsia: "ja-JP", bidi: "ar-SA" }, boldComplexScript: true, italicComplexScript: false,
      sizeComplexScript: 14, rtl: true, complexScript: true, fontEastAsia: "Yu Mincho", fontComplexScript: "Arial",
    })], { bidi: true }),
    run("Run RTL=false, complex_script=false, bCs=false, iCs=true; independent from paragraph direction.",
      { rtl: false, complexScript: false, boldComplexScript: false, italicComplexScript: true }),
    heading("03 Styles, inheritance, links and direct overrides"),
    para([
      text("Paragraph style Catalog Child (forward base/next/link); "),
      text("character style Catalog Character with bold=false override", { style: "Catalog Character", bold: false }),
    ], { style: "Catalog Child" }),
    para([text("Updated template paragraph/character styles: original bold retained, italic added.",
      { style: "Catalog Existing Character" })], { style: "Catalog Existing" }),
  );
}

export function formattingStyles() {
  return [
    { kind: "paragraph", name: "Catalog Child", baseStyle: "Catalog Base", nextParagraphStyle: "Normal",
      linkedStyle: "Catalog Character", lineHeight: lineHeight("minimum", 16), tabStops: [tabStop(72)], bidi: false },
    { kind: "character", name: "Catalog Character", baseStyle: "Catalog Character Base", linkedStyle: "Catalog Child",
      format: { bold: true, italic: true, color: [48, 64, 144] } },
    { kind: "paragraph", name: "Catalog Base", baseStyle: "Normal", font: "Arial", fontSize: 12, bold: true,
      italic: false, underline: false, color: [24, 64, 96], alignment: "left", spacingBefore: 6, spacingAfter: 6,
      leftIndent: 0.25, rightIndent: 0.125, firstLineIndent: -0.125, keepTogether: true, keepWithNext: false,
      pageBreakBefore: false, widowControl: true, language: { latin: "en-US" },
      boldComplexScript: false, italicComplexScript: false, sizeComplexScript: 12, rtl: false,
      complexScript: false, fontEastAsia: "Yu Mincho", fontComplexScript: "Arial" },
    { kind: "character", name: "Catalog Character Base", format: { font: "Calibri", size: 12 } },
    { kind: "paragraph", name: "Catalog Existing", italic: true },
    { kind: "character", name: "Catalog Existing Character", format: { italic: true } },
  ];
}
