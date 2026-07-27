use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;

pub struct RstParser;

impl Parser for RstParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let mut state = RstParserState::new(input);
        let children = state.parse_blocks()?;
        Ok(Document { children })
    }
}

// RST heading underline characters by convention (in order of precedence)
const HEADING_CHARS: &[char] = &['=', '-', '~', '"', '^', '\''];

struct RstParserState {
    lines: Vec<String>,
    pos: usize,
    heading_chars_seen: Vec<char>,
}

impl RstParserState {
    fn new(input: &str) -> Self {
        RstParserState {
            lines: input.lines().map(|l| l.to_string()).collect(),
            pos: 0,
            heading_chars_seen: Vec::new(),
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.lines.len()
    }

    fn current_line(&self) -> &str {
        &self.lines[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn is_adornment_line(line: &str) -> Option<char> {
        let trimmed = line.trim_end();
        if trimmed.len() < 3 {
            return None;
        }
        let ch = trimmed.chars().next()?;
        if HEADING_CHARS.contains(&ch) && trimmed.chars().all(|c| c == ch) {
            Some(ch)
        } else {
            None
        }
    }

    fn heading_level_for_char(&mut self, ch: char) -> u8 {
        if let Some(pos) = self.heading_chars_seen.iter().position(|&c| c == ch) {
            (pos + 1) as u8
        } else {
            self.heading_chars_seen.push(ch);
            self.heading_chars_seen.len() as u8
        }
    }

    fn parse_blocks(&mut self) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();

            if line.trim().is_empty() {
                self.advance();
                continue;
            }

            // Transition / horizontal rule: ---- (4+ chars, standalone)
            if self.is_transition(&line) {
                blocks.push(Block::HorizontalRule);
                self.advance();
                continue;
            }

            // Heading: text followed by adornment, or overline + text + underline
            if let Some(block) = self.try_parse_heading() {
                blocks.push(block);
                continue;
            }

            // Directive: .. directive::
            if line.trim_start().starts_with(".. ")
                && let Some(block) = self.try_parse_directive()?
            {
                blocks.push(block);
                continue;
            }

            // Blockquote: indented text (not in a list or directive context)
            if (line.starts_with("   ") || line.starts_with('\t')) && !line.trim().is_empty() {
                blocks.push(self.parse_blockquote()?);
                continue;
            }

            // Bullet list
            if self.is_bullet_list_start(&line) {
                blocks.push(self.parse_bullet_list()?);
                continue;
            }

            // Enumerated list
            if self.is_enum_list_start(&line) {
                blocks.push(self.parse_enum_list()?);
                continue;
            }

            // Field list (used for description lists)
            if line.trim_start().starts_with(':') && line.contains(":`") {
                // Not a standard description list in RST
            }

            // Simple table
            if self.is_simple_table_start(&line) {
                blocks.push(self.parse_simple_table()?);
                continue;
            }

            // Grid table
            if line.trim().starts_with('+')
                && line.contains('-')
                && let Some(block) = self.try_parse_grid_table()?
            {
                blocks.push(block);
                continue;
            }

            blocks.push(self.parse_paragraph()?);
        }
        Ok(blocks)
    }

    fn is_transition(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 4 {
            return false;
        }
        let ch = trimmed.chars().next().unwrap();
        (ch == '-' || ch == '=' || ch == '_' || ch == '*' || ch == '+' || ch == '#')
            && trimmed.chars().all(|c| c == ch)
            // A transition must be preceded and followed by blank lines (or be at start/end)
            && (self.pos == 0 || self.pos > 0 && self.lines.get(self.pos - 1).map(|l| l.trim().is_empty()).unwrap_or(true))
    }

    fn try_parse_heading(&mut self) -> Option<Block> {
        let line = self.current_line().to_string();
        let trimmed = line.trim();

        // Check for overline style: adornment + text + adornment
        if let Some(ch) = Self::is_adornment_line(trimmed)
            && self.pos + 2 < self.lines.len()
        {
            let text_line = self.lines[self.pos + 1].trim().to_string();
            let under_line = self.lines[self.pos + 2].trim().to_string();
            if !text_line.is_empty() && Self::is_adornment_line(&under_line) == Some(ch) {
                let level = self.heading_level_for_char(ch);
                self.pos += 3;
                return Some(Block::Heading {
                    level,
                    content: parse_rst_inlines(&text_line),
                });
            }
        }

        // Underline style: text + adornment
        if self.pos + 1 < self.lines.len() && !trimmed.is_empty() {
            let next = self.lines[self.pos + 1].trim().to_string();
            if let Some(ch) = Self::is_adornment_line(&next)
                && next.len() >= trimmed.len()
            {
                let level = self.heading_level_for_char(ch);
                self.pos += 2;
                return Some(Block::Heading {
                    level,
                    content: parse_rst_inlines(trimmed),
                });
            }
        }

        None
    }

    fn try_parse_directive(&mut self) -> Result<Option<Block>, ParseError> {
        let line = self.current_line().trim().to_string();
        // .. code-block:: lang or .. code:: lang
        if line.starts_with(".. code-block::") || line.starts_with(".. code::") {
            let lang_part = if let Some(rest) = line.strip_prefix(".. code-block::") {
                rest
            } else {
                &line[".. code::".len()..]
            };
            let language = {
                let l = lang_part.trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            };
            self.advance();

            // Skip blank lines and options
            while !self.at_end() {
                let l = self.current_line().to_string();
                if l.trim().is_empty() {
                    self.advance();
                    continue;
                }
                // Skip option lines like :linenos:
                if l.trim().starts_with(':') && l.trim().ends_with(':') {
                    self.advance();
                    continue;
                }
                break;
            }

            // Collect indented content
            let mut content_lines = Vec::new();
            while !self.at_end() {
                let l = self.current_line().to_string();
                if l.trim().is_empty() {
                    // Could be blank line within code block
                    if self.pos + 1 < self.lines.len() {
                        let next = &self.lines[self.pos + 1];
                        if next.starts_with("   ")
                            || next.starts_with('\t')
                            || next.trim().is_empty()
                        {
                            content_lines.push(String::new());
                            self.advance();
                            continue;
                        }
                    }
                    break;
                }
                if l.starts_with("   ") || l.starts_with('\t') {
                    // Remove 3-space indent
                    let stripped = if let Some(rest) = l.strip_prefix("   ") {
                        rest
                    } else {
                        l.trim_start()
                    };
                    content_lines.push(stripped.to_string());
                    self.advance();
                } else {
                    break;
                }
            }
            // Remove trailing empty lines
            while content_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
                content_lines.pop();
            }

            return Ok(Some(Block::CodeBlock {
                language,
                content: content_lines.join("\n"),
            }));
        }

        // .. image:: url
        if let Some(rest) = line.strip_prefix(".. image::") {
            let url = rest.trim().to_string();
            self.advance();
            // Skip options like :alt:, :width: etc.
            let mut alt_text = String::new();
            while !self.at_end() {
                let l = self.current_line().to_string();
                if l.trim().is_empty() {
                    break;
                }
                if l.starts_with("   ") || l.starts_with('\t') {
                    let opt = l.trim();
                    if let Some(rest) = opt.strip_prefix(":alt:") {
                        alt_text = rest.trim().to_string();
                    }
                    self.advance();
                } else {
                    break;
                }
            }
            return Ok(Some(Block::Paragraph {
                content: vec![Inline::Image {
                    url,
                    alt: if alt_text.is_empty() {
                        vec![]
                    } else {
                        vec![Inline::Text(alt_text)]
                    },
                    title: None,
                    link: None,
                }],
            }));
        }

        // Unknown directive - treat as raw block
        let directive_line = self.current_line().to_string();
        self.advance();
        let mut content_lines = vec![directive_line];
        while !self.at_end() {
            let l = self.current_line().to_string();
            if l.trim().is_empty() {
                break;
            }
            if l.starts_with("   ") || l.starts_with('\t') {
                content_lines.push(l);
                self.advance();
            } else {
                break;
            }
        }
        Ok(Some(Block::RawBlock {
            format: Some("rst".to_string()),
            content: content_lines.join("\n"),
        }))
    }

