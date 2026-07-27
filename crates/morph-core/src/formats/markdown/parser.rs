use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;
use std::collections::HashMap;

pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let mut parser = MarkdownParserState::new(input);
        let children = parser.parse_blocks()?;
        Ok(Document { children })
    }
}

struct LinkDef {
    url: String,
    title: Option<String>,
}

struct MarkdownParserState {
    lines: Vec<String>,
    pos: usize,
    link_defs: HashMap<String, LinkDef>,
}

impl MarkdownParserState {
    fn new(input: &str) -> Self {
        let lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();
        let mut state = MarkdownParserState {
            lines,
            pos: 0,
            link_defs: HashMap::new(),
        };
        state.collect_link_defs();
        state
    }

    fn collect_link_defs(&mut self) {
        let mut to_remove = Vec::new();
        for i in 0..self.lines.len() {
            if let Some(def) = Self::parse_link_def_line(&self.lines[i]) {
                self.link_defs.insert(
                    def.0,
                    LinkDef {
                        url: def.1,
                        title: def.2,
                    },
                );
                to_remove.push(i);
            }
        }
        for i in to_remove.into_iter().rev() {
            self.lines.remove(i);
        }
    }

    fn parse_link_def_line(line: &str) -> Option<(String, String, Option<String>)> {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') {
            return None;
        }
        let rest = &trimmed[1..];
        let close = rest.find(']')?;
        let id = rest[..close].to_string();
        let after = rest[close + 1..].trim_start();
        if !after.starts_with(':') {
            return None;
        }
        let url_part = after[1..].trim();
        let (url, remainder) = if url_part.starts_with('<') {
            let end = url_part.find('>')?;
            (&url_part[1..end], url_part[end + 1..].trim())
        } else {
            match url_part.find(|c: char| c.is_whitespace()) {
                Some(i) => (&url_part[..i], url_part[i..].trim()),
                None => (url_part, ""),
            }
        };
        let title = if remainder.starts_with('"') && remainder.ends_with('"') && remainder.len() > 1
        {
            Some(remainder[1..remainder.len() - 1].to_string())
        } else {
            None
        };
        Some((id.to_lowercase(), url.to_string(), title))
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

    fn parse_blocks(&mut self) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();

            if line.trim().is_empty() {
                self.advance();
                continue;
            }

            if line.trim().starts_with("<table")
                && let Some(block) = self.parse_html_table()?
            {
                blocks.push(block);
                continue;
            }

            if let Some(block) = self.try_parse_atx_heading() {
                blocks.push(block);
                continue;
            }

            if let Some(block) = self.try_parse_setext_heading() {
                blocks.push(block);
                continue;
            }

            if self.is_horizontal_rule(&line) {
                blocks.push(Block::HorizontalRule);
                self.advance();
                continue;
            }

            if line.trim_start().starts_with("```") {
                blocks.push(self.parse_fenced_code_block()?);
                continue;
            }

            if line.starts_with("    ") && !line.trim().is_empty() {
                blocks.push(self.parse_indented_code_block()?);
                continue;
            }

            if line.trim_start().starts_with("> ") || line.trim_start() == ">" {
                blocks.push(self.parse_blockquote()?);
                continue;
            }

            if self.is_unordered_list_start(&line) {
                blocks.push(self.parse_unordered_list(0)?);
                continue;
            }

            if self.is_ordered_list_start(&line) {
                blocks.push(self.parse_ordered_list(0)?);
                continue;
            }

            if self.is_table_start() {
                blocks.push(self.parse_table()?);
                continue;
            }

            if self.is_description_list_start() {
                blocks.push(self.parse_description_list()?);
                continue;
            }

