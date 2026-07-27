use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;

pub struct AsciiDocParser;

impl Parser for AsciiDocParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let mut state = AsciiDocParserState::new(input);
        let children = state.parse_blocks()?;
        Ok(Document { children })
    }
}

struct AsciiDocParserState {
    lines: Vec<String>,
    pos: usize,
}

impl AsciiDocParserState {
    fn new(input: &str) -> Self {
        AsciiDocParserState {
            lines: input.lines().map(|l| l.to_string()).collect(),
            pos: 0,
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

    fn parse_blocks(&mut self) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();

            if line.trim().is_empty() {
                self.advance();
                continue;
            }

            if let Some(block) = self.try_parse_heading(&line) {
                blocks.push(block);
                continue;
            }

            if line.trim() == "'''" {
                blocks.push(Block::HorizontalRule);
                self.advance();
                continue;
            }

            if self.is_source_block_start(&line) {
                blocks.push(self.parse_source_block()?);
                continue;
            }

            if line.trim() == "----" {
                blocks.push(self.parse_listing_block()?);
                continue;
            }

            if self.is_blockquote_start(&line) {
                blocks.push(self.parse_blockquote()?);
                continue;
            }

            if line.trim() == "|===" {
                blocks.push(self.parse_table()?);
                continue;
            }

            if self.is_unordered_list_start(&line) {
                blocks.push(self.parse_unordered_list(1)?);
                continue;
            }

            if self.is_ordered_list_start(&line) {
                blocks.push(self.parse_ordered_list(1)?);
                continue;
            }

            if self.is_description_list_start() {
                blocks.push(self.parse_description_list()?);
                continue;
            }

            // Skip attribute lines like [cols="..."] or [source,...]
            if line.trim().starts_with('[') && line.trim().ends_with(']') {
                self.advance();
                continue;
            }

            blocks.push(self.parse_paragraph()?);
        }
        Ok(blocks)
    }

    fn try_parse_heading(&mut self, line: &str) -> Option<Block> {
        let trimmed = line.trim();
        if !trimmed.starts_with('=') {
            return None;
        }
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level > 6 {
            return None;
        }
        let rest = &trimmed[level..];
        if !rest.starts_with(' ') {
            return None;
        }
        let text = rest.trim();
        self.advance();
        Some(Block::Heading {
            level: level as u8,
            content: parse_adoc_inlines(text),
        })
    }

