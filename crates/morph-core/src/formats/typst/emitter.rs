use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;

pub struct TypstEmitter;

impl Emitter for TypstEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut ctx = TypstEmitContext::new();
        ctx.emit_blocks(&doc.children)?;
        Ok(ctx.finish())
    }
}

struct TypstEmitContext {
    output: String,
}

impl TypstEmitContext {
    fn new() -> Self {
        TypstEmitContext {
            output: String::new(),
        }
    }

    fn finish(self) -> String {
        let trimmed = self.output.trim_end_matches('\n');
        format!("{trimmed}\n")
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push_newline(&mut self) {
        self.output.push('\n');
    }

    fn ensure_blank_line(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            if self.output.ends_with('\n') {
                self.push_newline();
            } else {
                self.push("\n\n");
            }
        }
    }

    fn emit_blocks(&mut self, blocks: &[Block]) -> Result<(), EmitError> {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                match block {
                    Block::Paragraph { content } if content.is_empty() => continue,
                    _ => self.ensure_blank_line(),
                }
            }
            match block {
                Block::Heading { level, content } => {
                    let markers: String = "=".repeat(*level as usize);
                    self.push(&markers);
                    self.push(" ");
                    self.emit_inlines(content)?;
                    self.push_newline();
                }
                Block::Paragraph { content } => {
                    if content.is_empty() {
                        continue;
                    }
                    self.emit_inlines(content)?;
                    self.push_newline();
                }
                Block::CodeBlock { language, content } => {
                    self.push("```");
                    if let Some(lang) = language {
                        self.push(lang);
                    }
                    self.push_newline();
                    self.push(content);
                    self.push_newline();
                    self.push("```\n");
                }
                Block::BlockQuote { children } => {
                    self.push("#quote[");
                    self.push_newline();
                    let inner = emit_blocks_to_string(children)?;
                    for line in inner.lines() {
                        self.push("  ");
                        self.push(line);
                        self.push_newline();
                    }
                    self.push("]\n");
                }
                Block::UnorderedList { items } => {
                    self.emit_unordered_list(items)?;
                }
                Block::OrderedList { items, .. } => {
                    self.emit_ordered_list(items)?;
                }
                Block::DescriptionList { items } => {
                    for item in items {
                        self.push("/ ");
                        self.emit_inlines(&item.term)?;
                        self.push(": ");
                        for def_blocks in &item.definitions {
                            for def_block in def_blocks {
                                if let Block::Paragraph { content } = def_block {
                                    self.emit_inlines(content)?;
                                }
                            }
                        }
                        self.push_newline();
                    }
                }
                Block::Table { headers, rows, .. } => {
                    self.emit_table(headers, rows)?;
                }
                Block::HorizontalRule => {
                    self.push("#line(length: 100%)\n");
                }
                Block::RawBlock { content, .. } => {
                    self.push(content);
                    self.push_newline();
                }
            }
        }
        Ok(())
    }

    fn emit_unordered_list(&mut self, items: &[ListItem]) -> Result<(), EmitError> {
        for item in items {
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() {
                            continue;
                        }
                        if j == 0 {
                            self.push("- ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        // Indent nested list
                        let inner = emit_unordered_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("  ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        let inner = emit_ordered_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("  ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    _ => {
                        self.emit_blocks(std::slice::from_ref(block))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_ordered_list(&mut self, items: &[ListItem]) -> Result<(), EmitError> {
        for item in items {
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() {
                            continue;
                        }
                        if j == 0 {
                            self.push("+ ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        let inner = emit_unordered_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("  ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        let inner = emit_ordered_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("  ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    _ => {
                        self.emit_blocks(std::slice::from_ref(block))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        let num_cols = headers.len();
        self.push(&format!("#table(\n  columns: {num_cols},\n"));
        for header in headers {
            self.emit_table_cell(header)?;
        }
        for row in rows {
            for cell in row {
                self.emit_table_cell(cell)?;
            }
        }
        self.push(")\n");
        Ok(())
    }

    fn emit_table_cell(&mut self, cell: &TableCell) -> Result<(), EmitError> {
        if cell.has_span() {
            self.push("  table.cell(");
            let mut parts = Vec::new();
            if cell.colspan > 1 {
                parts.push(format!("colspan: {}", cell.colspan));
            }
            if cell.rowspan > 1 {
                parts.push(format!("rowspan: {}", cell.rowspan));
            }
            self.push(&parts.join(", "));
            self.push(")[");
            self.emit_inlines(&cell.content)?;
            self.push("],\n");
        } else {
            self.push("  [");
            self.emit_inlines(&cell.content)?;
            self.push("],\n");
        }
        Ok(())
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) -> Result<(), EmitError> {
        for inline in inlines {
            self.emit_inline(inline)?;
        }
        Ok(())
    }

    fn emit_inline(&mut self, inline: &Inline) -> Result<(), EmitError> {
        match inline {
            Inline::Text(t) => self.push(t),
            Inline::Bold(content) => {
                self.push("*");
                self.emit_inlines(content)?;
                self.push("*");
            }
            Inline::Italic(content) => {
                self.push("_");
                self.emit_inlines(content)?;
                self.push("_");
            }
            Inline::BoldItalic(content) => {
                self.push("*_");
                self.emit_inlines(content)?;
                self.push("_*");
            }
            Inline::Strikethrough(content) => {
                self.push("#strike[");
                self.emit_inlines(content)?;
                self.push("]");
            }
            Inline::Superscript(content) => {
                self.push("#super[");
                self.emit_inlines(content)?;
                self.push("]");
            }
            Inline::Subscript(content) => {
                self.push("#sub[");
                self.emit_inlines(content)?;
                self.push("]");
            }
            Inline::InlineCode(code) => {
                self.push("`");
                self.push(code);
                self.push("`");
            }
            Inline::Link { url, text, .. } => {
                let text_str = inlines_to_string(text);
                if text_str == *url {
                    self.push(&format!("#link(\"{url}\")"));
                } else {
                    self.push(&format!("#link(\"{url}\")["));
                    self.emit_inlines(text)?;
                    self.push("]");
                }
            }
            Inline::Image { url, .. } => {
                self.push(&format!("#image(\"{url}\")"));
            }
            Inline::HardLineBreak => {
                self.push(" \\\n");
            }
            Inline::SoftLineBreak => {
                self.push_newline();
            }
            Inline::RawInline { content, .. } => {
                self.push(content);
            }
        }
        Ok(())
    }
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut ctx = TypstEmitContext::new();
    let _ = ctx.emit_inlines(inlines);
    ctx.output
}

fn emit_blocks_to_string(blocks: &[Block]) -> Result<String, EmitError> {
    let mut ctx = TypstEmitContext::new();
    ctx.emit_blocks(blocks)?;
    Ok(ctx.finish())
}

fn emit_unordered_list_to_string(items: &[ListItem]) -> Result<String, EmitError> {
    let mut ctx = TypstEmitContext::new();
    ctx.emit_unordered_list(items)?;
    Ok(ctx.output)
}

fn emit_ordered_list_to_string(items: &[ListItem]) -> Result<String, EmitError> {
    let mut ctx = TypstEmitContext::new();
    ctx.emit_ordered_list(items)?;
    Ok(ctx.output)
}