            blocks.push(self.parse_paragraph()?);
        }
        Ok(blocks)
    }

    fn try_parse_atx_heading(&mut self) -> Option<Block> {
        let line = self.current_line().to_string();
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return None;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if hashes > 6 {
            return None;
        }
        let rest = &trimmed[hashes..];
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        let text = rest.trim().trim_end_matches('#').trim();
        self.advance();
        Some(Block::Heading {
            level: hashes as u8,
            content: self.parse_inlines(text),
        })
    }

    fn try_parse_setext_heading(&mut self) -> Option<Block> {
        if self.pos + 1 >= self.lines.len() {
            return None;
        }
        let next_line = self.lines[self.pos + 1].trim().to_string();
        let level =
            if !next_line.is_empty() && next_line.chars().all(|c| c == '=') && next_line.len() >= 3
            {
                1
            } else if !next_line.is_empty()
                && next_line.chars().all(|c| c == '-')
                && next_line.len() >= 3
            {
                2
            } else {
                return None;
            };
        let text = self.current_line().trim().to_string();
        self.advance();
        self.advance();
        Some(Block::Heading {
            level,
            content: self.parse_inlines(&text),
        })
    }

    fn is_horizontal_rule(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let no_spaces: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        (no_spaces.chars().all(|c| c == '-')
            || no_spaces.chars().all(|c| c == '*')
            || no_spaces.chars().all(|c| c == '_'))
            && no_spaces.len() >= 3
    }

    fn parse_fenced_code_block(&mut self) -> Result<Block, ParseError> {
        let line = self.current_line().trim_start().to_string();
        let fence_count = line.chars().take_while(|&c| c == '`').count();
        let info = line[fence_count..].trim();
        let language = if info.is_empty() {
            None
        } else {
            Some(info.to_string())
        };
        self.advance();

        let mut content_lines = Vec::new();
        while !self.at_end() {
            let l = self.current_line().to_string();
            if l.trim_start().starts_with("```")
                && l.trim().chars().filter(|&c| c == '`').count() >= fence_count
            {
                self.advance();
                break;
            }
            content_lines.push(l);
            self.advance();
        }

        Ok(Block::CodeBlock {
            language,
            content: content_lines.join("\n"),
        })
    }

    fn parse_indented_code_block(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if let Some(rest) = line.strip_prefix("    ") {
                lines.push(rest.to_string());
                self.advance();
            } else if line.trim().is_empty() {
                lines.push(String::new());
                self.advance();
            } else {
                break;
            }
        }
        while lines.last().map(|s| s.as_str()) == Some("") {
            lines.pop();
        }
        Ok(Block::CodeBlock {
            language: None,
            content: lines.join("\n"),
        })
    }

    fn parse_blockquote(&mut self) -> Result<Block, ParseError> {
        let mut inner_lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if let Some(rest) = line.strip_prefix("> ") {
                inner_lines.push(rest.to_string());
                self.advance();
            } else if line.trim() == ">" {
                inner_lines.push(String::new());
                self.advance();
            } else {
                break;
            }
        }
        let inner_text = inner_lines.join("\n");
        let mut inner_parser = MarkdownParserState::new(&inner_text);
        inner_parser.link_defs = HashMap::new(); // inherit nothing for now
        let children = inner_parser.parse_blocks()?;
        Ok(Block::BlockQuote { children })
    }

    fn is_unordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("* ")
            || trimmed.starts_with("- ")
            || trimmed.starts_with("+ ")
            || trimmed == "*"
            || trimmed == "-"
            || trimmed == "+"
    }

    fn get_list_indent(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }

    fn parse_unordered_list(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                let saved = self.pos;
                self.advance();
                while !self.at_end() && self.current_line().trim().is_empty() {
                    self.advance();
                }
                if !self.at_end() {
                    let next = self.current_line().to_string();
                    let indent = Self::get_list_indent(&next);
                    if indent == base_indent && self.is_unordered_list_start(&next) {
                        items.push(ListItem {
                            content: vec![Block::Paragraph { content: vec![] }],
                        });
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let indent = Self::get_list_indent(&line);
            if indent < base_indent {
                break;
            }
            let trimmed = line.trim_start().to_string();
            if indent > base_indent {
                if self.is_unordered_list_start(&trimmed) {
                    let sub = self.parse_unordered_list(indent)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                if self.is_ordered_list_start(&trimmed) {
                    let sub = self.parse_ordered_list(indent)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(self.parse_inlines(&trimmed));
                }
                self.advance();
                continue;
            }

            if self.is_unordered_list_start(&trimmed) {
                let text = if trimmed.len() > 2 { &trimmed[2..] } else { "" };
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: self.parse_inlines(text),
                    }],
                });
                self.advance();
            } else {
                break;
            }
        }

        Ok(Block::UnorderedList { items })
    }

    fn is_ordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty()
        } else {
            false
        }
    }

    fn parse_ordered_list(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items = Vec::new();
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
                    let next = self.current_line().to_string();
                    let indent = Self::get_list_indent(&next);
                    let trimmed = next.trim_start().to_string();
                    if indent == base_indent && self.is_ordered_list_start(&trimmed) {
                        items.push(ListItem {
                            content: vec![Block::Paragraph { content: vec![] }],
                        });
                        continue;
                    }
                    if indent > base_indent
                        && (self.is_unordered_list_start(&trimmed)
                            || self.is_ordered_list_start(&trimmed))
                    {
                        // Sub-list after blank line - attach to previous item
                        // Add empty paragraph separator inside the item for spacing
                        if let Some(last) = items.last_mut() {
                            last.content.push(Block::Paragraph { content: vec![] });
                        }
                        let sub = if self.is_unordered_list_start(&trimmed) {
                            self.parse_unordered_list(indent)?
                        } else {
                            self.parse_ordered_list(indent)?
                        };
                        if let Some(last) = items.last_mut() {
                            last.content.push(sub);
                        }
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let indent = Self::get_list_indent(&line);
            if indent < base_indent {
                break;
            }
            let trimmed = line.trim_start().to_string();

            if indent > base_indent {
                if self.is_unordered_list_start(&trimmed) {
                    let sub = self.parse_unordered_list(indent)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                if self.is_ordered_list_start(&trimmed) {
                    let sub = self.parse_ordered_list(indent)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(self.parse_inlines(&trimmed));
                }
                self.advance();
                continue;
            }

            if self.is_ordered_list_start(&trimmed) {
                let dot_pos = trimmed.find(". ").unwrap();
                let num: u32 = trimmed[..dot_pos].parse().unwrap_or(1);
                if first {
                    start = num;
                    first = false;
                }
                let text = &trimmed[dot_pos + 2..];
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: self.parse_inlines(text),
                    }],
                });
                self.advance();
            } else {
                break;
            }
        }

        Ok(Block::OrderedList { start, items })
    }

    fn is_table_start(&self) -> bool {
        if self.pos + 1 >= self.lines.len() {
            return false;
        }
        let line = self.current_line().trim().to_string();
        let next = self.lines[self.pos + 1].trim().to_string();
        line.contains('|') && self.is_table_separator(&next)
    }

    fn is_table_separator(&self, line: &str) -> bool {
        let trimmed = line.trim().trim_start_matches('|').trim_end_matches('|');
        if trimmed.is_empty() {
            return false;
        }
        trimmed.split('|').all(|cell| {
            let c = cell.trim();
            !c.is_empty()
                && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
                && c.contains('-')
        })
    }

    fn parse_table(&mut self) -> Result<Block, ParseError> {
        let header_line = self.current_line().to_string();
        let headers = self.parse_table_row(&header_line);
        self.advance();

        let sep_line = self.current_line().to_string();
        let alignments = self.parse_table_alignments(&sep_line);
        self.advance();

        let mut rows = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if !line.contains('|') || line.trim().is_empty() {
                break;
            }
            rows.push(self.parse_table_row(&line));
            self.advance();
        }

        Ok(Block::Table {
            headers,
            alignments,
            rows,
        })
    }

    fn parse_table_row(&self, line: &str) -> Vec<TableCell> {
        let trimmed = line.trim();
        let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
        let inner = inner.strip_suffix('|').unwrap_or(inner);
        inner
            .split('|')
            .map(|cell| TableCell::new(self.parse_inlines(cell.trim())))
            .collect()
    }

    fn parse_table_alignments(&self, line: &str) -> Vec<ColumnAlignment> {
        let trimmed = line.trim().trim_start_matches('|').trim_end_matches('|');
        trimmed
            .split('|')
            .map(|cell| {
                let c = cell.trim();
                let left = c.starts_with(':');
                let right = c.ends_with(':');
                match (left, right) {
                    (true, true) => ColumnAlignment::Center,
                    (false, true) => ColumnAlignment::Right,
                    (true, false) => ColumnAlignment::Left,
                    (false, false) => ColumnAlignment::Default,
                }
            })
            .collect()
    }

    fn parse_html_table(&mut self) -> Result<Option<Block>, ParseError> {
        let mut html = String::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            html.push_str(&line);
            html.push('\n');
            self.advance();
            if line.contains("</table>") {
                break;
            }
        }
        let mut headers = Vec::new();
        let mut rows: Vec<Vec<TableCell>> = Vec::new();
        let mut in_header = false;
        let mut current_row: Vec<TableCell> = Vec::new();

        for line in html.lines() {
            let trimmed = line.trim();
            if trimmed.contains("<th>") || trimmed.contains("<th ") {
                in_header = true;
            }
            if trimmed.contains("<tr>") || trimmed.contains("<tr ") {
                current_row = Vec::new();
            }
            // Extract cells BEFORE checking </tr>
            // Match both simple tags and tags with attributes
            let mut search = trimmed;
            loop {
                // Find the earliest opening tag among <th>, <th ...>, <td>, <td ...>
                let mut best: Option<(usize, usize, &str)> = None; // (start, tag_end, close_tag)

                for tag_name in &["th", "td"] {
                    let open_simple = format!("<{}>", tag_name);
                    let open_attr = format!("<{} ", tag_name);
                    let close = if *tag_name == "th" { "</th>" } else { "</td>" };

                    if let Some(idx) = search.find(&open_simple) {
                        let tag_end = idx + open_simple.len();
                        if best.is_none() || idx < best.unwrap().0 {
                            best = Some((idx, tag_end, close));
                        }
                    }
                    if let Some(idx) = search.find(&open_attr)
                        && let Some(gt) = search[idx..].find('>')
                    {
                        let tag_end = idx + gt + 1;
                        if best.is_none() || idx < best.unwrap().0 {
                            best = Some((idx, tag_end, close));
                        }
                    }
                }

                let (start_idx, tag_end, close) = match best {
                    Some(b) => b,
                    None => break,
                };

                // Extract attributes from the opening tag
                let tag_content = &search[start_idx..tag_end];
                let colspan = extract_html_attr(tag_content, "colspan").unwrap_or(1);
                let rowspan = extract_html_attr(tag_content, "rowspan").unwrap_or(1);

                if let Some(end) = search[tag_end..].find(close) {
                    let cell_text = &search[tag_end..tag_end + end];
                    current_row.push(TableCell::with_span(
                        self.parse_inlines(cell_text),
                        colspan,
                        rowspan,
                    ));
                    search = &search[tag_end + end + close.len()..];
                } else {
                    break;
                }
            }
            if trimmed.contains("</tr>") {
                if in_header && headers.is_empty() {
                    headers = current_row.clone();
                    in_header = false;
                } else if !current_row.is_empty() {
                    rows.push(current_row.clone());
                }
                current_row = Vec::new();
            }
        }

        let num_cols = count_logical_columns(&headers);
        let alignments = vec![ColumnAlignment::Default; num_cols];
        Ok(Some(Block::Table {
            headers,
            alignments,
            rows,
        }))
    }

    fn is_description_list_start(&self) -> bool {
        if self.pos + 1 >= self.lines.len() {
            return false;
        }
        let line = self.current_line().trim().to_string();
        let next = self.lines[self.pos + 1].trim().to_string();
        !line.is_empty() && !line.starts_with('#') && next.starts_with(":   ")
    }

    fn parse_description_list(&mut self) -> Result<Block, ParseError> {
        let mut items = Vec::new();
        while !self.at_end() && self.is_description_list_start() {
            let term_text = self.current_line().trim().to_string();
            let term = self.parse_inlines(&term_text);
            self.advance();

            let mut def_lines = Vec::new();
            while !self.at_end() {
                let line = self.current_line().to_string();
                if let Some(rest) = line.strip_prefix(":   ") {
                    def_lines.push(rest.to_string());
                    self.advance();
                } else if line.starts_with("    ") || line.starts_with('\t') {
                    def_lines.push(line.trim_start().to_string());
                    self.advance();
                } else if line.trim().is_empty() {
                    self.advance();
                    if !self.at_end() && self.is_description_list_start() {
                        break;
                    }
                    if !self.at_end() {
                        let next = self.current_line().to_string();
                        if next.starts_with(":   ") || next.starts_with("    ") {
                            def_lines.push(String::new());
                            continue;
                        }
                    }
                    break;
                } else {
                    def_lines.push(line.to_string());
                    self.advance();
                }
            }

            let def_text = def_lines.join("\n");
            let definition = vec![Block::Paragraph {
                content: self.parse_inlines(&def_text),
            }];

            items.push(DescriptionItem {
                term,
                definitions: vec![definition],
            });
        }
        Ok(Block::DescriptionList { items })
    }

    fn parse_paragraph(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                break;
            }
            if line.trim_start().starts_with("```")
                || line.trim_start().starts_with('#')
                || self.is_horizontal_rule(&line)
                || self.is_unordered_list_start(&line)
                || self.is_ordered_list_start(&line)
                || self.is_table_start()
                || line.trim_start().starts_with("> ")
                || line.trim_start().starts_with("<table")
            {
                break;
            }
            if self.pos + 1 < self.lines.len() {
                let next = self.lines[self.pos + 1].trim().to_string();
                if ((!next.is_empty() && next.chars().all(|c| c == '=') && next.len() >= 3)
                    || (!next.is_empty() && next.chars().all(|c| c == '-') && next.len() >= 3))
                    && lines.is_empty()
                {
                    break;
                }
            }
            lines.push(line);
            self.advance();
        }
        let text = lines.join("\n");
        Ok(Block::Paragraph {
            content: self.parse_inlines(&text),
        })
    }

    /// Parse inlines with access to link definitions
    fn parse_inlines(&self, input: &str) -> Vec<Inline> {
        let mut parser = InlineParser::new(input, &self.link_defs);
        parser.parse()
    }
}

