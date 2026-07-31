use morph::ast::{Block, ColumnAlignment, Inline};
use morph::format::Format;
use std::path::PathBuf;

const SOURCE: &str = include_str!("../../../demo/source/all-elements.html");

#[derive(Default)]
struct Coverage {
    heading: bool,
    paragraph: bool,
    code_block: bool,
    block_quote: bool,
    unordered_list: bool,
    ordered_list: bool,
    nondefault_list_start: bool,
    description_list: bool,
    table: bool,
    horizontal_rule: bool,
    raw_block: bool,
    text: bool,
    bold: bool,
    italic: bool,
    bold_italic: bool,
    strikethrough: bool,
    superscript: bool,
    subscript: bool,
    inline_code: bool,
    link: bool,
    titled_link: bool,
    image: bool,
    titled_linked_image: bool,
    hard_break: bool,
    soft_break: bool,
    raw_inline: bool,
    left_alignment: bool,
    center_alignment: bool,
    right_alignment: bool,
    default_alignment: bool,
    colspan: bool,
    rowspan: bool,
    multiple_definitions: bool,
}

impl Coverage {
    fn visit_blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            match block {
                Block::Heading { content, .. } => {
                    self.heading = true;
                    self.visit_inlines(content);
                }
                Block::Paragraph { content } => {
                    self.paragraph = true;
                    self.visit_inlines(content);
                }
                Block::CodeBlock { .. } => self.code_block = true,
                Block::BlockQuote { children } => {
                    self.block_quote = true;
                    self.visit_blocks(children);
                }
                Block::UnorderedList { items } => {
                    self.unordered_list = true;
                    for item in items {
                        self.visit_blocks(&item.content);
                    }
                }
                Block::OrderedList { start, items } => {
                    self.ordered_list = true;
                    self.nondefault_list_start |= *start != 1;
                    for item in items {
                        self.visit_blocks(&item.content);
                    }
                }
                Block::DescriptionList { items } => {
                    self.description_list = true;
                    for item in items {
                        self.multiple_definitions |= item.definitions.len() > 1;
                        self.visit_inlines(&item.term);
                        for definition in &item.definitions {
                            self.visit_blocks(definition);
                        }
                    }
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => {
                    self.table = true;
                    for alignment in alignments {
                        match alignment {
                            ColumnAlignment::Left => self.left_alignment = true,
                            ColumnAlignment::Center => self.center_alignment = true,
                            ColumnAlignment::Right => self.right_alignment = true,
                            ColumnAlignment::Default => self.default_alignment = true,
                        }
                    }
                    for cell in headers.iter().chain(rows.iter().flatten()) {
                        self.colspan |= cell.colspan > 1;
                        self.rowspan |= cell.rowspan > 1;
                        self.visit_inlines(&cell.content);
                    }
                }
                Block::HorizontalRule => self.horizontal_rule = true,
                Block::RawBlock { .. } => self.raw_block = true,
            }
        }
    }

    fn visit_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            let nested = match inline {
                Inline::Text(_) => {
                    self.text = true;
                    None
                }
                Inline::Bold(content) => {
                    self.bold = true;
                    Some(content)
                }
                Inline::Italic(content) => {
                    self.italic = true;
                    Some(content)
                }
                Inline::BoldItalic(content) => {
                    self.bold_italic = true;
                    Some(content)
                }
                Inline::Strikethrough(content) => {
                    self.strikethrough = true;
                    Some(content)
                }
                Inline::Superscript(content) => {
                    self.superscript = true;
                    Some(content)
                }
                Inline::Subscript(content) => {
                    self.subscript = true;
                    Some(content)
                }
                Inline::InlineCode(_) => {
                    self.inline_code = true;
                    None
                }
                Inline::Link { text, title, .. } => {
                    self.link = true;
                    self.titled_link |= title.is_some();
                    Some(text)
                }
                Inline::Image {
                    alt, title, link, ..
                } => {
                    self.image = true;
                    self.titled_linked_image |= title.is_some() && link.is_some();
                    Some(alt)
                }
                Inline::HardLineBreak => {
                    self.hard_break = true;
                    None
                }
                Inline::SoftLineBreak => {
                    self.soft_break = true;
                    None
                }
                Inline::RawInline { .. } => {
                    self.raw_inline = true;
                    None
                }
            };
            if let Some(content) = nested {
                self.visit_inlines(content);
            }
        }
    }

    fn assert_complete(&self) {
        let checks = [
            ("heading", self.heading),
            ("paragraph", self.paragraph),
            ("code block", self.code_block),
            ("block quote", self.block_quote),
            ("unordered list", self.unordered_list),
            ("ordered list", self.ordered_list),
            ("non-default ordered-list start", self.nondefault_list_start),
            ("description list", self.description_list),
            ("multiple definitions", self.multiple_definitions),
            ("table", self.table),
            ("horizontal rule", self.horizontal_rule),
            ("raw block", self.raw_block),
            ("text", self.text),
            ("bold", self.bold),
            ("italic", self.italic),
            ("bold italic", self.bold_italic),
            ("strikethrough", self.strikethrough),
            ("superscript", self.superscript),
            ("subscript", self.subscript),
            ("inline code", self.inline_code),
            ("link", self.link),
            ("titled link", self.titled_link),
            ("image", self.image),
            ("titled linked image", self.titled_linked_image),
            ("hard break", self.hard_break),
            ("soft break", self.soft_break),
            ("raw inline", self.raw_inline),
            ("left table alignment", self.left_alignment),
            ("center table alignment", self.center_alignment),
            ("right table alignment", self.right_alignment),
            ("default table alignment", self.default_alignment),
            ("column span", self.colspan),
            ("row span", self.rowspan),
        ];
        let missing: Vec<&str> = checks
            .into_iter()
            .filter_map(|(name, present)| (!present).then_some(name))
            .collect();
        assert!(
            missing.is_empty(),
            "static demo is missing supported elements: {}",
            missing.join(", ")
        );
    }
}

fn demo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../demo")
}

#[test]
fn static_demo_contains_every_supported_element() {
    let document = morph::parse(SOURCE, Format::Html).expect("parse strict HTML demo source");
    let mut coverage = Coverage::default();
    coverage.visit_blocks(&document.children);
    coverage.assert_complete();
}

#[test]
fn static_demo_outputs_are_current() {
    let formats = [
        (Format::Markdown, "md"),
        (Format::AsciiDoc, "adoc"),
        (Format::Rst, "rst"),
        (Format::Typst, "typ"),
        (Format::Latex, "tex"),
        (Format::Djot, "dj"),
        (Format::Org, "org"),
        (Format::Textile, "textile"),
        (Format::Html, "html"),
        (Format::DocBook, "dbk"),
    ];

    for (format, extension) in formats {
        let expected_path = demo_root()
            .join("generated")
            .join(format!("all-elements.{extension}"));
        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", expected_path.display()));
        let actual = morph::convert(SOURCE, Format::Html, format)
            .unwrap_or_else(|error| panic!("emit {format:?}: {error}"));
        assert_eq!(
            actual,
            expected,
            "{} is stale; run ./demo/generate.sh",
            expected_path.display()
        );
    }
}
