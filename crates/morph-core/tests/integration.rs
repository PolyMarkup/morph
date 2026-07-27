use morph::format::Format;

const FIXTURE_DIR: &str = "tests/fixtures";

struct FormatInfo {
    format: Format,
    extension: &'static str,
    name: &'static str,
}

const FORMATS: &[FormatInfo] = &[
    FormatInfo {
        format: Format::Markdown,
        extension: "md",
        name: "Markdown",
    },
    FormatInfo {
        format: Format::AsciiDoc,
        extension: "adoc",
        name: "AsciiDoc",
    },
    FormatInfo {
        format: Format::Rst,
        extension: "rst",
        name: "RST",
    },
    FormatInfo {
        format: Format::Typst,
        extension: "typ",
        name: "Typst",
    },
    FormatInfo {
        format: Format::Latex,
        extension: "tex",
        name: "LaTeX",
    },
];

fn run_category(category: &str) {
    run_category_formats(category, FORMATS);
}

/// Run integration tests for a category using only the specified format subset.
fn run_category_formats(category: &str, formats: &[FormatInfo]) {
    // Load format files
    let mut contents: Vec<(&FormatInfo, String)> = Vec::new();
    for info in formats {
        let path = format!("{FIXTURE_DIR}/{category}/{category}.{}", info.extension);
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        contents.push((info, content));
    }

    // Test every conversion pair
    let mut pass_count = 0;
    let mut failures = Vec::new();

    for (from_info, from_content) in &contents {
        for (to_info, to_content) in &contents {
            if from_info.extension == to_info.extension {
                continue;
            }

            let actual = morph::convert(from_content, from_info.format, to_info.format)
                .unwrap_or_else(|e| {
                    panic!(
                        "{category}: {} -> {} conversion error: {e}",
                        from_info.name, to_info.name
                    )
                });

            if actual == *to_content {
                pass_count += 1;
            } else {
                failures.push((from_info.name, to_info.name, to_content.clone(), actual));
            }
        }
    }

    if !failures.is_empty() {
        let total = pass_count + failures.len();
        let mut msg = format!(
            "\n{category}: {pass_count} passed, {} failed out of {total} conversions\n",
            failures.len()
        );
        for (from, to, expected, actual) in &failures {
            msg.push_str(&format!("\n  FAIL: {from} -> {to}\n"));
            msg.push_str("    Expected:\n");
            for line in expected.lines() {
                msg.push_str(&format!("      |{line}|\n"));
            }
            msg.push_str("    Actual:\n");
            for line in actual.lines() {
                msg.push_str(&format!("      |{line}|\n"));
            }
        }
        panic!("{msg}");
    }
}

// --- Headings ---
// Demonstrates: # H1 / ## H2 / ### H3 across all formats
// MD: # Heading    AsciiDoc: = Heading    RST: Heading\n======    Typst: = Heading    LaTeX: \section{Heading}

#[test]
fn integration_headings() {
    run_category("headings");
}

// --- Paragraphs ---
// Demonstrates: Plain text paragraphs separated by blank lines

#[test]
fn integration_paragraphs() {
    run_category("paragraphs");
}

// --- Code ---
// Demonstrates: `inline code` and fenced code blocks with language
// MD: ```java    AsciiDoc: [source,java]\n----    RST: .. code-block:: java
// Typst: ```java    LaTeX: \begin{lstlisting}[language=java]

#[test]
fn integration_code() {
    run_category("code");
}

// --- Markup ---
// Demonstrates: **bold** and *italic* formatting
// MD: **bold** *italic*    AsciiDoc: *bold* _italic_    RST: **bold** *italic*
// Typst: *bold* _italic_    LaTeX: \textbf{bold} \textit{italic}

#[test]
fn integration_markup() {
    run_category("markup");
}

// --- Links ---
// Demonstrates: Hyperlinks with display text
// MD: [text](url)    AsciiDoc: url[text]    RST: `text <url>`_
// Typst: #link("url")[text]    LaTeX: \href{url}{text}

#[test]
fn integration_links() {
    run_category("links");
}

// --- Lists ---
// Demonstrates: Unordered (bullet) and ordered (numbered) lists
// MD: - item / 1. item    AsciiDoc: * item / . item    RST: * item / #. item
// Typst: - item / + item    LaTeX: itemize / enumerate

#[test]
fn integration_lists() {
    run_category("lists");
}

// --- Tables ---
// Demonstrates: Simple two-column table with header row
// MD: | H | H |    AsciiDoc: |===    RST: ===    Typst: #table(...)
// LaTeX: \begin{tabular}{ll}

#[test]
fn integration_tables() {
    run_category("tables");
}

// --- Blockquotes ---
// Demonstrates: Quoted text blocks
// MD: > text    AsciiDoc: ____\ntext\n____    RST: (indented)
// Typst: #quote[text]    LaTeX: \begin{quote}

#[test]
fn integration_blockquotes() {
    run_category("blockquotes");
}

// --- Horizontal Rules ---
// Demonstrates: Horizontal divider lines
// MD: ---    AsciiDoc: '''    RST: ----    Typst: #line(length: 100%)    LaTeX: \hrule

#[test]
fn integration_horizontal_rules() {
    run_category("horizontal_rules");
}

// --- Table spans ---
// Demonstrates: colspan and rowspan in tables
// AsciiDoc, Typst, and LaTeX round-trip spans; RST is tested as a one-way target.

const SPAN_FORMATS: &[FormatInfo] = &[
    FormatInfo {
        format: Format::AsciiDoc,
        extension: "adoc",
        name: "AsciiDoc",
    },
    FormatInfo {
        format: Format::Rst,
        extension: "rst",
        name: "RST",
    },
    FormatInfo {
        format: Format::Typst,
        extension: "typ",
        name: "Typst",
    },
    FormatInfo {
        format: Format::Latex,
        extension: "tex",
        name: "LaTeX",
    },
];

#[test]
fn integration_table_spans() {
    run_category_formats("table_spans", SPAN_FORMATS);
}

#[test]
fn integration_table_spans_to_markdown() {
    // One-way degradation: span-aware formats to Markdown (spans expand to flat cells)
    for info in SPAN_FORMATS {
        let path = format!("{FIXTURE_DIR}/table_spans/table_spans.{}", info.extension);
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

        let md = morph::convert(&content, info.format, Format::Markdown)
            .unwrap_or_else(|e| panic!("{} -> Markdown conversion error: {e}", info.name));

        // Verify it's a valid Markdown table with the right number of columns
        let lines: Vec<&str> = md.lines().collect();
        assert!(
            lines.len() >= 5,
            "{} -> Markdown should have header + sep + 4 data rows:\n{md}",
            info.name
        );
        // Each row should have 3 pipe-delimited columns
        for (i, line) in lines.iter().enumerate() {
            let pipes = line.chars().filter(|&c| c == '|').count();
            assert!(
                pipes >= 4,
                "{} -> Markdown row {i} should have at least 4 pipes (3 cols): {line}",
                info.name
            );
        }
    }
}
