use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;

pub struct RstEmitter;

impl Emitter for RstEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut ctx = RstEmitContext::new();
        ctx.emit_blocks(&doc.children)?;
        Ok(ctx.finish())
    }
}

// Conventional underline characters for RST heading levels
const HEADING_UNDERLINES: &[char] = &['=', '-', '~', '"', '^', '\''];

struct RstEmitContext {
    output: String,
}

impl RstEmitContext {
    fn new() -> Self {
        RstEmitContext {
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
                    let text = inlines_to_string(content);
                    let underline_char = HEADING_UNDERLINES
                        .get((*level as usize).saturating_sub(1))
                        .copied()
                        .unwrap_or('=');
                    let underline: String =
                        std::iter::repeat_n(underline_char, text.len().max(3)).collect();
                    self.push(&text);
                    self.push_newline();
                    self.push(&underline);
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
                    if let Some(lang) = language {
                        self.push(&format!(".. code-block:: {lang}"));
                    } else {
                        self.push(".. code-block::");
                    }
                    self.push_newline();
                    self.push_newline();
                    for line in content.lines() {
                        self.push("   ");
                        self.push(line);
                        self.push_newline();
                    }
                }
                Block::BlockQuote { children } => {
                    let inner = emit_blocks_to_string(children)?;
                    for line in inner.lines() {
                        if line.is_empty() {
                            self.push_newline();
                        } else {
                            self.push("   ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                }
                Block::UnorderedList { items } => {
                    self.emit_bullet_list(items)?;
                }
                Block::OrderedList { items, .. } => {
                    self.emit_enum_list(items)?;
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
                                    self.push("   ");
                                    self.emit_inlines(content)?;
                                    self.push_newline();
                                }
                            }
                        }
                    }
                }
                Block::Table {
                    headers,
                    rows,
                    alignments,
                } => {
                    if table_has_spans(headers, rows) {
                        self.emit_grid_table(headers, rows, alignments.len())?;
                    } else {
                        self.emit_simple_table(headers, rows)?;
                    }
                }
                Block::HorizontalRule => {
                    self.push("----\n");
                }
                Block::RawBlock { content, .. } => {
                    self.push(content);
                    self.push_newline();
                }
            }
        }
        Ok(())
    }

    fn emit_bullet_list(&mut self, items: &[ListItem]) -> Result<(), EmitError> {
        for item in items {
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() {
                            continue;
                        }
                        if j == 0 {
                            self.push("* ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        } else {
                            self.push("  ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        // Indent sub-list
                        let inner = emit_bullet_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("  ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        let inner = emit_enum_list_to_string(sub_items)?;
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

    fn emit_enum_list(&mut self, items: &[ListItem]) -> Result<(), EmitError> {
        for item in items {
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() {
                            continue;
                        }
                        if j == 0 {
                            self.push("#. ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        } else {
                            self.push("   ");
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        let inner = emit_bullet_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("   ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        let inner = emit_enum_list_to_string(sub_items)?;
                        for line in inner.lines() {
                            self.push("   ");
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

    fn emit_simple_table(
        &mut self,
        headers: &[TableCell],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        // Calculate column widths
        let mut col_widths: Vec<usize> = headers
            .iter()
            .map(|h| inlines_to_string(&h.content).len().max(3))
            .collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    let w = inlines_to_string(&cell.content).len();
                    if w > col_widths[i] {
                        col_widths[i] = w;
                    }
                }
            }
        }

        // Top border
        let border: String = col_widths
            .iter()
            .map(|&w| "=".repeat(w))
            .collect::<Vec<_>>()
            .join("  ");
        self.push(&border);
        self.push_newline();

        // Header row
        let header_strs: Vec<String> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let s = inlines_to_string(&h.content);
                format!(
                    "{:<width$}",
                    s,
                    width = col_widths.get(i).copied().unwrap_or(3)
                )
            })
            .collect();
        self.push(&header_strs.join("  "));
        self.push_newline();

        // Header separator
        self.push(&border);
        self.push_newline();

        // Data rows
        for row in rows {
            let row_strs: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let s = inlines_to_string(&cell.content);
                    format!(
                        "{:<width$}",
                        s,
                        width = col_widths.get(i).copied().unwrap_or(3)
                    )
                })
                .collect();
            self.push(&row_strs.join("  "));
            self.push_newline();
        }

        // Bottom border
        self.push(&border);
        self.push_newline();
        Ok(())
    }

    fn emit_grid_table(
        &mut self,
        headers: &[TableCell],
        rows: &[Vec<TableCell>],
        num_cols: usize,
    ) -> Result<(), EmitError> {
        // Build a flat grid: each cell knows its content, colspan, rowspan
        // all_rows[0] = headers, rest = data rows
        let total_rows = 1 + rows.len();
        let mut grid: Vec<Vec<(String, u32, u32)>> = Vec::new(); // (content, colspan, rowspan)

        // Header row
        let mut header_flat = Vec::new();
        for cell in headers {
            header_flat.push((inlines_to_string(&cell.content), cell.colspan, cell.rowspan));
        }
        grid.push(header_flat);

        // Data rows
        for row in rows {
            let mut row_flat = Vec::new();
            for cell in row {
                row_flat.push((inlines_to_string(&cell.content), cell.colspan, cell.rowspan));
            }
            grid.push(row_flat);
        }

        // Calculate column widths (minimum 3 chars for content)
        let mut col_widths = vec![3usize; num_cols];
        // Build occupancy grid to know which logical cell owns each position
        let mut occupied = vec![vec![false; num_cols]; total_rows];

        for (row_idx, row) in grid.iter().enumerate() {
            let mut col = 0;
            for (content, cspan, rspan) in row {
                while col < num_cols && occupied[row_idx][col] {
                    col += 1;
                }
                if col >= num_cols {
                    break;
                }
                let cs = *cspan as usize;
                let rs = *rspan as usize;

                // Calculate width needed: content must fit in the combined columns
                // Combined width = sum of col_widths for spanned cols + (cs-1)*3 for separators
                let min_width = content.len().max(1);
                if cs == 1 {
                    if min_width > col_widths[col] {
                        col_widths[col] = min_width;
                    }
                } else {
                    // Distribute among spanned columns
                    let current_total: usize = (0..cs)
                        .filter_map(|c| col_widths.get(col + c))
                        .sum::<usize>()
                        + (cs - 1) * 3;
                    if min_width > current_total {
                        let extra = min_width - current_total;
                        if col < col_widths.len() {
                            col_widths[col] += extra;
                        }
                    }
                }

                // Mark occupied
                for r in 0..rs {
                    for c in 0..cs {
                        if row_idx + r < total_rows && col + c < num_cols && (r > 0 || c > 0) {
                            occupied[row_idx + r][col + c] = true;
                        }
                    }
                }
                col += cs;
            }
        }

        // Rebuild occupancy for rendering
        let mut cell_map: Vec<Vec<Option<(String, u32, u32)>>> =
            vec![vec![None; num_cols]; total_rows];
        let mut occupied2 = vec![vec![false; num_cols]; total_rows];
        for (row_idx, row) in grid.iter().enumerate() {
            let mut col = 0;
            for (content, cspan, rspan) in row {
                while col < num_cols && occupied2[row_idx][col] {
                    col += 1;
                }
                if col >= num_cols {
                    break;
                }
                let cs = *cspan as usize;
                let rs = *rspan as usize;
                cell_map[row_idx][col] = Some((content.clone(), *cspan, *rspan));
                for r in 0..rs {
                    for c in 0..cs {
                        if row_idx + r < total_rows && col + c < num_cols && (r > 0 || c > 0) {
                            occupied2[row_idx + r][col + c] = true;
                        }
                    }
                }
                col += cs;
            }
        }

        // Emit the grid table
        // Top border
        self.push(&grid_separator(
            &col_widths,
            &cell_map,
            &occupied2,
            0,
            '-',
            true,
        ));
        self.push_newline();

        for row_idx in 0..total_rows {
            // Content line
            self.push("|");
            let mut col = 0;
            while col < num_cols {
                if let Some((ref content, cspan, _rspan)) = cell_map[row_idx][col] {
                    let cs = cspan as usize;
                    let total_width: usize =
                        (0..cs).map(|c| col_widths[col + c]).sum::<usize>() + (cs - 1) * 3;
                    self.push(&format!(" {:<width$} ", content, width = total_width));
                    col += cs;
                } else if occupied2[row_idx][col] {
                    // Part of a spanning cell - find the owner
                    let total_width = col_widths[col];
                    self.push(&format!(" {:<width$} ", "", width = total_width));
                    col += 1;
                } else {
                    let total_width = col_widths[col];
                    self.push(&format!(" {:<width$} ", "", width = total_width));
                    col += 1;
                }
                if col < num_cols {
                    // Check if next column is part of a colspan from this cell
                    if occupied2[row_idx][col] {
                        // Check if the cell that owns this position spans across this boundary
                        // Look back to find which cell owns col
                        let mut owner_col = col;
                        while owner_col > 0 {
                            owner_col -= 1;
                            if let Some((_, cs, _)) = cell_map[row_idx][owner_col]
                                && owner_col + cs as usize > col
                            {
                                // This cell spans across, don't print |
                                break;
                            }
                            if !occupied2[row_idx][owner_col] {
                                break;
                            }
                        }
                        // Check if we should skip the |
                        let skip = if let Some((_, cs, _)) = cell_map[row_idx][owner_col] {
                            owner_col + cs as usize > col
                        } else {
                            false
                        };
                        if !skip {
                            self.push("|");
                        }
                    } else {
                        self.push("|");
                    }
                }
            }
            self.push("|\n");

            // Row separator
            if row_idx == 0 && !rows.is_empty() {
                // Header separator uses =
                self.push(&grid_separator(
                    &col_widths,
                    &cell_map,
                    &occupied2,
                    row_idx + 1,
                    '=',
                    false,
                ));
            } else if row_idx < total_rows - 1 {
                self.push(&grid_separator(
                    &col_widths,
                    &cell_map,
                    &occupied2,
                    row_idx + 1,
                    '-',
                    false,
                ));
            } else {
                // Bottom border
                self.push(&grid_separator(
                    &col_widths,
                    &cell_map,
                    &occupied2,
                    row_idx + 1,
                    '-',
                    true,
                ));
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
                // RST doesn't have native bold-italic; use bold wrapping italic
                self.push("**");
                self.push("*");
                self.emit_inlines(content)?;
                self.push("*");
                self.push("**");
            }
            Inline::Strikethrough(content) => {
                // RST has no native strikethrough; emit as plain text
                self.push("~~");
                self.emit_inlines(content)?;
                self.push("~~");
            }
            Inline::Superscript(content) => {
                self.push("\\ :sup:`");
                self.emit_inlines(content)?;
                self.push("`\\ ");
            }
            Inline::Subscript(content) => {
                self.push("\\ :sub:`");
                self.emit_inlines(content)?;
                self.push("`\\ ");
            }
            Inline::InlineCode(code) => {
                self.push("``");
                self.push(code);
                self.push("``");
            }
            Inline::Link { url, text, .. } => {
                let text_str = inlines_to_string(text);
                if text_str == *url {
                    self.push(url);
                } else {
                    self.push("`");
                    self.push(&text_str);
                    self.push(" <");
                    self.push(url);
                    self.push(">`_");
                }
            }
            Inline::Image { url: _, alt, .. } => {
                // Inline image reference - emit as directive (block-level)
                // For inline context, use substitution reference or just |image|
                let alt_text = inlines_to_string(alt);
                self.push(&format!("|{alt_text}|"));
                // Note: actual image directive should be at block level
                // This is a simplification for inline context
            }
            Inline::HardLineBreak => {
                self.push_newline();
                self.push("| ");
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

/// Generate a grid table separator line.
/// `next_row` is the row index below the separator (used to check rowspans).
/// `is_border` true for top/bottom borders (always draw full separators).
fn grid_separator(
    col_widths: &[usize],
    cell_map: &[Vec<Option<(String, u32, u32)>>],
    occupied: &[Vec<bool>],
    next_row: usize,
    fill: char,
    is_border: bool,
) -> String {
    let num_cols = col_widths.len();
    let total_rows = cell_map.len();
    let mut result = String::new();
    result.push('+');
    for col in 0..num_cols {
        // Check if a rowspan from above crosses this separator
        let has_rowspan_across =
            !is_border && next_row < total_rows && occupied[next_row][col] && {
                // Find owner cell
                let mut owner_found = false;
                for r in (0..next_row).rev() {
                    if let Some((_, _cs, rs)) = &cell_map[r][col] {
                        if r + *rs as usize > next_row {
                            owner_found = true;
                        }
                        break;
                    }
                    if !occupied[r][col] {
                        break;
                    }
                }
                owner_found
            };

        if has_rowspan_across {
            // Replace dashes with spaces (no separator here)
            let w = col_widths[col] + 2;
            for _ in 0..w {
                result.push(' ');
            }
        } else {
            let w = col_widths[col] + 2;
            for _ in 0..w {
                result.push(fill);
            }
        }

        // Check if we need + or not between columns
        if col + 1 < num_cols {
            // Check if a colspan spans this boundary in the row above or below
            let colspan_above = if next_row > 0 && next_row <= total_rows {
                let row_above = next_row - 1;
                // Find if a cell with colspan spans col and col+1
                let mut spans = false;
                for start_col in 0..=col {
                    if let Some((_, cs, _)) = &cell_map[row_above][start_col]
                        && start_col + *cs as usize > col + 1
                    {
                        spans = true;
                        break;
                    }
                }
                spans
            } else {
                false
            };

            let colspan_below = if !is_border && next_row < total_rows {
                let mut spans = false;
                for start_col in 0..=col {
                    if let Some((_, cs, _)) = &cell_map[next_row][start_col]
                        && start_col + *cs as usize > col + 1
                    {
                        spans = true;
                        break;
                    }
                }
                spans
            } else {
                false
            };

            if colspan_above && colspan_below && !is_border {
                result.push(fill);
            } else {
                result.push('+');
            }
        }
    }
    result.push('+');
    result
}

fn table_has_spans(headers: &[TableCell], rows: &[Vec<TableCell>]) -> bool {
    headers.iter().any(|c| c.has_span()) || rows.iter().any(|r| r.iter().any(|c| c.has_span()))
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut ctx = RstEmitContext::new();
    let _ = ctx.emit_inlines(inlines);
    ctx.output
}

fn emit_blocks_to_string(blocks: &[Block]) -> Result<String, EmitError> {
    let mut ctx = RstEmitContext::new();
    ctx.emit_blocks(blocks)?;
    Ok(ctx.finish())
}

fn emit_bullet_list_to_string(items: &[ListItem]) -> Result<String, EmitError> {
    let mut ctx = RstEmitContext::new();
    ctx.emit_bullet_list(items)?;
    Ok(ctx.output)
}

fn emit_enum_list_to_string(items: &[ListItem]) -> Result<String, EmitError> {
    let mut ctx = RstEmitContext::new();
    ctx.emit_enum_list(items)?;
    Ok(ctx.output)
}