    fn parse_blockquote(&mut self) -> Result<Block, ParseError> {
        let mut inner_lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                // Check if next line is also indented
                if self.pos + 1 < self.lines.len() {
                    let next = &self.lines[self.pos + 1];
                    if next.starts_with("   ") || next.starts_with('\t') {
                        inner_lines.push(String::new());
                        self.advance();
                        continue;
                    }
                }
                break;
            }
            if line.starts_with("   ") || line.starts_with('\t') {
                let stripped = if line.starts_with("   ") {
                    &line[3..]
                } else {
                    line.trim_start()
                };
                inner_lines.push(stripped.to_string());
                self.advance();
            } else {
                break;
            }
        }
        let inner_text = inner_lines.join("\n");
        let mut inner_parser = RstParserState::new(&inner_text);
        let children = inner_parser.parse_blocks()?;
        Ok(Block::BlockQuote { children })
    }

    fn is_bullet_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        (trimmed.starts_with("* ") || trimmed.starts_with("- ") || trimmed.starts_with("+ "))
            && !self.is_transition(line)
    }

    fn parse_bullet_list(&mut self) -> Result<Block, ParseError> {
        let base_indent = self.current_line().len() - self.current_line().trim_start().len();
        self.parse_bullet_list_at_indent(base_indent)
    }

    fn parse_bullet_list_at_indent(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                // Check if list continues at our indent level
                let saved = self.pos;
                self.advance();
                while !self.at_end() && self.current_line().trim().is_empty() {
                    self.advance();
                }
                if !self.at_end() {
                    let next_indent =
                        self.current_line().len() - self.current_line().trim_start().len();
                    if next_indent >= base_indent && self.is_bullet_list_start(self.current_line())
                    {
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let indent = line.len() - line.trim_start().len();

            // Deeper indented bullet list -> sub-list
            if indent > base_indent && self.is_bullet_list_start(&line) {
                let sub_list = self.parse_bullet_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Deeper indented enum list -> sub-list
            if indent > base_indent && self.is_enum_list_start(&line) {
                let sub_list = self.parse_enum_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Same indent, bullet list start -> new item
            if indent == base_indent && self.is_bullet_list_start(&line) {
                let trimmed = line.trim_start();
                let text = &trimmed[2..]; // skip "* " or "- "
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_rst_inlines(text),
                    }],
                });
                self.advance();
                continue;
            }

            // Continuation line (deeper indent, not a list)
            if indent > base_indent {
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(parse_rst_inlines(line.trim()));
                }
                self.advance();
                continue;
            }

            // Same or less indent, not a bullet -> end of list
            break;
        }
        Ok(Block::UnorderedList { items })
    }

    fn is_enum_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        // #. item or 1. item
        if trimmed.starts_with("#. ") {
            return true;
        }
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            return prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty();
        }
        false
    }

    fn parse_enum_list(&mut self) -> Result<Block, ParseError> {
        let base_indent = self.current_line().len() - self.current_line().trim_start().len();
        self.parse_enum_list_at_indent(base_indent)
    }

    fn parse_enum_list_at_indent(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();
        let mut start = 1u32;
        let mut first = true;

        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                let saved = self.pos;
                self.advance();
                while !self.at_end() && self.current_line().trim().is_empty() {
                    self.advance();
                }
                if !self.at_end() {
                    let next_indent =
                        self.current_line().len() - self.current_line().trim_start().len();
                    if next_indent >= base_indent && self.is_enum_list_start(self.current_line()) {
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let indent = line.len() - line.trim_start().len();

            // Deeper indented bullet list -> sub-list
            if indent > base_indent && self.is_bullet_list_start(&line) {
                let sub_list = self.parse_bullet_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Deeper indented enum list -> sub-list
            if indent > base_indent && self.is_enum_list_start(&line) {
                let sub_list = self.parse_enum_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Same indent, enum list start -> new item
            if indent == base_indent && self.is_enum_list_start(&line) {
                let trimmed = line.trim_start();
                let (num_str, text) = if trimmed.starts_with("#. ") {
                    ("#", &trimmed[3..])
                } else {
                    let dot_pos = trimmed.find(". ").unwrap();
                    (&trimmed[..dot_pos], &trimmed[dot_pos + 2..])
                };

                if first {
                    if num_str != "#" {
                        start = num_str.parse().unwrap_or(1);
                    }
                    first = false;
                }

                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_rst_inlines(text),
                    }],
                });
                self.advance();
                continue;
            }

            // Continuation line
            if indent > base_indent {
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(parse_rst_inlines(line.trim()));
                }
                self.advance();
                continue;
            }

            break;
        }

        Ok(Block::OrderedList { start, items })
    }

    fn is_simple_table_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.len() >= 3
            && trimmed.chars().all(|c| c == '=' || c == ' ')
            && trimmed.contains('=')
            && trimmed.contains(' ')
    }

    fn parse_simple_table(&mut self) -> Result<Block, ParseError> {
        // Parse column boundaries from the === === === line
        let border = self.current_line().to_string();
        let col_ranges = parse_table_column_ranges(&border);
        self.advance();

        let mut headers = Vec::new();
        let mut rows: Vec<Vec<TableCell>> = Vec::new();
        let mut in_header = true;

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = line.trim();

            // Another border line
            if !trimmed.is_empty()
                && trimmed.chars().all(|c| c == '=' || c == ' ')
                && trimmed.contains('=')
            {
                self.advance();
                if in_header {
                    in_header = false;
                    continue;
                }
                // End of table
                break;
            }

            if trimmed.chars().all(|c| c == '-' || c == ' ') && trimmed.contains('-') {
                self.advance();
                continue;
            }

            if trimmed.is_empty() {
                self.advance();
                break;
            }

            let cells = extract_cells_from_line(&line, &col_ranges);
            if in_header {
                headers = cells;
            } else {
                rows.push(cells);
            }
            self.advance();
        }

        let alignments = vec![ColumnAlignment::Default; headers.len()];
        Ok(Block::Table {
            headers,
            alignments,
            rows,
        })
    }

    fn try_parse_grid_table(&mut self) -> Result<Option<Block>, ParseError> {
        let first_line = self.current_line().to_string();
        if !first_line.trim().starts_with('+') || !first_line.contains('-') {
            return Ok(None);
        }

        // Collect all grid table lines
        let mut table_lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().starts_with('+') || line.trim().starts_with('|') {
                table_lines.push(line);
                self.advance();
            } else {
                break;
            }
        }

        if table_lines.len() < 3 {
            self.pos -= table_lines.len();
            return Ok(None);
        }

        // Parse column positions from first border (top line with all + marks)
        let col_positions = parse_grid_column_positions(&table_lines[0]);
        let num_cols = if col_positions.len() > 1 {
            col_positions.len() - 1
        } else {
            return Ok(None);
        };

        // Categorize lines: separator lines start with + , data lines start with |
        let mut separator_indices = Vec::new();
        let mut data_line_indices = Vec::new();
        let mut header_separator_idx = None;

        for (i, line) in table_lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('+') {
                separator_indices.push(i);
                if line.contains('=') {
                    header_separator_idx = Some(i);
                }
            } else if trimmed.starts_with('|') {
                data_line_indices.push(i);
            }
        }

        // Group data lines into logical rows (between consecutive separator lines)
        let mut row_groups: Vec<Vec<usize>> = Vec::new();
        for i in 0..separator_indices.len() - 1 {
            let start = separator_indices[i];
            let end = separator_indices[i + 1];
            let group: Vec<usize> = data_line_indices
                .iter()
                .filter(|&&idx| idx > start && idx < end)
                .copied()
                .collect();
            if !group.is_empty() {
                row_groups.push(group);
            }
        }

        // For each logical row, detect colspan by checking the separator line ABOVE it
        // A missing + at a column position means colspan
        // For rowspan: check if a separator line has spaces instead of dashes at a column position
        let mut all_rows: Vec<Vec<TableCell>> = Vec::new();
        let mut occupied: Vec<Vec<bool>> = vec![vec![false; num_cols]; row_groups.len()];

        for (group_idx, group) in row_groups.iter().enumerate() {
            // The separator line above this row group
            let sep_above_idx = separator_indices
                .iter()
                .rfind(|&&s| s < group[0])
                .copied()
                .unwrap_or(0);
            let sep_above = &table_lines[sep_above_idx];

            // The separator line below this row group
            let sep_below_idx = separator_indices
                .iter()
                .find(|&&s| s > *group.last().unwrap())
                .copied();

            // Detect which columns have a separator (+ at expected positions)
            let sep_chars: Vec<char> = sep_above.chars().collect();
            let mut has_plus = vec![false; col_positions.len()];
            for (i, &pos) in col_positions.iter().enumerate() {
                if pos < sep_chars.len() && sep_chars[pos] == '+' {
                    has_plus[i] = true;
                }
            }

            // Also check the content line for missing | at column positions.
            // The emitter only removes + when both adjacent rows span, so
            // the separator alone is insufficient for colspan detection.
            let first_data_line = &table_lines[group[0]];
            let data_chars: Vec<char> = first_data_line.chars().collect();
            for (i, &pos) in col_positions.iter().enumerate() {
                if i > 0
                    && i < col_positions.len() - 1
                    && has_plus[i]
                    && pos < data_chars.len()
                    && data_chars[pos] != '|'
                {
                    has_plus[i] = false;
                }
            }

            // Check rowspan: does the separator below have spaces at a column's position?
            let mut is_rowspan_col = vec![false; num_cols];
            if let Some(below_idx) = sep_below_idx {
                let sep_below = &table_lines[below_idx];
                let below_chars: Vec<char> = sep_below.chars().collect();
                for col in 0..num_cols {
                    let start = col_positions[col] + 1;
                    let end = col_positions[col + 1];
                    // Check if this segment has spaces instead of dashes
                    if start < below_chars.len() && end <= below_chars.len() {
                        let segment: String = below_chars[start..end].iter().collect();
                        if segment.trim().is_empty()
                            || (!segment.contains('-') && !segment.contains('='))
                        {
                            is_rowspan_col[col] = true;
                        }
                    }
                }
            }

            // Build cells for this row
            let mut cells = Vec::new();
            let mut col = 0;
            while col < num_cols {
                if occupied[group_idx][col] {
                    col += 1;
                    continue;
                }

                // Detect colspan: count consecutive columns where the + is missing
                let mut cspan = 1;
                while col + cspan < num_cols && !has_plus[col + cspan] {
                    cspan += 1;
                }

                // Detect rowspan: count how many subsequent row groups this cell spans
                let mut rspan = 1u32;
                if is_rowspan_col[col] {
                    // Look at subsequent separator lines
                    let mut check_group = group_idx + 1;
                    while check_group < row_groups.len() {
                        let next_sep_idx = separator_indices
                            .iter()
                            .rfind(|&&s| s < row_groups[check_group][0])
                            .copied();
                        if let Some(ns_idx) = next_sep_idx {
                            let ns = &table_lines[ns_idx];
                            let ns_chars: Vec<char> = ns.chars().collect();
                            let start = col_positions[col] + 1;
                            let end = col_positions[col + 1];
                            if start < ns_chars.len() && end <= ns_chars.len() {
                                let segment: String = ns_chars[start..end].iter().collect();
                                if segment.trim().is_empty()
                                    || (!segment.contains('-') && !segment.contains('='))
                                {
                                    rspan += 1;
                                    check_group += 1;
                                    continue;
                                }
                            }
                        }
                        break;
                    }
                }

                // Extract cell content
                let start = col_positions[col] + 1;
                let end = col_positions[col + cspan];
                let cell_text: String = data_chars
                    .get(start..end.min(data_chars.len()))
                    .unwrap_or(&[])
                    .iter()
                    .collect();
                let trimmed = cell_text.trim();

                cells.push(TableCell::with_span(
                    parse_rst_inlines(trimmed),
                    cspan as u32,
                    rspan,
                ));
                if rspan > 1 {
                    for r in 1..rspan as usize {
                        for c in 0..cspan {
                            if group_idx + r < occupied.len() && col + c < num_cols {
                                occupied[group_idx + r][col + c] = true;
                            }
                        }
                    }
                }
                col += cspan;
            }
            all_rows.push(cells);
        }

        // Separate headers from body
        let header_group_count = if let Some(h_sep) = header_separator_idx {
            row_groups.iter().filter(|g| g[0] < h_sep).count()
        } else {
            0
        };

        let (headers, rows) = if header_group_count > 0 && all_rows.len() > header_group_count {
            let body = all_rows.split_off(header_group_count);
            let hdrs = all_rows.into_iter().next().unwrap_or_default();
            (hdrs, body)
        } else if !all_rows.is_empty() {
            let rest = all_rows.split_off(1);
            (all_rows.into_iter().next().unwrap_or_default(), rest)
        } else {
            (Vec::new(), Vec::new())
        };

        let alignments = vec![ColumnAlignment::Default; num_cols];
        Ok(Some(Block::Table {
            headers,
            alignments,
            rows,
        }))
    }

    fn parse_paragraph(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                break;
            }
            // Stop at block-level constructs
            if line.trim_start().starts_with(".. ") {
                break;
            }
            if self.is_bullet_list_start(&line) || self.is_enum_list_start(&line) {
                break;
            }
            // Check for heading (next line is adornment)
            if self.pos + 1 < self.lines.len() {
                let next = self.lines[self.pos + 1].trim().to_string();
                if Self::is_adornment_line(&next).is_some()
                    && next.len() >= line.trim().len()
                    && lines.is_empty()
                {
                    break;
                }
            }
            // Check if current line is an adornment (heading overline)
            if Self::is_adornment_line(line.trim()).is_some() && lines.is_empty() {
                break;
            }
            lines.push(line);
            self.advance();
        }
        let text = lines.join("\n");
        Ok(Block::Paragraph {
            content: parse_rst_inlines(&text),
        })
    }
}

