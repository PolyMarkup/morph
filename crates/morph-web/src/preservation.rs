use morph::ast::{Block, ColumnAlignment, Document, Inline};
use morph::format::Format;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PreservationStatus {
    Preserved,
    Changed,
    Unverifiable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FeatureChange {
    pub(crate) feature: &'static str,
    pub(crate) before: usize,
    pub(crate) after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PreservationReport {
    pub(crate) status: PreservationStatus,
    pub(crate) changes: Vec<FeatureChange>,
}

pub(crate) fn analyze(original: &Document, output: &str, target: Format) -> PreservationReport {
    let reparsed = match morph::parse(output, target) {
        Ok(document) => document,
        Err(_) => {
            return PreservationReport {
                status: PreservationStatus::Unverifiable,
                changes: vec![FeatureChange {
                    feature: "target_reparse",
                    before: 1,
                    after: 0,
                }],
            };
        }
    };

    if *original == reparsed {
        return PreservationReport {
            status: PreservationStatus::Preserved,
            changes: Vec::new(),
        };
    }

    let before = inventory(original);
    let after = inventory(&reparsed);
    let mut changes = Vec::new();
    let features = before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for feature in features {
        let before_count = before.get(feature).copied().unwrap_or(0);
        let after_count = after.get(feature).copied().unwrap_or(0);
        if before_count != after_count {
            changes.push(FeatureChange {
                feature,
                before: before_count,
                after: after_count,
            });
        }
    }
    if changes.is_empty() {
        changes.push(FeatureChange {
            feature: "semantic_structure",
            before: 1,
            after: 0,
        });
    }

    PreservationReport {
        status: PreservationStatus::Changed,
        changes,
    }
}

fn inventory(document: &Document) -> BTreeMap<&'static str, usize> {
    let mut inventory = Inventory::default();
    inventory.visit_blocks(&document.children, 1);
    inventory.counts
}

#[derive(Default)]
struct Inventory {
    counts: BTreeMap<&'static str, usize>,
    max_block_depth: usize,
}

impl Inventory {
    fn increment(&mut self, feature: &'static str) {
        *self.counts.entry(feature).or_default() += 1;
    }

    fn add(&mut self, feature: &'static str, amount: usize) {
        *self.counts.entry(feature).or_default() += amount;
    }

    fn visit_blocks(&mut self, blocks: &[Block], depth: usize) {
        self.max_block_depth = self.max_block_depth.max(depth);
        self.counts
            .insert("maximum_block_depth", self.max_block_depth);
        for block in blocks {
            self.increment("block_nodes");
            match block {
                Block::Heading { content, .. } => {
                    self.increment("headings");
                    self.visit_inlines(content);
                }
                Block::Paragraph { content } => {
                    self.increment("paragraphs");
                    self.visit_inlines(content);
                }
                Block::CodeBlock { language, .. } => {
                    self.increment("code_blocks");
                    if language.is_some() {
                        self.increment("code_languages");
                    }
                }
                Block::BlockQuote { children } => {
                    self.increment("block_quotes");
                    self.visit_blocks(children, depth + 1);
                }
                Block::UnorderedList { items } => {
                    self.increment("unordered_lists");
                    self.add("list_items", items.len());
                    for item in items {
                        self.visit_blocks(&item.content, depth + 1);
                    }
                }
                Block::OrderedList { start, items } => {
                    self.increment("ordered_lists");
                    self.add("ordered_start_total", *start as usize);
                    if *start != 1 {
                        self.increment("nondefault_ordered_starts");
                    }
                    self.add("list_items", items.len());
                    for item in items {
                        self.visit_blocks(&item.content, depth + 1);
                    }
                }
                Block::DescriptionList { items } => {
                    self.increment("description_lists");
                    self.add("description_items", items.len());
                    for item in items {
                        self.add("definitions", item.definitions.len());
                        self.visit_inlines(&item.term);
                        for definition in &item.definitions {
                            self.visit_blocks(definition, depth + 1);
                        }
                    }
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => {
                    self.increment("tables");
                    self.add("table_rows", rows.len() + 1);
                    for alignment in alignments {
                        self.increment(match alignment {
                            ColumnAlignment::Left => "alignment_left",
                            ColumnAlignment::Center => "alignment_center",
                            ColumnAlignment::Right => "alignment_right",
                            ColumnAlignment::Default => "alignment_default",
                        });
                    }
                    for cell in headers.iter().chain(rows.iter().flatten()) {
                        self.increment("table_cells");
                        if cell.colspan > 1 {
                            self.increment("column_spans");
                            self.add("column_span_width", cell.colspan as usize);
                        }
                        if cell.rowspan > 1 {
                            self.increment("row_spans");
                            self.add("row_span_height", cell.rowspan as usize);
                        }
                        self.visit_inlines(&cell.content);
                    }
                }
                Block::HorizontalRule => self.increment("horizontal_rules"),
                Block::RawBlock { format, .. } => {
                    self.increment("raw_blocks");
                    if format.is_some() {
                        self.increment("raw_block_formats");
                    }
                }
            }
        }
    }

    fn visit_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            self.increment("inline_nodes");
            let nested = match inline {
                Inline::Text(text) => {
                    self.add("text_characters", text.chars().count());
                    None
                }
                Inline::Bold(content) => {
                    self.increment("bold");
                    Some(content)
                }
                Inline::Italic(content) => {
                    self.increment("italic");
                    Some(content)
                }
                Inline::BoldItalic(content) => {
                    self.increment("bold_italic");
                    Some(content)
                }
                Inline::Strikethrough(content) => {
                    self.increment("strikethrough");
                    Some(content)
                }
                Inline::Superscript(content) => {
                    self.increment("superscript");
                    Some(content)
                }
                Inline::Subscript(content) => {
                    self.increment("subscript");
                    Some(content)
                }
                Inline::InlineCode(code) => {
                    self.increment("inline_code");
                    self.add("inline_code_characters", code.chars().count());
                    None
                }
                Inline::Link { text, title, .. } => {
                    self.increment("links");
                    if title.is_some() {
                        self.increment("link_titles");
                    }
                    Some(text)
                }
                Inline::Image {
                    alt, title, link, ..
                } => {
                    self.increment("images");
                    if title.is_some() {
                        self.increment("image_titles");
                    }
                    if link.is_some() {
                        self.increment("linked_images");
                    }
                    Some(alt)
                }
                Inline::HardLineBreak => {
                    self.increment("hard_breaks");
                    None
                }
                Inline::SoftLineBreak => {
                    self.increment("soft_breaks");
                    None
                }
                Inline::RawInline { format, .. } => {
                    self.increment("raw_inlines");
                    if format.is_some() {
                        self.increment("raw_inline_formats");
                    }
                    None
                }
            };
            if let Some(content) = nested {
                self.visit_inlines(content);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morph::ast::TableCell;

    #[test]
    fn reports_exact_round_trips_as_preserved() {
        let input = "# Preserved\n\nA **bold** paragraph.\n";
        let original = morph::parse(input, Format::Markdown).unwrap();
        let output = morph::emit(&original, Format::Html).unwrap();

        assert_eq!(
            analyze(&original, &output, Format::Html),
            PreservationReport {
                status: PreservationStatus::Preserved,
                changes: Vec::new(),
            }
        );
    }

    #[test]
    fn explains_span_degradation() {
        let original = Document {
            children: vec![Block::Table {
                headers: vec![
                    TableCell::new(vec![Inline::Text("A".into())]),
                    TableCell::new(vec![Inline::Text("B".into())]),
                ],
                alignments: vec![ColumnAlignment::Left, ColumnAlignment::Right],
                rows: vec![vec![TableCell::with_span(
                    vec![Inline::Text("Wide".into())],
                    2,
                    1,
                )]],
            }],
        };
        let output = morph::emit(&original, Format::Markdown).unwrap();
        let report = analyze(&original, &output, Format::Markdown);

        assert_eq!(report.status, PreservationStatus::Changed);
        assert!(report.changes.iter().any(|change| {
            change.feature == "column_spans" && change.before == 1 && change.after == 0
        }));
    }

    #[test]
    fn explains_alignment_degradation() {
        let original = Document {
            children: vec![Block::Table {
                headers: vec![
                    TableCell::new(vec![Inline::Text("A".into())]),
                    TableCell::new(vec![Inline::Text("B".into())]),
                ],
                alignments: vec![ColumnAlignment::Left, ColumnAlignment::Right],
                rows: vec![vec![
                    TableCell::new(vec![Inline::Text("C".into())]),
                    TableCell::new(vec![Inline::Text("D".into())]),
                ]],
            }],
        };
        let output = morph::emit(&original, Format::AsciiDoc).unwrap();
        let report = analyze(&original, &output, Format::AsciiDoc);

        assert_eq!(report.status, PreservationStatus::Changed);
        assert!(report.changes.iter().any(|change| {
            change.feature == "alignment_right" && change.before == 1 && change.after == 0
        }));
    }

    #[test]
    fn reports_outputs_that_cannot_be_reparsed() {
        let original = morph::parse("# Source\n", Format::Markdown).unwrap();
        let report = analyze(&original, "<script>", Format::Html);

        assert_eq!(report.status, PreservationStatus::Unverifiable);
        assert_eq!(report.changes[0].feature, "target_reparse");
    }
}
