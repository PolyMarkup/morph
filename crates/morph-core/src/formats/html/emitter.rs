use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;
use crate::formats::strict_xml::{escape_xml, escape_xml_attr};

pub struct HtmlEmitter;

impl Emitter for HtmlEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut emitter = HtmlEmitContext {
            output: String::new(),
        };
        emitter.emit_blocks(&doc.children)?;
        Ok(format!("{}\n", emitter.output.trim_end_matches('\n')))
    }
}

struct HtmlEmitContext {
    output: String,
}

impl HtmlEmitContext {
    fn emit_blocks(&mut self, blocks: &[Block]) -> Result<(), EmitError> {
        for block in blocks {
            match block {
                Block::Heading { level, content } => {
                    self.output.push_str(&format!("<h{level}>"));
                    self.emit_inlines(content)?;
                    self.output.push_str(&format!("</h{level}>\n"));
                }
                Block::Paragraph { content } => {
                    if !content.is_empty() {
                        self.output.push_str("<p>");
                        self.emit_inlines(content)?;
                        self.output.push_str("</p>\n");
                    }
                }
                Block::CodeBlock { language, content } => {
                    self.output.push_str("<pre><code");
                    if let Some(language) = language {
                        self.output.push_str(" class=\"language-");
                        self.output.push_str(&escape_xml_attr(language));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</code></pre>\n");
                }
                Block::BlockQuote { children } => {
                    self.output.push_str("<blockquote>\n");
                    self.emit_blocks(children)?;
                    self.output.push_str("</blockquote>\n");
                }
                Block::UnorderedList { items } => self.emit_list("ul", items, None)?,
                Block::OrderedList { start, items } => self.emit_list("ol", items, Some(*start))?,
                Block::DescriptionList { items } => {
                    self.output.push_str("<dl>\n");
                    for item in items {
                        self.output.push_str("<dt>");
                        self.emit_inlines(&item.term)?;
                        self.output.push_str("</dt>\n");
                        for definition in &item.definitions {
                            self.output.push_str("<dd>\n");
                            self.emit_blocks(definition)?;
                            self.output.push_str("</dd>\n");
                        }
                    }
                    self.output.push_str("</dl>\n");
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => self.emit_table(headers, alignments, rows)?,
                Block::HorizontalRule => self.output.push_str("<hr>\n"),
                Block::RawBlock { format, content } => {
                    self.output.push_str("<pre data-morph-raw-format=\"");
                    self.output
                        .push_str(&escape_xml_attr(format.as_deref().unwrap_or("")));
                    self.output.push_str("\">");
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</pre>\n");
                }
            }
        }
        Ok(())
    }

    fn emit_list(
        &mut self,
        tag: &str,
        items: &[ListItem],
        start: Option<u32>,
    ) -> Result<(), EmitError> {
        self.output.push('<');
        self.output.push_str(tag);
        if let Some(start) = start
            && start != 1
        {
            self.output.push_str(" start=\"");
            self.output.push_str(&start.to_string());
            self.output.push('"');
        }
        self.output.push_str(">\n");
        for item in items {
            self.output.push_str("<li>\n");
            self.emit_blocks(&item.content)?;
            self.output.push_str("</li>\n");
        }
        self.output.push_str("</");
        self.output.push_str(tag);
        self.output.push_str(">\n");
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        self.output.push_str("<table>\n<thead><tr>");
        for (index, cell) in headers.iter().enumerate() {
            self.emit_cell("th", cell, alignments.get(index))?;
        }
        self.output.push_str("</tr></thead>\n<tbody>\n");
        for row in rows {
            self.output.push_str("<tr>");
            for (index, cell) in row.iter().enumerate() {
                self.emit_cell("td", cell, alignments.get(index))?;
            }
            self.output.push_str("</tr>\n");
        }
        self.output.push_str("</tbody>\n</table>\n");
        Ok(())
    }

    fn emit_cell(
        &mut self,
        tag: &str,
        cell: &TableCell,
        alignment: Option<&ColumnAlignment>,
    ) -> Result<(), EmitError> {
        self.output.push('<');
        self.output.push_str(tag);
        if cell.colspan > 1 {
            self.output
                .push_str(&format!(" colspan=\"{}\"", cell.colspan));
        }
        if cell.rowspan > 1 {
            self.output
                .push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
        }
        if let Some(alignment) = alignment {
            let value = match alignment {
                ColumnAlignment::Left => Some("left"),
                ColumnAlignment::Center => Some("center"),
                ColumnAlignment::Right => Some("right"),
                ColumnAlignment::Default => None,
            };
            if let Some(value) = value {
                self.output.push_str(" align=\"");
                self.output.push_str(value);
                self.output.push('"');
            }
        }
        self.output.push('>');
        self.emit_inlines(&cell.content)?;
        self.output.push_str("</");
        self.output.push_str(tag);
        self.output.push('>');
        Ok(())
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) -> Result<(), EmitError> {
        for inline in inlines {
            match inline {
                Inline::Text(text) => self.output.push_str(&escape_xml(text)),
                Inline::Bold(content) => self.emit_wrapped("strong", content)?,
                Inline::Italic(content) => self.emit_wrapped("em", content)?,
                Inline::BoldItalic(content) => {
                    self.output.push_str("<strong><em>");
                    self.emit_inlines(content)?;
                    self.output.push_str("</em></strong>");
                }
                Inline::Strikethrough(content) => self.emit_wrapped("del", content)?,
                Inline::Superscript(content) => self.emit_wrapped("sup", content)?,
                Inline::Subscript(content) => self.emit_wrapped("sub", content)?,
                Inline::InlineCode(code) => {
                    self.output.push_str("<code>");
                    self.output.push_str(&escape_xml(code));
                    self.output.push_str("</code>");
                }
                Inline::Link { url, text, title } => {
                    self.output.push_str("<a href=\"");
                    self.output.push_str(&escape_xml_attr(url));
                    self.output.push('"');
                    if let Some(title) = title {
                        self.output.push_str(" title=\"");
                        self.output.push_str(&escape_xml_attr(title));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    self.emit_inlines(text)?;
                    self.output.push_str("</a>");
                }
                Inline::Image {
                    url,
                    alt,
                    title,
                    link,
                } => {
                    if let Some(link) = link {
                        self.output.push_str("<a href=\"");
                        self.output.push_str(&escape_xml_attr(link));
                        self.output.push_str("\">");
                    }
                    self.output.push_str("<img src=\"");
                    self.output.push_str(&escape_xml_attr(url));
                    self.output.push_str("\" alt=\"");
                    self.output.push_str(&escape_xml_attr(&plain_text(alt)));
                    self.output.push('"');
                    if let Some(title) = title {
                        self.output.push_str(" title=\"");
                        self.output.push_str(&escape_xml_attr(title));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    if link.is_some() {
                        self.output.push_str("</a>");
                    }
                }
                Inline::HardLineBreak => self.output.push_str("<br>"),
                Inline::SoftLineBreak => self.output.push('\n'),
                Inline::RawInline { format, content } => {
                    self.output.push_str("<span data-morph-raw-format=\"");
                    self.output
                        .push_str(&escape_xml_attr(format.as_deref().unwrap_or("")));
                    self.output.push_str("\">");
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</span>");
                }
            }
        }
        Ok(())
    }

    fn emit_wrapped(&mut self, tag: &str, content: &[Inline]) -> Result<(), EmitError> {
        self.output.push('<');
        self.output.push_str(tag);
        self.output.push('>');
        self.emit_inlines(content)?;
        self.output.push_str("</");
        self.output.push_str(tag);
        self.output.push('>');
        Ok(())
    }
}

fn plain_text(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::InlineCode(text) => output.push_str(text),
            Inline::Bold(content)
            | Inline::Italic(content)
            | Inline::BoldItalic(content)
            | Inline::Strikethrough(content)
            | Inline::Superscript(content)
            | Inline::Subscript(content) => output.push_str(&plain_text(content)),
            Inline::Link { text, .. } => output.push_str(&plain_text(text)),
            Inline::Image { alt, .. } => output.push_str(&plain_text(alt)),
            Inline::HardLineBreak | Inline::SoftLineBreak => output.push(' '),
            Inline::RawInline { content, .. } => output.push_str(content),
        }
    }
    output
}
