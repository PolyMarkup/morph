use morph::format::Format;

fn convert(input: &str, from: Format, to: Format) -> String {
    morph::convert(input, from, to).expect("conversion failed")
}

fn parse_and_check_spans(input: &str, format: Format, expected_spans: &[(u32, u32)]) {
    let doc = morph::parse(input, format).expect("parse failed");
    let mut spans = Vec::new();
    for block in &doc.children {
        if let morph::ast::Block::Table { headers, rows, .. } = block {
            for cell in headers {
                spans.push((cell.colspan, cell.rowspan));
            }
            for row in rows {
                for cell in row {
                    spans.push((cell.colspan, cell.rowspan));
                }
            }
        }
    }
    assert_eq!(
        spans, expected_spans,
        "\nInput:\n{}\nExpected spans: {:?}\nActual spans: {:?}",
        input, expected_spans, spans
    );
}

// ===== Typst parsing =====

#[test]
fn typst_parse_colspan() {
    let input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    parse_and_check_spans(
        input,
        Format::Typst,
        &[(1, 1), (1, 1), (1, 1), (2, 1), (1, 1)],
    );
}

#[test]
fn typst_parse_rowspan() {
    let input = r#"#table(
  columns: 2,
  [A], [B],
  table.cell(rowspan: 2)[Tall], [C],
  [D],
)"#;
    parse_and_check_spans(
        input,
        Format::Typst,
        &[(1, 1), (1, 1), (1, 2), (1, 1), (1, 1)],
    );
}

#[test]
fn typst_parse_both_spans() {
    let input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2, rowspan: 2)[Big], [D],
  [E],
)"#;
    parse_and_check_spans(
        input,
        Format::Typst,
        &[(1, 1), (1, 1), (1, 1), (2, 2), (1, 1), (1, 1)],
    );
}

#[test]
fn typst_parse_no_spans() {
    let input = r#"#table(
  columns: 2,
  [A], [B],
  [C], [D],
)"#;
    parse_and_check_spans(input, Format::Typst, &[(1, 1), (1, 1), (1, 1), (1, 1)]);
}

// ===== Typst emitter =====

#[test]
fn typst_emit_colspan() {
    let input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    let output = convert(input, Format::Typst, Format::Typst);
    assert!(
        output.contains("table.cell(colspan: 2)[Wide]"),
        "Expected table.cell(colspan: 2) in output:\n{}",
        output
    );
}

#[test]
fn typst_emit_rowspan() {
    let input = r#"#table(
  columns: 2,
  [A], [B],
  table.cell(rowspan: 2)[Tall], [C],
  [D],
)"#;
    let output = convert(input, Format::Typst, Format::Typst);
    assert!(
        output.contains("table.cell(rowspan: 2)[Tall]"),
        "Expected table.cell(rowspan: 2) in output:\n{}",
        output
    );
}

// ===== AsciiDoc parsing =====

#[test]
fn asciidoc_parse_colspan() {
    let input = "|===\n|A |B |C\n\n2+|Wide |D\n|===\n";
    parse_and_check_spans(
        input,
        Format::AsciiDoc,
        &[(1, 1), (1, 1), (1, 1), (2, 1), (1, 1)],
    );
}

#[test]
fn asciidoc_parse_rowspan() {
    let input = "|===\n|A |B\n\n.2+|Tall |C\n|D\n|===\n";
    parse_and_check_spans(
        input,
        Format::AsciiDoc,
        &[(1, 1), (1, 1), (1, 2), (1, 1), (1, 1)],
    );
}

#[test]
fn asciidoc_parse_both_spans() {
    let input = "|===\n|A |B |C\n\n2.2+|Big |D\n|E\n|===\n";
    parse_and_check_spans(
        input,
        Format::AsciiDoc,
        &[(1, 1), (1, 1), (1, 1), (2, 2), (1, 1), (1, 1)],
    );
}

// ===== AsciiDoc emitter =====

#[test]
fn asciidoc_emit_colspan() {
    let typst_input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    let output = convert(typst_input, Format::Typst, Format::AsciiDoc);
    assert!(
        output.contains("2+|Wide"),
        "Expected '2+|Wide' in output:\n{}",
        output
    );
}

#[test]
fn asciidoc_emit_rowspan() {
    let typst_input = r#"#table(
  columns: 2,
  [A], [B],
  table.cell(rowspan: 2)[Tall], [C],
  [D],
)"#;
    let output = convert(typst_input, Format::Typst, Format::AsciiDoc);
    assert!(
        output.contains(".2+|Tall"),
        "Expected '.2+|Tall' in output:\n{}",
        output
    );
}

#[test]
fn asciidoc_emit_both_spans() {
    let typst_input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2, rowspan: 2)[Big], [D],
  [E],
)"#;
    let output = convert(typst_input, Format::Typst, Format::AsciiDoc);
    assert!(
        output.contains("2.2+|Big"),
        "Expected '2.2+|Big' in output:\n{}",
        output
    );
}

// ===== Markdown emitter (graceful degradation) =====

#[test]
fn markdown_expand_colspan() {
    let typst_input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    let output = convert(typst_input, Format::Typst, Format::Markdown);
    // Should have 3 cells per row (Wide + empty + D)
    let lines: Vec<&str> = output.lines().collect();
    // Header: | A | B | C |
    assert!(
        lines[0].contains("| A |"),
        "Header should have 3 cells: {}",
        lines[0]
    );
    // Data row: | Wide |  | D | (Wide expanded with empty cell)
    let data_line = lines[2]; // skip separator
    let cells: Vec<&str> = data_line.split('|').filter(|s| !s.is_empty()).collect();
    assert_eq!(
        cells.len(),
        3,
        "Data row should have 3 cells: {}",
        data_line
    );
}

