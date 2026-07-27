use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;
use crate::formats::strict_xml::{escape_xml, escape_xml_attr};

pub struct DocBookEmitter;

impl Emitter for DocBookEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut emitter = DocBookEmitContext {
            output: String::new(),
        };
        emitter.output.push_str(
            "<article xmlns=\"http://docbook.org/ns/docbook\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\" version=\"5.2\">\n",
        );
        emitter.emit_blocks(&doc.children)?;
        emitter.output.push_str("</article>\n");
        Ok(emitter.output)
    }
}

struct DocBookEmitContext {
    output: String,
}

impl DocBookEmitContext {
    fn emit_blocks(&mut self, blocks: &[Block]) -> Result<(), EmitError> {
        for block in blocks {
            match block {
                Block::Heading { level, content } => {
                    self.output
                        .push_str(&format!("<bridgehead renderas=\"sect{level}\">"));
                    self.emit_inlines(content)?;
                    self.output.push_str("</bridgehead>\n");
                }
                Block::Paragraph { content } => {
                    if !content.is_empty() {
                        self.output.push_str("<para>");
                        self.emit_inlines(content)?;
                        self.output.push_str("</para>\n");
                    }
                }
                Block::CodeBlock { language, content } => {
                    self.output.push_str("<programlisting");
                    if let Some(language) = language {
                        self.output.push_str(" language=\"");
                        self.output.push_str(&escape_xml_attr(language));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</programlisting>\n");
                }
                Block::BlockQuote { children } => {
                    self.output.push_str("<blockquote>\n");
                    self.emit_blocks(children)?;
                    self.output.push_str("</blockquote>\n");
                }
                Block::UnorderedList { items } => self.emit_list("itemizedlist", items, None)?,
                Block::OrderedList { start, items } => {
                    self.emit_list("orderedlist", items, Some(*start))?
                }
                Block::DescriptionList { items } => self.emit_variable_list(items)?,
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => self.emit_table(headers, alignments, rows)?,
                Block::HorizontalRule => {
                    self.output
                        .push_str("<para role=\"morph-horizontal-rule\"/>\n");
                }
                Block::RawBlock { format, content } => {
                    self.output.push_str("<programlisting role=\"morph-raw\"");
                    if let Some(format) = format {
                        self.output.push_str(" remap=\"");
                        self.output.push_str(&escape_xml_attr(format));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</programlisting>\n");
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
            self.output.push_str(" startingnumber=\"");
            self.output.push_str(&start.to_string());
            self.output.push('"');
        }
        self.output.push_str(">\n");
        for item in items {
            self.output.push_str("<listitem>\n");
            self.emit_blocks(&item.content)?;
            self.output.push_str("</listitem>\n");
        }
        self.output.push_str("</");
        self.output.push_str(tag);
        self.output.push_str(">\n");
        Ok(())
    }

    fn emit_variable_list(&mut self, items: &[DescriptionItem]) -> Result<(), EmitError> {
        self.output.push_str("<variablelist>\n");
        for item in items {
            self.output.push_str("<varlistentry><term>");
            self.emit_inlines(&item.term)?;
            self.output.push_str("</term>\n");
            for definition in &item.definitions {
                self.output.push_str("<listitem>\n");
                self.emit_blocks(definition)?;
                self.output.push_str("</listitem>\n");
            }
            self.output.push_str("</varlistentry>\n");
        }
        self.output.push_str("</variablelist>\n");
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        let columns = logical_columns(headers).max(
            rows.iter()
                .map(|row| logical_columns(row))
                .max()
                .unwrap_or(0),
        );
        self.output
            .push_str(&format!("<informaltable><tgroup cols=\"{columns}\">\n"));
        for index in 0..columns {
            self.output.push_str("<colspec colname=\"c");
            self.output.push_str(&(index + 1).to_string());
            self.output.push('"');
            if let Some(alignment) = alignments.get(index) {
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
            self.output.push_str("/>\n");
        }
        self.output.push_str("<thead>\n");
        self.emit_cals_rows(std::slice::from_ref(&headers.to_vec()), columns)?;
        self.output.push_str("</thead>\n<tbody>\n");
        self.emit_cals_rows(rows, columns)?;
        self.output
            .push_str("</tbody>\n</tgroup></informaltable>\n");
        Ok(())
    }

    fn emit_cals_rows(&mut self, rows: &[Vec<TableCell>], columns: usize) -> Result<(), EmitError> {
        let mut occupied = vec![0u32; columns];
        for row in rows {
            self.output.push_str("<row>");
            let mut column = 0;
            for cell in row {
                while column < columns && occupied[column] > 0 {
                    column += 1;
                }
                self.output.push_str("<entry");
                if cell.colspan > 1 {
                    self.output.push_str(&format!(
                        " namest=\"c{}\" nameend=\"c{}\"",
                        column + 1,
                        column + cell.colspan as usize
                    ));
                }
                if cell.rowspan > 1 {
                    self.output
                        .push_str(&format!(" morerows=\"{}\"", cell.rowspan - 1));
                }
                self.output.push('>');
                self.emit_inlines(&cell.content)?;
                self.output.push_str("</entry>");
                if cell.rowspan > 1 {
                    for slot in occupied.iter_mut().skip(column).take(cell.colspan as usize) {
                        *slot = cell.rowspan;
                    }
                }
                column += cell.colspan as usize;
            }
            self.output.push_str("</row>\n");
            for slot in &mut occupied {
                *slot = slot.saturating_sub(1);
            }
        }
        Ok(())
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) -> Result<(), EmitError> {
        for inline in inlines {
            match inline {
                Inline::Text(text) => self.output.push_str(&escape_xml(text)),
                Inline::Bold(content) => {
                    self.output.push_str("<emphasis role=\"strong\">");
                    self.emit_inlines(content)?;
                    self.output.push_str("</emphasis>");
                }
                Inline::Italic(content) => self.emit_wrapped("emphasis", content)?,
                Inline::BoldItalic(content) => {
                    self.output.push_str("<phrase role=\"bold-italic\">");
                    self.emit_inlines(content)?;
                    self.output.push_str("</phrase>");
                }
                Inline::Strikethrough(content) => {
                    self.output.push_str("<phrase role=\"strikethrough\">");
                    self.emit_inlines(content)?;
                    self.output.push_str("</phrase>");
                }
                Inline::Superscript(content) => self.emit_wrapped("superscript", content)?,
                Inline::Subscript(content) => self.emit_wrapped("subscript", content)?,
                Inline::InlineCode(code) => {
                    self.output.push_str("<code>");
                    self.output.push_str(&escape_xml(code));
                    self.output.push_str("</code>");
                }
                Inline::Link { url, text, title } => {
                    self.output.push_str("<link xlink:href=\"");
                    self.output.push_str(&escape_xml_attr(url));
                    if let Some(title) = title {
                        self.output.push_str("\" xlink:title=\"");
                        self.output.push_str(&escape_xml_attr(title));
                    }
                    self.output.push_str("\">");
                    self.emit_inlines(text)?;
                    self.output.push_str("</link>");
                }
                Inline::Image {
                    url,
                    alt,
                    title,
                    link,
                } => {
                    if let Some(link) = link {
                        self.output.push_str("<link xlink:href=\"");
                        self.output.push_str(&escape_xml_attr(link));
                        self.output.push_str("\">");
                    }
                    self.output
                        .push_str("<inlinemediaobject><imageobject><imagedata fileref=\"");
                    self.output.push_str(&escape_xml_attr(url));
                    if let Some(title) = title {
                        self.output.push_str("\" xlink:title=\"");
                        self.output.push_str(&escape_xml_attr(title));
                    }
                    self.output
                        .push_str("\"/></imageobject><textobject><phrase>");
                    self.output.push_str(&escape_xml(&plain_text(alt)));
                    self.output
                        .push_str("</phrase></textobject></inlinemediaobject>");
                    if link.is_some() {
                        self.output.push_str("</link>");
                    }
                }
                Inline::HardLineBreak => {
                    self.output.push_str("<phrase role=\"morph-hard-break\"/>");
                }
                Inline::SoftLineBreak => {
                    self.output.push_str("<phrase role=\"morph-soft-break\"/>");
                }
                Inline::RawInline { format, content } => {
                    self.output.push_str("<phrase role=\"morph-raw\"");
                    if let Some(format) = format {
                        self.output.push_str(" remap=\"");
                        self.output.push_str(&escape_xml_attr(format));
                        self.output.push('"');
                    }
                    self.output.push('>');
                    self.output.push_str(&escape_xml(content));
                    self.output.push_str("</phrase>");
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

fn logical_columns(cells: &[TableCell]) -> usize {
    cells.iter().map(|cell| cell.colspan as usize).sum()
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
