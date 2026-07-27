use morph::ast::{Block, ColumnAlignment, DescriptionItem, Document, Inline, ListItem, TableCell};
use morph::format::Format;

fn common_document() -> Document {
    Document {
        children: vec![
            Block::Heading {
                level: 2,
                content: vec![Inline::Text("Portable markup".to_string())],
            },
            Block::Paragraph {
                content: vec![
                    Inline::Text("A ".to_string()),
                    Inline::Bold(vec![Inline::Text("bold".to_string())]),
                    Inline::Text(" and ".to_string()),
                    Inline::Italic(vec![Inline::Text("careful".to_string())]),
                    Inline::Text(" converter with ".to_string()),
                    Inline::InlineCode("code".to_string()),
                    Inline::Text(" and a ".to_string()),
                    Inline::Link {
                        url: "https://example.com".to_string(),
                        text: vec![Inline::Text("link".to_string())],
                        title: None,
                    },
                    Inline::Text(".".to_string()),
                ],
            },
            Block::CodeBlock {
                language: Some("rust".to_string()),
                content: "fn main() {}".to_string(),
            },
            Block::BlockQuote {
                children: vec![Block::Paragraph {
                    content: vec![Inline::Text("Quoted text.".to_string())],
                }],
            },
            Block::UnorderedList {
                items: vec![
                    ListItem {
                        content: vec![Block::Paragraph {
                            content: vec![Inline::Text("First".to_string())],
                        }],
                    },
                    ListItem {
                        content: vec![
                            Block::Paragraph {
                                content: vec![Inline::Text("Second".to_string())],
                            },
                            Block::OrderedList {
                                start: 3,
                                items: vec![ListItem {
                                    content: vec![Block::Paragraph {
                                        content: vec![Inline::Text("Nested".to_string())],
                                    }],
                                }],
                            },
                        ],
                    },
                ],
            },
            Block::DescriptionList {
                items: vec![DescriptionItem {
                    term: vec![Inline::Text("Morph".to_string())],
                    definitions: vec![vec![Block::Paragraph {
                        content: vec![Inline::Text("A converter.".to_string())],
                    }]],
                }],
            },
            Block::Table {
                headers: vec![
                    TableCell::new(vec![Inline::Text("Name".to_string())]),
                    TableCell::new(vec![Inline::Text("Role".to_string())]),
                ],
                alignments: vec![ColumnAlignment::Left, ColumnAlignment::Right],
                rows: vec![vec![
                    TableCell::new(vec![Inline::Text("Ada".to_string())]),
                    TableCell::new(vec![Inline::Text("Engineer".to_string())]),
                ]],
            },
            Block::HorizontalRule,
        ],
    }
}

fn rich_xml_document() -> Document {
    let mut document = common_document();
    document.children.push(Block::Paragraph {
        content: vec![
            Inline::Strikethrough(vec![Inline::Text("old".to_string())]),
            Inline::Text(" ".to_string()),
            Inline::Superscript(vec![Inline::Text("up".to_string())]),
            Inline::Subscript(vec![Inline::Text("down".to_string())]),
            Inline::HardLineBreak,
            Inline::Text("next".to_string()),
            Inline::SoftLineBreak,
            Inline::Image {
                url: "diagram.svg".to_string(),
                alt: vec![Inline::Text("Diagram".to_string())],
                title: Some("Architecture".to_string()),
                link: Some("https://example.com/diagram".to_string()),
            },
        ],
    });
    document.children.push(Block::Table {
        headers: vec![
            TableCell::new(vec![Inline::Text("A".to_string())]),
            TableCell::new(vec![Inline::Text("B".to_string())]),
            TableCell::new(vec![Inline::Text("C".to_string())]),
        ],
        alignments: vec![
            ColumnAlignment::Left,
            ColumnAlignment::Center,
            ColumnAlignment::Right,
        ],
        rows: vec![
            vec![
                TableCell::with_span(vec![Inline::Text("Wide".to_string())], 2, 2),
                TableCell::new(vec![Inline::Text("D".to_string())]),
            ],
            vec![TableCell::new(vec![Inline::Text("E".to_string())])],
        ],
    });
    document.children.push(Block::RawBlock {
        format: Some("custom".to_string()),
        content: "<native value=\"1\">".to_string(),
    });
    document
}

