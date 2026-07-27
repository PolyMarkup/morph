use morph::ast::{Block, Document, Inline};
use morph::format::Format;

#[test]
fn latex_format_names_and_extensions_are_detected() {
    assert_eq!(Format::from_extension("tex"), Some(Format::Latex));
    assert_eq!(Format::from_extension("latex"), Some(Format::Latex));
    assert_eq!(Format::from_name("tex"), Some(Format::Latex));
    assert_eq!(Format::from_name("LaTeX"), Some(Format::Latex));
}

#[test]
fn markdown_round_trips_through_latex() {
    let markdown = r#"# Heading

This has **bold**, *italic*, `code`, and a [link](https://example.com).

- First
- Second
  - Nested

> A quotation.

```rust
fn main() {}
```

| Name | Role |
| --- | --- |
| Ada | Engineer |

---
"#;

    let original = morph::parse(markdown, Format::Markdown).unwrap();
    let latex = morph::emit(&original, Format::Latex).unwrap();
    let reparsed = morph::parse(&latex, Format::Latex).unwrap();

    assert_eq!(original, reparsed, "LaTeX output:\n{latex}");
}

#[test]
fn latex_escapes_text_without_changing_the_ast() {
    let document = Document {
        children: vec![Block::Paragraph {
            content: vec![
                Inline::Text("Symbols: # $ % & _ { } \\ ^ ~ and ".to_string()),
                Inline::InlineCode("value_with_{braces}".to_string()),
                Inline::Text(" plus ".to_string()),
                Inline::Link {
                    url: "https://example.com/a_b?x=1&y=2#frag".to_string(),
                    text: vec![Inline::Text("special URL".to_string())],
                    title: None,
                },
            ],
        }],
    };

    let latex = morph::emit(&document, Format::Latex).unwrap();
    let reparsed = morph::parse(&latex, Format::Latex).unwrap();

    assert_eq!(document, reparsed, "LaTeX output:\n{latex}");
}

#[test]
fn full_latex_documents_and_common_code_environments_are_parsed() {
    let latex = r#"\documentclass{article}
\usepackage{minted}
\begin{document}
\section{Hello}

A \textbf{bold} paragraph. % ignored comment

\begin{minted}{python}
print("hello")
\end{minted}

\begin{verbatim}
literal % content
\end{verbatim}
\end{document}
"#;

    let document = morph::parse(latex, Format::Latex).unwrap();
    assert!(matches!(
        document.children[0],
        Block::Heading { level: 1, .. }
    ));
    assert!(matches!(
        &document.children[2],
        Block::CodeBlock { language: Some(language), content }
            if language == "python" && content == "print(\"hello\")"
    ));
    assert!(matches!(
        &document.children[3],
        Block::CodeBlock { language: None, content } if content == "literal % content"
    ));
}

#[test]
fn latex_table_spans_are_preserved() {
    let latex = r#"\begin{tabular}{lll}
A & B & C \\
\hline
\multirow{2}{*}{\multicolumn{2}{l}{Wide}} & D \\
E \\
\end{tabular}
"#;

    let document = morph::parse(latex, Format::Latex).unwrap();
    let Block::Table { rows, .. } = &document.children[0] else {
        panic!("expected table");
    };
    assert_eq!((rows[0][0].colspan, rows[0][0].rowspan), (2, 2));

    let emitted = morph::emit(&document, Format::Latex).unwrap();
    assert!(emitted.contains("\\multirow{2}{*}{\\multicolumn{2}{l}{Wide}}"));
}