    fn is_source_block_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("[source") && trimmed.ends_with(']')
    }

    fn parse_source_block(&mut self) -> Result<Block, ParseError> {
        let attr_line = self.current_line().trim().to_string();
        // Extract language from [source,lang] or [source, lang]
        let language = if let Some(comma_pos) = attr_line.find(',') {
            let lang = attr_line[comma_pos + 1..attr_line.len() - 1].trim();
            if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            }
        } else {
            None
        };
        self.advance();

        // Expect ---- delimiter
        if !self.at_end() && self.current_line().trim() == "----" {
            self.advance();
        }

        let mut content_lines = Vec::new();
        while !self.at_end() {
            let l = self.current_line().to_string();
            if l.trim() == "----" {
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

    fn parse_listing_block(&mut self) -> Result<Block, ParseError> {
        // ---- without [source] = plain listing block
        self.advance(); // skip opening ----
        let mut content_lines = Vec::new();
        while !self.at_end() {
            let l = self.current_line().to_string();
            if l.trim() == "----" {
                self.advance();
                break;
            }
            content_lines.push(l);
            self.advance();
        }
        Ok(Block::CodeBlock {
            language: None,
            content: content_lines.join("\n"),
        })
    }

    fn is_blockquote_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        !trimmed.is_empty() && trimmed.chars().all(|c| c == '_') && trimmed.len() >= 4
    }

    fn parse_blockquote(&mut self) -> Result<Block, ParseError> {
        self.advance(); // skip opening ____
        let mut inner_lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.chars().all(|c| c == '_') && trimmed.len() >= 4 {
                self.advance(); // skip closing ____
                break;
            }
            inner_lines.push(line);
            self.advance();
        }
        let inner_text = inner_lines.join("\n");
        let mut inner_parser = AsciiDocParserState::new(&inner_text);
        let children = inner_parser.parse_blocks()?;
        Ok(Block::BlockQuote { children })
    }

    fn parse_table(&mut self) -> Result<Block, ParseError> {
        self.advance(); // skip opening |===

        let mut all_rows: Vec<Vec<TableCell>> = Vec::new();

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = line.trim();

            if trimmed == "|===" {
                self.advance();
                break;
            }

            if trimmed.is_empty() {
                self.advance();
                continue;
            }

            if trimmed.starts_with('|') || line_has_cell_delimiter(trimmed) {
                let row = self.parse_table_row(trimmed);
                all_rows.push(row);
                self.advance();
            } else {
                self.advance();
            }
        }

        let (headers, rows) = if all_rows.len() > 1 {
            let headers = all_rows.remove(0);
            (headers, all_rows)
        } else if all_rows.len() == 1 {
            (all_rows.remove(0), Vec::new())
        } else {
            (Vec::new(), Vec::new())
        };

        let alignments = vec![ColumnAlignment::Default; headers.len()];
        Ok(Block::Table {
            headers,
            alignments,
            rows,
        })
    }

    fn parse_table_row(&self, line: &str) -> Vec<TableCell> {
        let mut cells = Vec::new();
        // Scan for cells: [span_prefix]|content
        // The span prefix (e.g., 2+, .2+, 2.3+) appears BEFORE the | delimiter
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= chars.len() {
                break;
            }

            // Try to detect span prefix before |
            let mut colspan = 1u32;
            let mut rowspan = 1u32;

            // Look for pattern: [digits][.digits]+|
            let segment: String = chars[i..].iter().collect();
            let (cs, rs, rest) = parse_cell_span_prefix(&segment);
            if (cs > 1 || rs > 1) && rest.starts_with('|') {
                // Skip past the span prefix to the |
                colspan = cs;
                rowspan = rs;
                i += segment.len() - rest.len();
            } else if chars[i] == '|' {
                // No span prefix, just |
            } else {
                // Not a cell start, skip
                i += 1;
                continue;
            }

            if i >= chars.len() || chars[i] != '|' {
                i += 1;
                continue;
            }

            i += 1; // skip |

            // Find the end of cell content (next span prefix + | or next | or end of line)
            let content_start = i;
            let mut content_end = chars.len();

            // Scan forward to find the next cell boundary
            let mut j = i;
            while j < chars.len() {
                if chars[j] == '|' {
                    content_end = j;
                    break;
                }
                // Check if we're at a span prefix followed by |
                let remaining: String = chars[j..].iter().collect();
                let (cs2, rs2, rest2) = parse_cell_span_prefix(&remaining);
                if (cs2 > 1 || rs2 > 1) && rest2.starts_with('|') {
                    content_end = j;
                    break;
                }
                j += 1;
            }

            let content: String = chars[content_start..content_end].iter().collect();
            let trimmed = content.trim();

            cells.push(TableCell::with_span(
                parse_adoc_inlines(trimmed),
                colspan,
                rowspan,
            ));

            i = content_end;
        }

        cells
    }

    fn is_unordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        // * item, ** item, *** item, etc.
        let stars = trimmed.chars().take_while(|&c| c == '*').count();
        stars >= 1 && trimmed.len() > stars && trimmed.as_bytes().get(stars) == Some(&b' ')
    }

    fn parse_unordered_list(&mut self, depth: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Check if next non-empty line continues the list
                let saved = self.pos;
                self.advance();
                while !self.at_end() && self.current_line().trim().is_empty() {
                    self.advance();
                }
                if !self.at_end() {
                    let next = self.current_line().trim().to_string();
                    let next_stars = next.chars().take_while(|&c| c == '*').count();
                    if next_stars >= depth
                        && next.len() > next_stars
                        && next.as_bytes().get(next_stars) == Some(&b' ')
                    {
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let stars = trimmed.chars().take_while(|&c| c == '*').count();
            if stars > 0 && trimmed.len() > stars && trimmed.as_bytes().get(stars) == Some(&b' ') {
                if stars < depth {
                    break;
                }
                if stars > depth {
                    let sub = self.parse_unordered_list(stars)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                // Same depth
                let text = &trimmed[stars + 1..];
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_adoc_inlines(text),
                    }],
                });
                self.advance();
            } else if self.is_ordered_list_start(&line) {
                // Nested ordered list inside unordered
                let dots = trimmed.chars().take_while(|&c| c == '.').count();
                let sub = self.parse_ordered_list(dots)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub);
                }
            } else {
                break;
            }
        }

        Ok(Block::UnorderedList { items })
    }

    fn is_ordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let dots = trimmed.chars().take_while(|&c| c == '.').count();
        dots >= 1 && trimmed.len() > dots && trimmed.as_bytes().get(dots) == Some(&b' ')
    }

    fn parse_ordered_list(&mut self, depth: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = line.trim();

            if trimmed.is_empty() {
                let saved = self.pos;
                self.advance();
                while !self.at_end() && self.current_line().trim().is_empty() {
                    self.advance();
                }
                if !self.at_end() {
                    let next = self.current_line().trim().to_string();
                    let next_dots = next.chars().take_while(|&c| c == '.').count();
                    if next_dots >= depth
                        && next.len() > next_dots
                        && next.as_bytes().get(next_dots) == Some(&b' ')
                    {
                        continue;
                    }
                }
                self.pos = saved;
                break;
            }

            let dots = trimmed.chars().take_while(|&c| c == '.').count();
            if dots > 0 && trimmed.len() > dots && trimmed.as_bytes().get(dots) == Some(&b' ') {
                if dots < depth {
                    break;
                }
                if dots > depth {
                    let sub = self.parse_ordered_list(dots)?;
                    if let Some(last) = items.last_mut() {
                        last.content.push(sub);
                    }
                    continue;
                }
                let text = &trimmed[dots + 1..];
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_adoc_inlines(text),
                    }],
                });
                self.advance();
            } else if self.is_unordered_list_start(&line) {
                let stars = trimmed.chars().take_while(|&c| c == '*').count();
                let sub = self.parse_unordered_list(stars)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub);
                }
            } else {
                break;
            }
        }

        Ok(Block::OrderedList { start: 1, items })
    }

    fn is_description_list_start(&self) -> bool {
        let line = self.current_line().trim().to_string();
        line.ends_with("::")
    }

    fn parse_description_list(&mut self) -> Result<Block, ParseError> {
        let mut items = Vec::new();
        while !self.at_end() {
            let line = self.current_line().trim().to_string();
            if !line.ends_with("::") {
                break;
            }
            let term_text = &line[..line.len() - 2];
            let term = parse_adoc_inlines(term_text);
            self.advance();

            let mut def_lines = Vec::new();
            while !self.at_end() {
                let dl = self.current_line().to_string();
                if dl.trim().is_empty() {
                    self.advance();
                    break;
                }
                if dl.trim().ends_with("::") && !dl.starts_with(' ') && !dl.starts_with('\t') {
                    break;
                }
                def_lines.push(dl.trim().to_string());
                self.advance();
            }

            let def_text = def_lines.join("\n");
            let definition = vec![Block::Paragraph {
                content: parse_adoc_inlines(&def_text),
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
            // Stop at block-level constructs
            let trimmed = line.trim();
            if trimmed.starts_with('=')
                && trimmed
                    .chars()
                    .nth(trimmed.chars().take_while(|&c| c == '=').count())
                    == Some(' ')
            {
                break;
            }
            if trimmed == "'''" || trimmed == "----" {
                break;
            }
            if trimmed.starts_with("[source") {
                break;
            }
            if !trimmed.is_empty() && trimmed.chars().all(|c| c == '_') && trimmed.len() >= 4 {
                break;
            }
            if trimmed == "|===" {
                break;
            }
            if self.is_unordered_list_start(&line) || self.is_ordered_list_start(&line) {
                break;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                break;
            }
            lines.push(line);
            self.advance();
        }
        let text = lines.join("\n");
        Ok(Block::Paragraph {
            content: parse_adoc_inlines(&text),
        })
    }
}

// --- Inline parsing for AsciiDoc ---

fn parse_adoc_inlines(input: &str) -> Vec<Inline> {
    let mut parser = AdocInlineParser::new(input);
    parser.parse()
}

struct AdocInlineParser {
    chars: Vec<char>,
    pos: usize,
}

impl AdocInlineParser {
    fn new(input: &str) -> Self {
        AdocInlineParser {
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
            '*' => self.parse_bold(),
            '_' => self.parse_italic(),
            '`' => self.parse_inline_code(),
            '^' => self.parse_superscript(),
            '~' => self.parse_subscript(),
            '[' => self.parse_role_or_text(),
            '<' => self.parse_angle_link(),
            '\n' => self.parse_line_break(),
            _ => self.parse_text_or_link(),
        }
    }

    fn parse_bold(&mut self) -> Option<Inline> {
        // Check for *_bold italic_*
        if self.peek(1) == Some('_') {
            self.pos += 2;
            if let Some(content) = self.parse_until_str("_*") {
                let inlines = AdocInlineParser::new(&content).parse();
                return Some(Inline::BoldItalic(inlines));
            }
            self.pos -= 2;
        }
        self.pos += 1;
        if let Some(content) = self.parse_until_char('*') {
            let inlines = AdocInlineParser::new(&content).parse();
            return Some(Inline::Bold(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("*".to_string()))
    }

    fn parse_italic(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_char('_') {
            let inlines = AdocInlineParser::new(&content).parse();
            return Some(Inline::Italic(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("_".to_string()))
    }

    fn parse_inline_code(&mut self) -> Option<Inline> {
        // Handle doubled backticks: ``code``
        if self.peek(1) == Some('`') {
            self.pos += 2;
            if let Some(content) = self.parse_until_str("``") {
                return Some(Inline::InlineCode(self.unescape_code(&content)));
            }
            self.pos -= 2;
        }

        self.pos += 1;
        // Collect content until closing backtick
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

        Some(Inline::InlineCode(self.unescape_code(&content)))
    }

    fn unescape_code(&self, code: &str) -> String {
        // Handle +code+ passthrough: strip surrounding + if present
        if code.starts_with('+') && code.ends_with('+') && code.len() >= 2 {
            return code[1..code.len() - 1].to_string();
        }
        // Handle pass:c[code]
        if let Some(rest) = code.strip_prefix("pass:c[")
            && let Some(inner) = rest.strip_suffix(']')
        {
            return inner.to_string();
        }
        code.to_string()
    }

    fn parse_superscript(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_char('^') {
            let inlines = AdocInlineParser::new(&content).parse();
            return Some(Inline::Superscript(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("^".to_string()))
    }

    fn parse_subscript(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_char('~') {
            let inlines = AdocInlineParser::new(&content).parse();
            return Some(Inline::Subscript(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("~".to_string()))
    }

    fn parse_role_or_text(&mut self) -> Option<Inline> {
        // [line-through]#text# => strikethrough
        let remaining: String = self.chars[self.pos..].iter().collect();
        if remaining.starts_with("[line-through]#") {
            self.pos += "[line-through]#".len();
            if let Some(content) = self.parse_until_char('#') {
                let inlines = AdocInlineParser::new(&content).parse();
                return Some(Inline::Strikethrough(inlines));
            }
            self.pos -= "[line-through]#".len();
        }
        self.pos += 1;
        Some(Inline::Text("[".to_string()))
    }

    fn parse_angle_link(&mut self) -> Option<Inline> {
        // <<anchor,text>> => internal link
        if self.peek(1) == Some('<') {
            self.pos += 2;
            let start = self.pos;
            while !self.at_end() && !(self.current() == '>' && self.peek(1) == Some('>')) {
                self.pos += 1;
            }
            if !self.at_end() {
                let content: String = self.chars[start..self.pos].iter().collect();
                self.pos += 2; // skip >>
                if let Some(comma) = content.find(',') {
                    let anchor = &content[..comma];
                    let text = content[comma + 1..].trim();
                    return Some(Inline::Link {
                        url: format!("#{anchor}"),
                        text: AdocInlineParser::new(text).parse(),
                        title: None,
                    });
                }
                return Some(Inline::Link {
                    url: format!("#{content}"),
                    text: vec![Inline::Text(content.clone())],
                    title: None,
                });
            }
            self.pos = start - 2;
        }
        self.pos += 1;
        Some(Inline::Text("<".to_string()))
    }

    fn parse_line_break(&mut self) -> Option<Inline> {
        // Check for " +\n" hard line break
        if self.pos >= 2 && self.chars[self.pos - 1] == '+' && self.chars[self.pos - 2] == ' ' {
            self.pos += 1;
            return Some(Inline::HardLineBreak);
        }
        self.pos += 1;
        Some(Inline::SoftLineBreak)
    }

    fn parse_text_or_link(&mut self) -> Option<Inline> {
        let start = self.pos;

        // Check for URL pattern: http:// or https:// or link:
        let remaining: String = self.chars[self.pos..].iter().collect();

        if remaining.starts_with("image:") {
            return self.parse_image_macro();
        }

        if remaining.starts_with("link:") {
            return self.parse_link_macro();
        }

        if remaining.starts_with("http://")
            || remaining.starts_with("https://")
            || remaining.starts_with("ftp://")
        {
            return self.parse_url_link();
        }

        // Regular text
        while !self.at_end() {
            match self.current() {
                '*' | '_' | '`' | '^' | '~' | '[' | '<' | '\n' => break,
                _ => {
                    // Check if we're at a URL start
                    let rem: String = self.chars[self.pos..].iter().collect();
                    if rem.starts_with("http://")
                        || rem.starts_with("https://")
                        || rem.starts_with("link:")
                        || rem.starts_with("image:")
                    {
                        break;
                    }
                    self.pos += 1;
                }
            }
        }

        if self.pos == start {
            self.pos += 1;
            return Some(Inline::Text(self.chars[start].to_string()));
        }

        let text: String = self.chars[start..self.pos].iter().collect();
        // Handle {nbsp} -> non-breaking space
        let text = text.replace("{nbsp}", "\u{a0}");
        Some(Inline::Text(text))
    }

    fn parse_url_link(&mut self) -> Option<Inline> {
        let start = self.pos;
        // Consume URL until [ or whitespace
        while !self.at_end() && self.current() != '[' && !self.current().is_whitespace() {
            self.pos += 1;
        }
        let url: String = self.chars[start..self.pos].iter().collect();

        if !self.at_end() && self.current() == '[' {
            self.pos += 1; // skip [
            if let Some(text) = self.parse_bracket_content() {
                // Unquote if needed
                let text = if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text[1..text.len() - 1].to_string()
                } else {
                    text
                };
                return Some(Inline::Link {
                    url,
                    text: AdocInlineParser::new(&text).parse(),
                    title: None,
                });
            }
            self.pos -= 1; // back to [
        }

        // Bare URL without text
        Some(Inline::Link {
            url: url.clone(),
            text: vec![Inline::Text(url)],
            title: None,
        })
    }

    fn parse_link_macro(&mut self) -> Option<Inline> {
        let start = self.pos;
        self.pos += 5; // skip "link:"
        let url_start = self.pos;
        while !self.at_end() && self.current() != '[' && !self.current().is_whitespace() {
            self.pos += 1;
        }
        let url: String = self.chars[url_start..self.pos].iter().collect();

        if !self.at_end() && self.current() == '[' {
            self.pos += 1;
            if let Some(text) = self.parse_bracket_content() {
                let text = if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text[1..text.len() - 1].to_string()
                } else {
                    text
                };
                return Some(Inline::Link {
                    url,
                    text: AdocInlineParser::new(&text).parse(),
                    title: None,
                });
            }
            self.pos -= 1;
        }

        self.pos = start;
        self.pos += 1;
        Some(Inline::Text("l".to_string()))
    }

    fn parse_image_macro(&mut self) -> Option<Inline> {
        let start = self.pos;
        self.pos += 6; // skip "image:"
        let url_start = self.pos;
        while !self.at_end() && self.current() != '[' && !self.current().is_whitespace() {
            self.pos += 1;
        }
        let url: String = self.chars[url_start..self.pos].iter().collect();

        if !self.at_end() && self.current() == '[' {
            self.pos += 1;
            if let Some(attrs) = self.parse_bracket_content() {
                // Parse alt text and optional link= attribute
                let (alt_text, link) = parse_image_attrs(&attrs);
                return Some(Inline::Image {
                    url,
                    alt: AdocInlineParser::new(&alt_text).parse(),
                    title: None,
                    link,
                });
            }
            self.pos -= 1;
        }

        self.pos = start;
        self.pos += 1;
        Some(Inline::Text("i".to_string()))
    }

    fn parse_bracket_content(&mut self) -> Option<String> {
        let start = self.pos;
        let mut depth = 1;
        let mut content = Vec::new();
        while !self.at_end() && depth > 0 {
            match self.current() {
                '[' => {
                    depth += 1;
                    content.push('[');
                }
                ']' => {
                    depth -= 1;
                    if depth > 0 {
                        content.push(']');
                    }
                }
                _ => content.push(self.current()),
            }
            self.pos += 1;
        }
        if depth != 0 {
            self.pos = start;
            return None;
        }
        Some(content.iter().collect())
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

fn parse_image_attrs(attrs: &str) -> (String, Option<String>) {
    // Format: "alt text" or "alt text,link=url" or alt text,link=url
    let mut alt = attrs.to_string();
    let mut link = None;

    // Check for link= attribute
    if let Some(link_idx) = attrs.find(",link=") {
        alt = attrs[..link_idx].to_string();
        link = Some(attrs[link_idx + 6..].to_string());
    } else if let Some(link_idx) = attrs.find("link=")
        && link_idx == 0
    {
        link = Some(attrs[5..].to_string());
        alt = String::new();
    }

    // Unquote alt text
    if alt.starts_with('"') && alt.ends_with('"') && alt.len() >= 2 {
        alt = alt[1..alt.len() - 1].to_string();
    }

    (alt, link)
}

/// Check if a line contains a cell delimiter (span prefix followed by |)
fn line_has_cell_delimiter(line: &str) -> bool {
    let (cs, rs, rest) = parse_cell_span_prefix(line);
    (cs > 1 || rs > 1) && rest.starts_with('|')
}

/// Parse AsciiDoc cell span prefix: `2+`, `.3+`, `2.3+`
/// Returns (colspan, rowspan, remaining_content)
fn parse_cell_span_prefix(cell: &str) -> (u32, u32, &str) {
    let bytes = cell.as_bytes();
    let mut i = 0;
    let mut colspan = 1u32;
    let mut rowspan = 1u32;

    // Try to parse: [digits][.digits]+
    // First, check for leading digits (colspan)
    let col_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let has_col_digits = i > col_start;
    let col_end = i;

    // Check for dot (rowspan prefix)
    let mut has_dot = false;
    if i < bytes.len() && bytes[i] == b'.' {
        has_dot = true;
        i += 1;
        let row_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > row_start {
            if let Ok(n) = cell[row_start..i].parse::<u32>() {
                rowspan = n;
            }
        } else {
            // dot without digits - not a span prefix
            return (1, 1, cell);
        }
    }

    // Must end with +
    if i < bytes.len() && bytes[i] == b'+' {
        if has_col_digits && let Ok(n) = cell[col_start..col_end].parse::<u32>() {
            colspan = n;
        }
        if !has_col_digits && !has_dot {
            return (1, 1, cell);
        }
        return (colspan, rowspan, &cell[i + 1..]);
    }

    (1, 1, cell)
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
