use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;

pub struct MarkdownEmitter;

impl Emitter for MarkdownEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut ctx = MdEmitContext::new();
        ctx.emit_blocks(&doc.children)?;
        Ok(ctx.finish())
    }
}

struct MdEmitContext {
    output: String,
    list_indent: String,
}

impl MdEmitContext {
    fn new() -> Self {
        MdEmitContext {
            output: String::new(),
            list_indent: String::new(),
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
                    let hashes: String = "#".repeat(*level as usize);
                    self.push(&hashes);
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
                    let inner = emit_blocks_to_string(children)?;
                    for line in inner.lines() {
                        if line.is_empty() {
                            self.push(">\n");
                        } else {
                            self.push("> ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                }
                Block::UnorderedList { items } => {
                    self.emit_unordered_list(items)?;
                }
                Block::OrderedList { start, items } => {
                    self.emit_ordered_list(*start, items)?;
                }
                Block::DescriptionList { items } => {
                    for (j, item) in items.iter().enumerate() {
                        if j > 0 {
                            self.push_newline();
                        }
                        self.emit_inlines(&item.term)?;
                        self.push_newline();
                        for def_blocks in &item.definitions {
                            for def_block in def_blocks {
                                if let Block::Paragraph { content } = def_block {
                                    self.push(":   ");
                                    self.emit_inlines(content)?;
                                    self.push_newline();
                                }
                            }
                        }
                    }
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => {
                    self.emit_table(headers, alignments, rows)?;
                }
                Block::HorizontalRule => {
                    self.push("---\n");
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
        let outer_indent = self.list_indent.clone();
        // "- " is 2 chars, so sub-items indent by 2 spaces
        let inner_indent = format!("{outer_indent}  ");

        for item in items {
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() && j > 0 {
                            continue;
                        }
                        if j == 0 {
                            self.push(&outer_indent);
                            self.push("- ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        self.list_indent = inner_indent.clone();
                        self.emit_unordered_list(sub_items)?;
                        self.list_indent = outer_indent.clone();
                    }
                    Block::OrderedList {
                        start,
                        items: sub_items,
                    } => {
                        self.list_indent = inner_indent.clone();
                        self.emit_ordered_list(*start, sub_items)?;
                        self.list_indent = outer_indent.clone();
                    }
                    _ => {
                        self.emit_blocks(std::slice::from_ref(block))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn emit_ordered_list(&mut self, start: u32, items: &[ListItem]) -> Result<(), EmitError> {
        let outer_indent = self.list_indent.clone();
        // "1. " is 3 chars, so sub-items indent by 3 spaces
        let inner_indent = format!("{outer_indent}   ");

        for (idx, item) in items.iter().enumerate() {
            let num = start + idx as u32;
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() && j > 0 {
                            continue;
                        }
                        if j == 0 {
                            self.push(&outer_indent);
                            self.push(&format!("{num}. "));
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        self.list_indent = inner_indent.clone();
                        self.emit_unordered_list(sub_items)?;
                        self.list_indent = outer_indent.clone();
                    }
                    Block::OrderedList {
                        start: sub_start,
                        items: sub_items,
                    } => {
                        self.list_indent = inner_indent.clone();
                        self.emit_ordered_list(*sub_start, sub_items)?;
                        self.list_indent = outer_indent.clone();
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
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        let num_cols = alignments.len();
        let has_spans = headers.iter().any(|c| c.has_span())
            || rows.iter().any(|r| r.iter().any(|c| c.has_span()));

        if has_spans {
            self.emit_table_with_spans(headers, alignments, rows, num_cols)
        } else {
            self.emit_table_simple(headers, alignments, rows)
        }
    }

    fn emit_table_simple(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        // Header row
        self.push("|");
        for header in headers {
            self.push(" ");
            self.emit_inlines(&header.content)?;
            self.push(" |");
        }
        self.push_newline();

        // Separator row
        self.push("|");
        for (i, _) in headers.iter().enumerate() {
            let align = alignments.get(i).unwrap_or(&ColumnAlignment::Default);
            match align {
                ColumnAlignment::Left => self.push(" :--- "),
                ColumnAlignment::Center => self.push(" :---: "),
                ColumnAlignment::Right => self.push(" ---: "),
                ColumnAlignment::Default => self.push(" --- "),
            }
            self.push("|");
        }
        self.push_newline();

        // Data rows
        for row in rows {
            self.push("|");
            for cell in row {
                self.push(" ");
                self.emit_inlines(&cell.content)?;
                self.push(" |");
            }
            self.push_newline();
        }
        Ok(())
    }

    fn emit_table_with_spans(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
        num_cols: usize,
    ) -> Result<(), EmitError> {
        // Build an occupancy grid to track cells covered by rowspans
        // total_rows = 1 (header) + rows.len()
        let total_rows = 1 + rows.len();
        // occupied[row][col] = true if already claimed by a spanning cell above
        let mut occupied = vec![vec![false; num_cols]; total_rows];

        // Expand a logical row (cells with spans) into a flat grid row
        // Returns a vec of Option<&[Inline]> for each column
        let expand_row = |cells: &[TableCell],
                          row_idx: usize,
                          occupied: &mut Vec<Vec<bool>>|
         -> Vec<Option<Vec<Inline>>> {
            let mut result = vec![None; num_cols];
            let mut cell_idx = 0;
            for col in 0..num_cols {
                if occupied[row_idx][col] {
                    // Already claimed by a rowspan from above
                    result[col] = Some(vec![]);
                    continue;
                }
                if cell_idx >= cells.len() {
                    break;
                }
                let cell = &cells[cell_idx];
                result[col] = Some(cell.content.clone());
                // Mark extra colspan columns as empty
                for c in 1..cell.colspan as usize {
                    if col + c < num_cols {
                        result[col + c] = Some(vec![]);
                    }
                }
                // Mark rowspan cells in subsequent rows as occupied
                for r in 1..cell.rowspan as usize {
                    if row_idx + r < occupied.len() {
                        for c in 0..cell.colspan as usize {
                            if col + c < num_cols {
                                occupied[row_idx + r][col + c] = true;
                            }
                        }
                    }
                }
                cell_idx += 1;
            }
            result
        };

        // Header row
        let header_cells = expand_row(headers, 0, &mut occupied);
        self.push("|");
        for cell_content in &header_cells {
            self.push(" ");
            if let Some(content) = cell_content {
                self.emit_inlines(content)?;
            }
            self.push(" |");
        }
        self.push_newline();

        // Separator row
        self.push("|");
        for i in 0..num_cols {
            let align = alignments.get(i).unwrap_or(&ColumnAlignment::Default);
            match align {
                ColumnAlignment::Left => self.push(" :--- "),
                ColumnAlignment::Center => self.push(" :---: "),
                ColumnAlignment::Right => self.push(" ---: "),
                ColumnAlignment::Default => self.push(" --- "),
            }
            self.push("|");
        }
        self.push_newline();

        // Data rows
        for (row_idx, row) in rows.iter().enumerate() {
            let row_cells = expand_row(row, row_idx + 1, &mut occupied);
            self.push("|");
            for cell_content in &row_cells {
                self.push(" ");
                if let Some(content) = cell_content {
                    self.emit_inlines(content)?;
                }
                self.push(" |");
            }
            self.push_newline();
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
                self.push("**");
                self.emit_inlines(content)?;
                self.push("**");
            }
            Inline::Italic(content) => {
                self.push("*");
                self.emit_inlines(content)?;
                self.push("*");
            }
            Inline::BoldItalic(content) => {
                self.push("***");
                self.emit_inlines(content)?;
                self.push("***");
            }
            Inline::Strikethrough(content) => {
                self.push("~~");
                self.emit_inlines(content)?;
                self.push("~~");
            }
            Inline::Superscript(content) => {
                self.push("^");
                self.emit_inlines(content)?;
                self.push("^");
            }
            Inline::Subscript(content) => {
                self.push("~");
                self.emit_inlines(content)?;
                self.push("~");
            }
            Inline::InlineCode(code) => {
                if code.contains('`') {
                    self.push("`` ");
                    self.push(code);
                    self.push(" ``");
                } else {
                    self.push("`");
                    self.push(code);
                    self.push("`");
                }
            }
            Inline::Link { url, text, title } => {
                self.push("[");
                self.emit_inlines(text)?;
                self.push("](");
                self.push(url);
                if let Some(t) = title {
                    self.push(" \"");
                    self.push(t);
                    self.push("\"");
                }
                self.push(")");
            }
            Inline::Image {
                url,
                alt,
                title,
                link,
            } => {
                if let Some(link_url) = link {
                    self.push("[![");
                    self.emit_inlines(alt)?;
                    self.push("](");
                    self.push(url);
                    if let Some(t) = title {
                        self.push(" \"");
                        self.push(t);
                        self.push("\"");
                    }
                    self.push(")](");
                    self.push(link_url);
                    self.push(")");
                } else {
                    self.push("![");
                    self.emit_inlines(alt)?;
                    self.push("](");
                    self.push(url);
                    if let Some(t) = title {
                        self.push(" \"");
                        self.push(t);
                        self.push("\"");
                    }
                    self.push(")");
                }
            }
            Inline::HardLineBreak => {
                self.push("  \n");
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

fn emit_blocks_to_string(blocks: &[Block]) -> Result<String, EmitError> {
    let mut ctx = MdEmitContext::new();
    ctx.emit_blocks(blocks)?;
    Ok(ctx.finish())
}