// --- RST inline parsing ---

fn parse_rst_inlines(input: &str) -> Vec<Inline> {
    let mut parser = RstInlineParser::new(input);
    parser.parse()
}

struct RstInlineParser {
    chars: Vec<char>,
    pos: usize,
}

impl RstInlineParser {
    fn new(input: &str) -> Self {
        RstInlineParser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn current(&self) -> char {
        self.chars[self.pos]
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn parse(&mut self) -> Vec<Inline> {
        let mut result = Vec::new();
        while !self.at_end() {
            if let Some(inline) = self.try_parse_inline() {
                result.push(inline);
            }
        }
        merge_adjacent_text(result)
    }

    fn try_parse_inline(&mut self) -> Option<Inline> {
        if self.at_end() {
            return None;
        }
        match self.current() {
            '*' => self.parse_emphasis(),
            '`' => self.parse_interpreted_or_code(),
            '\\' => self.parse_escape(),
            '\n' => self.parse_line_break(),
            _ => self.parse_text(),
        }
    }

    fn parse_emphasis(&mut self) -> Option<Inline> {
        // ** bold **
        if self.peek(1) == Some('*') {
            self.pos += 2;
            if let Some(content) = self.parse_until_str("**") {
                let inlines = RstInlineParser::new(&content).parse();
                return Some(Inline::Bold(inlines));
            }
            self.pos -= 2;
        }
        // * italic *
        self.pos += 1;
        if let Some(content) = self.parse_until_char('*') {
            let inlines = RstInlineParser::new(&content).parse();
            return Some(Inline::Italic(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("*".to_string()))
    }

    fn parse_interpreted_or_code(&mut self) -> Option<Inline> {
        // ``code`` - inline literal
        if self.peek(1) == Some('`') {
            self.pos += 2;
            if let Some(content) = self.parse_until_str("``") {
                return Some(Inline::InlineCode(content));
            }
            self.pos -= 2;
        }

        // `text <url>`_ - link
        // `interpreted text` - roles
        self.pos += 1;
        let start = self.pos;
        while !self.at_end() && self.current() != '`' {
            self.pos += 1;
        }
        if self.at_end() {
            self.pos = start - 1;
            self.pos += 1;
            return Some(Inline::Text("`".to_string()));
        }
        let content: String = self.chars[start..self.pos].iter().collect();
        self.pos += 1; // skip closing `

        // Check for link suffix _
        if !self.at_end() && self.current() == '_' {
            self.pos += 1;
            // Parse `text <url>`_
            if let Some(angle_start) = content.rfind('<')
                && content.ends_with('>')
            {
                let text = content[..angle_start].trim();
                let url = &content[angle_start + 1..content.len() - 1];
                return Some(Inline::Link {
                    url: url.to_string(),
                    text: vec![Inline::Text(text.to_string())],
                    title: None,
                });
            }
            // Named reference
            return Some(Inline::Link {
                url: content.clone(),
                text: vec![Inline::Text(content)],
                title: None,
            });
        }

        // Plain interpreted text - treat as inline code or text
        Some(Inline::InlineCode(content))
    }

    fn parse_escape(&mut self) -> Option<Inline> {
        self.pos += 1;
        if !self.at_end() {
            let c = self.current();
            self.pos += 1;
            Some(Inline::Text(c.to_string()))
        } else {
            Some(Inline::Text("\\".to_string()))
        }
    }

    fn parse_line_break(&mut self) -> Option<Inline> {
        // RST uses | for line blocks; regular newlines are soft breaks
        self.pos += 1;
        Some(Inline::SoftLineBreak)
    }

    fn parse_text(&mut self) -> Option<Inline> {
        let start = self.pos;
        while !self.at_end() {
            match self.current() {
                '*' | '`' | '\\' | '\n' => break,
                _ => self.pos += 1,
            }
        }
        if self.pos == start {
            self.pos += 1;
            return Some(Inline::Text(self.chars[start].to_string()));
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        Some(Inline::Text(text))
    }

    fn parse_until_char(&mut self, delim: char) -> Option<String> {
        let start = self.pos;
        let mut content = Vec::new();
        while !self.at_end() {
            if self.current() == delim {
                self.pos += 1;
                return Some(content.iter().collect());
            }
            if self.current() == '\n' && self.peek(1) == Some('\n') {
                self.pos = start;
                return None;
            }
            content.push(self.current());
            self.pos += 1;
        }
        self.pos = start;
        None
    }

    fn parse_until_str(&mut self, delim: &str) -> Option<String> {
        let delim_chars: Vec<char> = delim.chars().collect();
        let start = self.pos;
        let mut content = Vec::new();
        while !self.at_end() {
            if self.chars[self.pos..].starts_with(&delim_chars) {
                self.pos += delim_chars.len();
                return Some(content.iter().collect());
            }
            content.push(self.current());
            self.pos += 1;
        }
        self.pos = start;
        None
    }
}

fn merge_adjacent_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut result: Vec<Inline> = Vec::new();
    for inline in inlines {
        if let Inline::Text(ref t) = inline
            && let Some(Inline::Text(prev)) = result.last_mut()
        {
            prev.push_str(t);
            continue;
        }
        result.push(inline);
    }
    result
}

fn parse_table_column_ranges(border: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let chars: Vec<char> = border.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '=' {
            let start = i;
            while i < chars.len() && chars[i] == '=' {
                i += 1;
            }
            ranges.push((start, i));
        } else {
            i += 1;
        }
    }
    ranges
}

fn extract_cells_from_line(line: &str, ranges: &[(usize, usize)]) -> Vec<TableCell> {
    let chars: Vec<char> = line.chars().collect();
    ranges
        .iter()
        .map(|&(start, end)| {
            let cell_chars: String = chars
                .get(start..end.min(chars.len()))
                .unwrap_or(&[])
                .iter()
                .collect();
            TableCell::new(parse_rst_inlines(cell_chars.trim()))
        })
        .collect()
}

fn parse_grid_column_positions(border: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    for (i, ch) in border.chars().enumerate() {
        if ch == '+' {
            positions.push(i);
        }
    }
    positions
}