// --- Inline parsing ---

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    let empty = HashMap::new();
    let mut parser = InlineParser::new(input, &empty);
    parser.parse()
}

struct InlineParser<'a> {
    chars: Vec<char>,
    pos: usize,
    link_defs: &'a HashMap<String, LinkDef>,
}

impl<'a> InlineParser<'a> {
    fn new(input: &str, link_defs: &'a HashMap<String, LinkDef>) -> Self {
        InlineParser {
            chars: input.chars().collect(),
            pos: 0,
            link_defs,
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
        let result = merge_adjacent_text(result);
        // Post-process: strip trailing spaces from Text before line breaks
        strip_trailing_spaces_before_breaks(result)
    }

    fn try_parse_inline(&mut self) -> Option<Inline> {
        if self.at_end() {
            return None;
        }

        match self.current() {
            '\\' => self.parse_escape(),
            '`' => self.parse_inline_code(),
            '*' | '_' => self.parse_emphasis(),
            '~' => self.parse_tilde(),
            '^' => self.parse_superscript(),
            '[' => self.parse_link_or_image(),
            '!' => self.parse_image(),
            '<' => self.parse_angle_bracket(),
            '\n' => self.parse_line_break(),
            _ => self.parse_text(),
        }
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

    fn parse_inline_code(&mut self) -> Option<Inline> {
        let start = self.pos;
        let backtick_count = self.chars[self.pos..]
            .iter()
            .take_while(|&&c| c == '`')
            .count();
        self.pos += backtick_count;

        let content_start = self.pos;
        loop {
            if self.at_end() {
                self.pos = start + 1;
                return Some(Inline::Text("`".to_string()));
            }
            if self.current() == '`' {
                let close_count = self.chars[self.pos..]
                    .iter()
                    .take_while(|&&c| c == '`')
                    .count();
                if close_count == backtick_count {
                    let content: String = self.chars[content_start..self.pos].iter().collect();
                    self.pos += close_count;
                    let content = if content.starts_with(' ')
                        && content.ends_with(' ')
                        && content.len() > 1
                    {
                        content[1..content.len() - 1].to_string()
                    } else {
                        content
                    };
                    return Some(Inline::InlineCode(content));
                }
                self.pos += close_count;
            } else {
                self.pos += 1;
            }
        }
    }

    fn parse_emphasis(&mut self) -> Option<Inline> {
        let marker = self.current();
        let run_len = self.chars[self.pos..]
            .iter()
            .take_while(|&&c| c == marker)
            .count();

        if run_len >= 3 {
            self.pos += 3;
            if let Some(content) =
                self.parse_until_delimiter(&format!("{}{}{}", marker, marker, marker))
            {
                let inlines = InlineParser::new(&content, self.link_defs).parse();
                return Some(Inline::BoldItalic(inlines));
            }
            self.pos -= 3;
        }

        if run_len >= 2 {
            self.pos += 2;
            if let Some(content) = self.parse_until_delimiter(&format!("{}{}", marker, marker)) {
                let inlines = InlineParser::new(&content, self.link_defs).parse();
                return Some(Inline::Bold(inlines));
            }
            self.pos -= 2;
        }

        if run_len >= 1 {
            self.pos += 1;
            if let Some(content) = self.parse_until_delimiter(&marker.to_string()) {
                let inlines = InlineParser::new(&content, self.link_defs).parse();
                return Some(Inline::Italic(inlines));
            }
            self.pos -= 1;
        }

        self.pos += 1;
        Some(Inline::Text(marker.to_string()))
    }

    fn parse_tilde(&mut self) -> Option<Inline> {
        if self.peek(1) == Some('~') {
            self.pos += 2;
            if let Some(content) = self.parse_until_delimiter("~~") {
                let inlines = InlineParser::new(&content, self.link_defs).parse();
                return Some(Inline::Strikethrough(inlines));
            }
            self.pos -= 2;
        }
        self.pos += 1;
        if let Some(content) = self.parse_until_delimiter("~") {
            let inlines = InlineParser::new(&content, self.link_defs).parse();
            return Some(Inline::Subscript(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("~".to_string()))
    }

    fn parse_superscript(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_delimiter("^") {
            let inlines = InlineParser::new(&content, self.link_defs).parse();
            return Some(Inline::Superscript(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("^".to_string()))
    }

    fn parse_link_or_image(&mut self) -> Option<Inline> {
        let start = self.pos;
        self.pos += 1; // skip [

        let mut depth = 1;
        let text_start = self.pos;
        while !self.at_end() && depth > 0 {
            match self.current() {
                '[' => depth += 1,
                ']' => depth -= 1,
                '\\' => {
                    self.pos += 1;
                }
                _ => {}
            }
            if depth > 0 {
                self.pos += 1;
            }
        }

        if self.at_end() || depth != 0 {
            self.pos = start + 1;
            return Some(Inline::Text("[".to_string()));
        }

        let text: String = self.chars[text_start..self.pos].iter().collect();
        self.pos += 1; // skip ]

        // Check for image inside link: [![alt](imgurl)](linkurl)
        if text.starts_with("![")
            && let Some((img_alt, img_url)) = parse_image_from_text(&text)
            && !self.at_end()
            && self.current() == '('
        {
            self.pos += 1;
            if let Some(url_info) = self.parse_url_in_parens() {
                return Some(Inline::Image {
                    url: img_url,
                    alt: InlineParser::new(&img_alt, self.link_defs).parse(),
                    title: None,
                    link: Some(url_info.0),
                });
            }
            self.pos -= 1;
        }

        // Inline link: [text](url)
        if !self.at_end() && self.current() == '(' {
            self.pos += 1;
            if let Some((url, title)) = self.parse_url_in_parens() {
                return Some(Inline::Link {
                    url,
                    text: InlineParser::new(&text, self.link_defs).parse(),
                    title,
                });
            }
            self.pos -= 1;
        }

        // Reference link: [text] [ref] or [text][ref]
        // Handle optional space between ] and [
        let mut ref_check_pos = self.pos;
        if ref_check_pos < self.chars.len() && self.chars[ref_check_pos] == ' ' {
            ref_check_pos += 1;
        }
        if ref_check_pos < self.chars.len() && self.chars[ref_check_pos] == '[' {
            self.pos = ref_check_pos + 1;
            let ref_start = self.pos;
            while !self.at_end() && self.current() != ']' {
                self.pos += 1;
            }
            if !self.at_end() {
                let ref_id: String = self.chars[ref_start..self.pos].iter().collect();
                self.pos += 1; // skip ]
                let key = if ref_id.is_empty() {
                    text.to_lowercase()
                } else {
                    ref_id.to_lowercase()
                };
                if let Some(def) = self.link_defs.get(&key) {
                    return Some(Inline::Link {
                        url: def.url.clone(),
                        text: InlineParser::new(&text, self.link_defs).parse(),
                        title: def.title.clone(),
                    });
                }
                // Not found - return as text
                self.pos = start + 1;
                return Some(Inline::Text("[".to_string()));
            }
            self.pos = start + 1;
            return Some(Inline::Text("[".to_string()));
        }

        // Bare reference: [text] - check link defs
        let key = text.to_lowercase();
        if let Some(def) = self.link_defs.get(&key) {
            return Some(Inline::Link {
                url: def.url.clone(),
                text: InlineParser::new(&text, self.link_defs).parse(),
                title: def.title.clone(),
            });
        }

        self.pos = start + 1;
        Some(Inline::Text("[".to_string()))
    }

    fn parse_url_in_parens(&mut self) -> Option<(String, Option<String>)> {
        let start = self.pos;
        let mut depth = 1;
        let mut url_chars = Vec::new();
        while !self.at_end() && depth > 0 {
            match self.current() {
                '(' => {
                    depth += 1;
                    url_chars.push('(');
                }
                ')' => {
                    depth -= 1;
                    if depth > 0 {
                        url_chars.push(')');
                    }
                }
                _ => url_chars.push(self.current()),
            }
            self.pos += 1;
        }
        if depth != 0 {
            self.pos = start;
            return None;
        }
        let raw: String = url_chars.into_iter().collect();
        let raw = raw.trim();
        if let Some(title_start) = raw.find(" \"") {
            let url = raw[..title_start].to_string();
            let title = raw[title_start + 2..].trim_end_matches('"').to_string();
            Some((url, Some(title)))
        } else {
            Some((raw.to_string(), None))
        }
    }

    fn parse_image(&mut self) -> Option<Inline> {
        if self.peek(1) != Some('[') {
            self.pos += 1;
            return Some(Inline::Text("!".to_string()));
        }
        let start = self.pos;
        self.pos += 2; // skip ![

        let alt_start = self.pos;
        let mut depth = 1;
        while !self.at_end() && depth > 0 {
            match self.current() {
                '[' => depth += 1,
                ']' => depth -= 1,
                '\\' => {
                    self.pos += 1;
                }
                _ => {}
            }
            if depth > 0 {
                self.pos += 1;
            }
        }
        if depth != 0 {
            self.pos = start + 1;
            return Some(Inline::Text("!".to_string()));
        }
        let alt_text: String = self.chars[alt_start..self.pos].iter().collect();
        self.pos += 1; // skip ]

        if !self.at_end() && self.current() == '(' {
            self.pos += 1;
            if let Some((url, title)) = self.parse_url_in_parens() {
                return Some(Inline::Image {
                    url,
                    alt: InlineParser::new(&alt_text, self.link_defs).parse(),
                    title,
                    link: None,
                });
            }
            self.pos -= 1;
        }

        // Reference image: ![alt][ref]
        if !self.at_end() && self.current() == '[' {
            self.pos += 1;
            let ref_start = self.pos;
            while !self.at_end() && self.current() != ']' {
                self.pos += 1;
            }
            if !self.at_end() {
                let ref_id: String = self.chars[ref_start..self.pos].iter().collect();
                self.pos += 1;
                let key = ref_id.to_lowercase();
                if let Some(def) = self.link_defs.get(&key) {
                    return Some(Inline::Image {
                        url: def.url.clone(),
                        alt: InlineParser::new(&alt_text, self.link_defs).parse(),
                        title: def.title.clone(),
                        link: None,
                    });
                }
            }
        }

        self.pos = start + 1;
        Some(Inline::Text("!".to_string()))
    }

    fn parse_angle_bracket(&mut self) -> Option<Inline> {
        self.pos += 1;
        Some(Inline::Text("<".to_string()))
    }

    fn parse_line_break(&mut self) -> Option<Inline> {
        // Check for hard line break: two or more spaces before newline
        // The spaces should already be in the text buffer, but since we accumulate text
        // separately, we need to check the chars before current position
        if self.pos >= 2 && self.chars[self.pos - 1] == ' ' && self.chars[self.pos - 2] == ' ' {
            self.pos += 1;
            return Some(Inline::HardLineBreak);
        }
        self.pos += 1;
        Some(Inline::SoftLineBreak)
    }

    fn parse_text(&mut self) -> Option<Inline> {
        let start = self.pos;
        while !self.at_end() {
            match self.current() {
                '\\' | '`' | '*' | '_' | '~' | '^' | '[' | '!' | '<' | '\n' => break,
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

    fn parse_until_delimiter(&mut self, delimiter: &str) -> Option<String> {
        let delim_chars: Vec<char> = delimiter.chars().collect();
        let start = self.pos;
        let mut content = Vec::new();

        while !self.at_end() {
            if self.chars[self.pos..].starts_with(&delim_chars) {
                self.pos += delim_chars.len();
                return Some(content.iter().collect());
            }
            if self.current() == '\n' {
                let remaining = &self.chars[self.pos + 1..];
                if remaining.starts_with(&['\n']) {
                    self.pos = start;
                    return None;
                }
            }
            content.push(self.current());
            self.pos += 1;
        }
        self.pos = start;
        None
    }
}

/// Extract a numeric HTML attribute value (e.g. colspan="2" -> 2)
fn extract_html_attr(tag: &str, attr: &str) -> Option<u32> {
    let pattern = format!("{}=\"", attr);
    if let Some(idx) = tag.find(&pattern) {
        let after = &tag[idx + pattern.len()..];
        if let Some(end) = after.find('"') {
            return after[..end].parse().ok();
        }
    }
    // Also try without quotes: colspan=2
    let pattern_nq = format!("{}=", attr);
    if let Some(idx) = tag.find(&pattern_nq) {
        let after = &tag[idx + pattern_nq.len()..];
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num_str.is_empty() {
            return num_str.parse().ok();
        }
    }
    None
}

/// Count logical columns from header cells (accounting for colspan)
fn count_logical_columns(headers: &[TableCell]) -> usize {
    headers
        .iter()
        .map(|c| c.colspan as usize)
        .sum::<usize>()
        .max(headers.len())
}

fn parse_image_from_text(text: &str) -> Option<(String, String)> {
    if !text.starts_with("![") {
        return None;
    }
    let rest = &text[2..];
    let close_bracket = rest.find(']')?;
    let alt = rest[..close_bracket].to_string();
    let after = &rest[close_bracket + 1..];
    if !after.starts_with('(') {
        return None;
    }
    let close_paren = after.rfind(')')?;
    let url = after[1..close_paren].to_string();
    Some((alt, url))
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

/// Strip trailing spaces from Text nodes that come before SoftLineBreak or HardLineBreak
fn strip_trailing_spaces_before_breaks(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut result = inlines;
    for i in 0..result.len() {
        if i + 1 < result.len() {
            let is_break = matches!(result[i + 1], Inline::SoftLineBreak | Inline::HardLineBreak);
            if is_break && let Inline::Text(ref mut text) = result[i] {
                *text = text.trim_end().to_string();
            }
        }
    }
    result
}