fn assert_roundtrip(document: &Document, format: Format) {
    let output = morph::emit(document, format).unwrap();
    let reparsed = morph::parse(&output, format)
        .unwrap_or_else(|error| panic!("{format:?} parse failed: {error}\n\n{output}"));
    assert_eq!(
        document, &reparsed,
        "{format:?} round trip failed\n\n{output}"
    );
}

#[test]
fn djot_org_and_textile_preserve_common_document_structure() {
    for format in [Format::Djot, Format::Org, Format::Textile] {
        assert_roundtrip(&common_document(), format);
    }
}

#[test]
fn strict_html_and_docbook_preserve_the_rich_ast() {
    let document = rich_xml_document();
    assert_roundtrip(&document, Format::Html);
    assert_roundtrip(&document, Format::DocBook);
}

#[test]
fn asciidoc_html_asciidoc_is_semantically_lossless() {
    let asciidoc = r#"= Heading

A *bold* paragraph with https://example.com[a link].

[source,rust]
----
fn main() {}
----

* First
* Second
** Nested

|===
|Name |Role

|Ada |Engineer
|===
"#;
    let original = morph::parse(asciidoc, Format::AsciiDoc).unwrap();
    let html = morph::emit(&original, Format::Html).unwrap();
    let from_html = morph::parse(&html, Format::Html).unwrap();
    assert_eq!(original, from_html, "HTML output:\n{html}");
    let asciidoc_again = morph::emit(&from_html, Format::AsciiDoc).unwrap();
    let reparsed = morph::parse(&asciidoc_again, Format::AsciiDoc).unwrap();
    assert_eq!(original, reparsed, "AsciiDoc output:\n{asciidoc_again}");
}

#[test]
fn strict_html_rejects_unknown_or_ambiguous_markup() {
    for input in [
        "<script>alert(1)</script>",
        "<div>not part of the strict subset</div>",
        "<p class=\"lead\">attributes are not silently discarded</p>",
        "unwrapped block text",
        "<p>unterminated",
    ] {
        assert!(
            morph::parse(input, Format::Html).is_err(),
            "strict HTML unexpectedly accepted: {input}"
        );
    }
}

#[test]
fn strict_docbook_rejects_unknown_or_ambiguous_markup() {
    for input in [
        "<book xmlns=\"http://docbook.org/ns/docbook\"/>",
        "<article xmlns=\"http://docbook.org/ns/docbook\"><section/></article>",
        "<article xmlns=\"http://docbook.org/ns/docbook\"><para class=\"lead\">x</para></article>",
    ] {
        assert!(
            morph::parse(input, Format::DocBook).is_err(),
            "strict DocBook unexpectedly accepted: {input}"
        );
    }
}

#[test]
fn new_format_names_and_extensions_are_detected() {
    assert_eq!(Format::from_name("djot"), Some(Format::Djot));
    assert_eq!(Format::from_extension("dj"), Some(Format::Djot));
    assert_eq!(Format::from_name("org-mode"), Some(Format::Org));
    assert_eq!(Format::from_extension("org"), Some(Format::Org));
    assert_eq!(Format::from_name("textile"), Some(Format::Textile));
    assert_eq!(Format::from_extension("html"), Some(Format::Html));
    assert_eq!(Format::from_extension("htm"), Some(Format::Html));
    assert_eq!(Format::from_name("docbook"), Some(Format::DocBook));
    assert_eq!(Format::from_extension("dbk"), Some(Format::DocBook));
    assert_eq!(Format::from_extension("xml"), None);
}