#[test]
fn markdown_expand_rowspan() {
    let typst_input = r#"#table(
  columns: 2,
  [A], [B],
  table.cell(rowspan: 2)[Tall], [C],
  [D],
)"#;
    let output = convert(typst_input, Format::Typst, Format::Markdown);
    let lines: Vec<&str> = output.lines().collect();
    // Should have 3 data rows: header + 2 body
    // Row 1: | Tall | C |
    // Row 2: |  | D |  (empty cell for rowspan)
    assert!(
        lines.len() >= 4,
        "Should have header + sep + 2 body rows:\n{}",
        output
    );
}

// ===== HTML table parsing with spans =====

#[test]
fn markdown_html_table_colspan() {
    let input = "<table>\n<tr><th>A</th><th>B</th><th>C</th></tr>\n<tr><td colspan=\"2\">Wide</td><td>D</td></tr>\n</table>\n";
    parse_and_check_spans(
        input,
        Format::Markdown,
        &[(1, 1), (1, 1), (1, 1), (2, 1), (1, 1)],
    );
}

#[test]
fn markdown_html_table_rowspan() {
    let input = "<table>\n<tr><th>A</th><th>B</th></tr>\n<tr><td rowspan=\"2\">Tall</td><td>C</td></tr>\n<tr><td>D</td></tr>\n</table>\n";
    parse_and_check_spans(
        input,
        Format::Markdown,
        &[(1, 1), (1, 1), (1, 2), (1, 1), (1, 1)],
    );
}

// ===== Cross-format conversions =====

#[test]
fn typst_colspan_to_asciidoc() {
    let input = r#"#table(
  columns: 3,
  [H1], [H2], [H3],
  table.cell(colspan: 3)[Full width],
)"#;
    let output = convert(input, Format::Typst, Format::AsciiDoc);
    assert!(output.contains("3+|Full width"), "Output:\n{}", output);
}

#[test]
fn asciidoc_colspan_to_typst() {
    let input = "|===\n|H1 |H2 |H3\n\n3+|Full width\n|===\n";
    let output = convert(input, Format::AsciiDoc, Format::Typst);
    assert!(
        output.contains("table.cell(colspan: 3)[Full width]"),
        "Output:\n{}",
        output
    );
}

#[test]
fn typst_spans_to_rst_grid() {
    let input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    let output = convert(input, Format::Typst, Format::Rst);
    // RST grid table should have + borders
    assert!(output.contains('+'), "Should be a grid table:\n{}", output);
}

#[test]
fn typst_spans_to_markdown_degradation() {
    let input = r#"#table(
  columns: 2,
  [X], [Y],
  table.cell(colspan: 2)[Both],
)"#;
    let output = convert(input, Format::Typst, Format::Markdown);
    // Should produce valid pipe table with 2 columns
    assert!(output.contains('|'), "Should be a pipe table:\n{}", output);
    // The "Both" cell should be expanded with an empty cell
    let data_lines: Vec<&str> = output.lines().skip(2).collect();
    assert!(!data_lines.is_empty());
}

// ===== Roundtrip between span-aware formats =====

#[test]
fn roundtrip_typst_asciidoc_colspan() {
    let input = r#"#table(
  columns: 3,
  [A], [B], [C],
  table.cell(colspan: 2)[Wide], [D],
)"#;
    let doc1 = morph::parse(input, Format::Typst).expect("parse");
    let adoc = morph::emit(&doc1, Format::AsciiDoc).expect("emit");
    let doc2 = morph::parse(&adoc, Format::AsciiDoc).expect("parse adoc");

    // Compare spans
    let spans1 = extract_all_spans(&doc1);
    let spans2 = extract_all_spans(&doc2);
    assert_eq!(
        spans1, spans2,
        "Spans should survive Typst -> AsciiDoc roundtrip\nAsciiDoc:\n{}",
        adoc
    );
}

#[test]
fn roundtrip_asciidoc_typst_rowspan() {
    let input = "|===\n|A |B\n\n.2+|Tall |C\n|D\n|===\n";
    let doc1 = morph::parse(input, Format::AsciiDoc).expect("parse");
    let typst = morph::emit(&doc1, Format::Typst).expect("emit");
    let doc2 = morph::parse(&typst, Format::Typst).expect("parse typst");

    let spans1 = extract_all_spans(&doc1);
    let spans2 = extract_all_spans(&doc2);
    assert_eq!(
        spans1, spans2,
        "Spans should survive AsciiDoc -> Typst roundtrip\nTypst:\n{}",
        typst
    );
}

// ===== Markdown -> span-aware format (no span info) =====

#[test]
fn markdown_no_spans_to_asciidoc() {
    let input = "| A | B |\n| --- | --- |\n| C | D |\n";
    let output = convert(input, Format::Markdown, Format::AsciiDoc);
    // Should not contain any span prefixes
    assert!(
        !output.contains("+|"),
        "No span prefixes expected:\n{}",
        output
    );
}

// ===== Helper =====

fn extract_all_spans(doc: &morph::ast::Document) -> Vec<(u32, u32)> {
    let mut spans = Vec::new();
    for block in &doc.children {
        if let morph::ast::Block::Table { headers, rows, .. } = block {
            for cell in headers {
                spans.push((cell.colspan, cell.rowspan));
            }
            for row in rows {
                for cell in row {
                    spans.push((cell.colspan, cell.rowspan));
                }
            }
        }
    }
    spans
}
