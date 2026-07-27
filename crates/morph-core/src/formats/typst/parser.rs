use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;

pub struct TypstParser;

impl Parser for TypstParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let mut state = TypstParserState::new(input);
        let children = state.parse_blocks()?;
        Ok(Document { children })
    }
}

struct TypstParserState {
    lines: Vec<String>,
    pos: usize,
}

impl TypstParserState {
    fn new(input: &str) -> Self {
        TypstParserState {
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

            if line.trim().starts_with("```") {
                blocks.push(self.parse_code_block()?);
                continue;
            }

            if line.trim().starts_with("#line(") {
                blocks.push(Block::HorizontalRule);
                self.advance();
                continue;
            }

            if line.trim().starts_with("#quote") {
                blocks.push(self.parse_quote()?);
                continue;
            }

            if line.trim().starts_with("#table(") || line.trim().starts_with("#table(\n") {
                blocks.push(self.parse_table()?);
                continue;
            }

            if self.is_unordered_list_start(&line) {
                blocks.push(self.parse_unordered_list()?);
                continue;
            }

            if self.is_ordered_list_start(&line) {
                blocks.push(self.parse_ordered_list()?);
                continue;
            }

            if line.trim().starts_with("#image(")
                && let Some(block) = self.parse_image_block()?
            {
                blocks.push(block);
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
            content: parse_typst_inlines(text),
        })
    }

    fn parse_code_block(&mut self) -> Result<Block, ParseError> {
        let line = self.current_line().trim().to_string();
        let after_backticks = &line[3..];
        let language = if after_backticks.trim().is_empty() {
            None
        } else {
            Some(after_backticks.trim().to_string())
        };
        self.advance();

        let mut content_lines = Vec::new();
        while !self.at_end() {
            let l = self.current_line().to_string();
            if l.trim() == "```" {
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

    fn parse_quote(&mut self) -> Result<Block, ParseError> {
        let line = self.current_line().trim().to_string();

        // #quote[content] on single line
        if let Some(content) = extract_bracket_content(&line, "#quote") {
            self.advance();
            let mut inner_parser = TypstParserState::new(&content);
            let children = inner_parser.parse_blocks()?;
            if children.is_empty() {
                return Ok(Block::BlockQuote {
                    children: vec![Block::Paragraph {
                        content: parse_typst_inlines(&content),
                    }],
                });
            }
            return Ok(Block::BlockQuote { children });
        }

        // Multi-line #quote[
        if line.starts_with("#quote[") {
            self.advance();
            let mut inner_lines = Vec::new();
            // Determine indentation to strip (from the first content line)
            let mut base_indent: Option<usize> = None;
            while !self.at_end() {
                let l = self.current_line().to_string();
                if l.trim() == "]" {
                    self.advance();
                    break;
                }
                if l.trim().ends_with(']') {
                    let stripped = l.trim().trim_end_matches(']').to_string();
                    inner_lines.push(stripped);
                    self.advance();
                    break;
                }
                // Calculate indentation to strip
                if !l.trim().is_empty() && base_indent.is_none() {
                    base_indent = Some(l.len() - l.trim_start().len());
                }
                let indent = base_indent.unwrap_or(0);
                if l.len() >= indent && l[..indent].trim().is_empty() {
                    inner_lines.push(l[indent..].to_string());
                } else {
                    inner_lines.push(l.trim().to_string());
                }
                self.advance();
            }
            let inner_text = inner_lines.join("\n");
            let mut inner_parser = TypstParserState::new(&inner_text);
            let children = inner_parser.parse_blocks()?;
            return Ok(Block::BlockQuote { children });
        }

        self.advance();
        Ok(Block::BlockQuote { children: vec![] })
    }

    fn parse_table(&mut self) -> Result<Block, ParseError> {
        // Collect all lines of the #table(...) call
        let mut table_text = String::new();
        let mut depth = 0;
        while !self.at_end() {
            let line = self.current_line().to_string();
            for ch in line.chars() {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                table_text.push(ch);
            }
            table_text.push('\n');
            self.advance();
            if depth <= 0 {
                break;
            }
        }

        // Parse columns parameter
        let columns = parse_table_columns(&table_text);

        // Extract all cell info (content + span data)
        let cell_infos = extract_table_cells(&table_text);

        if columns == 0 || cell_infos.is_empty() {
            return Ok(Block::Table {
                headers: vec![],
                alignments: vec![],
                rows: vec![],
            });
        }

        let mut headers = Vec::new();
        let mut rows: Vec<Vec<TableCell>> = Vec::new();
        let mut col_pos = 0; // track logical column position

        for info in &cell_infos {
            let cell = TableCell::with_span(
                parse_typst_inlines(&info.content),
                info.colspan,
                info.rowspan,
            );
            let span = info.colspan as usize;
            if col_pos < columns {
                headers.push(cell);
                col_pos += span;
            } else {
                let logical_col = col_pos - columns;
                if logical_col.is_multiple_of(columns) || rows.is_empty() {
                    rows.push(Vec::new());
                }
                rows.last_mut().unwrap().push(cell);
                col_pos += span;
                // If we've filled a row, reset for next
                if (col_pos - columns).is_multiple_of(columns) {
                    // row is complete
                }
            }
        }

        let alignments = vec![ColumnAlignment::Default; columns];
        Ok(Block::Table {
            headers,
            alignments,
            rows,
        })
    }

    fn is_unordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("- ") && !trimmed.starts_with("---")
    }

    fn parse_unordered_list(&mut self) -> Result<Block, ParseError> {
        let base_indent = self.current_line().len() - self.current_line().trim_start().len();
        self.parse_unordered_list_at_indent(base_indent)
    }

    fn parse_unordered_list_at_indent(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();

            if line.trim().is_empty() {
                break;
            }

            let indent = line.len() - line.trim_start().len();

            // Deeper indented unordered list -> sub-list
            if indent > base_indent && self.is_unordered_list_start(&line) {
                let sub_list = self.parse_unordered_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Deeper indented ordered list -> sub-list
            if indent > base_indent && self.is_ordered_list_start(&line) {
                let sub_list = self.parse_ordered_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Same indent, unordered list start -> new item
            if indent == base_indent && self.is_unordered_list_start(&line) {
                let trimmed = line.trim_start();
                let text = &trimmed[2..];
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_typst_inlines(text),
                    }],
                });
                self.advance();
                continue;
            }

            // Continuation line
            if indent > base_indent && !items.is_empty() {
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(parse_typst_inlines(line.trim()));
                }
                self.advance();
                continue;
            }

            break;
        }
        Ok(Block::UnorderedList { items })
    }

    fn is_ordered_list_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.starts_with("+ ")
    }

    fn parse_ordered_list(&mut self) -> Result<Block, ParseError> {
        let base_indent = self.current_line().len() - self.current_line().trim_start().len();
        self.parse_ordered_list_at_indent(base_indent)
    }

    fn parse_ordered_list_at_indent(&mut self, base_indent: usize) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();

            if line.trim().is_empty() {
                break;
            }

            let indent = line.len() - line.trim_start().len();

            // Deeper indented unordered list -> sub-list
            if indent > base_indent && self.is_unordered_list_start(&line) {
                let sub_list = self.parse_unordered_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Deeper indented ordered list -> sub-list
            if indent > base_indent && self.is_ordered_list_start(&line) {
                let sub_list = self.parse_ordered_list_at_indent(indent)?;
                if let Some(last) = items.last_mut() {
                    last.content.push(sub_list);
                }
                continue;
            }

            // Same indent, ordered list start -> new item
            if indent == base_indent && self.is_ordered_list_start(&line) {
                let trimmed = line.trim_start();
                let text = &trimmed[2..];
                items.push(ListItem {
                    content: vec![Block::Paragraph {
                        content: parse_typst_inlines(text),
                    }],
                });
                self.advance();
                continue;
            }

            // Continuation line
            if indent > base_indent && !items.is_empty() {
                if let Some(last) = items.last_mut()
                    && let Some(Block::Paragraph { content }) = last.content.last_mut()
                {
                    content.push(Inline::SoftLineBreak);
                    content.extend(parse_typst_inlines(line.trim()));
                }
                self.advance();
                continue;
            }

            break;
        }
        Ok(Block::OrderedList { start: 1, items })
    }

    fn parse_image_block(&mut self) -> Result<Option<Block>, ParseError> {
        let line = self.current_line().trim().to_string();
        // #image("url") or #image("url", alt: "text")
        if let Some(args) = extract_function_args(&line, "#image") {
            self.advance();
            let (url, _) = parse_string_arg(&args);
            return Ok(Some(Block::Paragraph {
                content: vec![Inline::Image {
                    url,
                    alt: vec![],
                    title: None,
                    link: None,
                }],
            }));
        }
        self.advance();
        Ok(None)
    }

    fn parse_paragraph(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if line.trim().is_empty() {
                break;
            }
            let trimmed = line.trim();
            // Stop at block constructs
            if trimmed.starts_with('=')
                && trimmed
                    .chars()
                    .nth(trimmed.chars().take_while(|&c| c == '=').count())
                    == Some(' ')
            {
                break;
            }
            if trimmed.starts_with("```")
                || trimmed.starts_with("#line(")
                || trimmed.starts_with("#quote")
                || trimmed.starts_with("#table(")
                || trimmed.starts_with("#image(")
            {
                break;
            }
            if self.is_unordered_list_start(&line) || self.is_ordered_list_start(&line) {
                break;
            }
            lines.push(line);
            self.advance();
        }
        let text = lines.join("\n");
        Ok(Block::Paragraph {
            content: parse_typst_inlines(&text),
        })
    }
}

// --- Typst inline parsing ---

fn parse_typst_inlines(input: &str) -> Vec<Inline> {
    let mut parser = TypstInlineParser::new(input);
    parser.parse()
}

struct TypstInlineParser {
    chars: Vec<char>,
    pos: usize,
}

impl TypstInlineParser {
    fn new(input: &str) -> Self {
        TypstInlineParser {
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

    fn remaining(&self) -> String {
        self.chars[self.pos..].iter().collect()
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
            '`' => self.parse_raw(),
            '#' => self.parse_function_call(),
            '\\' => self.parse_escape(),
            '\n' => self.parse_line_break(),
            _ => self.parse_text(),
        }
    }

    fn parse_bold(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_char('*') {
            let inlines = TypstInlineParser::new(&content).parse();
            return Some(Inline::Bold(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("*".to_string()))
    }

    fn parse_italic(&mut self) -> Option<Inline> {
        self.pos += 1;
        if let Some(content) = self.parse_until_char('_') {
            let inlines = TypstInlineParser::new(&content).parse();
            return Some(Inline::Italic(inlines));
        }
        self.pos -= 1;
        self.pos += 1;
        Some(Inline::Text("_".to_string()))
    }

    fn parse_raw(&mut self) -> Option<Inline> {
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
        self.pos += 1;
        Some(Inline::InlineCode(content))
    }

    fn parse_function_call(&mut self) -> Option<Inline> {
        let remaining = self.remaining();

        // #strike[text]
        if remaining.starts_with("#strike[") {
            self.pos += "#strike[".len();
            if let Some(content) = self.parse_bracket_content() {
                let inlines = TypstInlineParser::new(&content).parse();
                return Some(Inline::Strikethrough(inlines));
            }
            self.pos -= "#strike[".len();
        }

        // #super[text]
        if remaining.starts_with("#super[") {
            self.pos += "#super[".len();
            if let Some(content) = self.parse_bracket_content() {
                let inlines = TypstInlineParser::new(&content).parse();
                return Some(Inline::Superscript(inlines));
            }
            self.pos -= "#super[".len();
        }

        // #sub[text]
        if remaining.starts_with("#sub[") {
            self.pos += "#sub[".len();
            if let Some(content) = self.parse_bracket_content() {
                let inlines = TypstInlineParser::new(&content).parse();
                return Some(Inline::Subscript(inlines));
            }
            self.pos -= "#sub[".len();
        }

        // #link("url")[text]
        if remaining.starts_with("#link(") {
            self.pos += "#link(".len();
            // Parse URL in quotes
            if !self.at_end() && self.current() == '"' {
                self.pos += 1;
                let url_start = self.pos;
                while !self.at_end() && self.current() != '"' {
                    self.pos += 1;
                }
                if !self.at_end() {
                    let url: String = self.chars[url_start..self.pos].iter().collect();
                    self.pos += 1; // skip closing "
                    // Skip )
                    if !self.at_end() && self.current() == ')' {
                        self.pos += 1;
                    }
                    // Check for [text]
                    if !self.at_end() && self.current() == '[' {
                        self.pos += 1;
                        if let Some(text) = self.parse_bracket_content() {
                            return Some(Inline::Link {
                                url,
                                text: TypstInlineParser::new(&text).parse(),
                                title: None,
                            });
                        }
                    }
                    return Some(Inline::Link {
                        url: url.clone(),
                        text: vec![Inline::Text(url)],
                        title: None,
                    });
                }
            }
            self.pos -= "#link(".len();
        }

        // #image("url")
        if remaining.starts_with("#image(") {
            self.pos += "#image(".len();
            if !self.at_end() && self.current() == '"' {
                self.pos += 1;
                let url_start = self.pos;
                while !self.at_end() && self.current() != '"' {
                    self.pos += 1;
                }
                if !self.at_end() {
                    let url: String = self.chars[url_start..self.pos].iter().collect();
                    self.pos += 1; // skip closing "
                    // Skip to closing )
                    while !self.at_end() && self.current() != ')' {
                        self.pos += 1;
                    }
                    if !self.at_end() {
                        self.pos += 1; // skip )
                    }
                    return Some(Inline::Image {
                        url,
                        alt: vec![],
                        title: None,
                        link: None,
                    });
                }
            }
            self.pos -= "#image(".len();
        }

        // #strong[text] (alternative bold)
        if remaining.starts_with("#strong[") {
            self.pos += "#strong[".len();
            if let Some(content) = self.parse_bracket_content() {
                let inlines = TypstInlineParser::new(&content).parse();
                return Some(Inline::Bold(inlines));
            }
            self.pos -= "#strong[".len();
        }

        // #emph[text] (alternative italic)
        if remaining.starts_with("#emph[") {
            self.pos += "#emph[".len();
            if let Some(content) = self.parse_bracket_content() {
                let inlines = TypstInlineParser::new(&content).parse();
                return Some(Inline::Italic(inlines));
            }
            self.pos -= "#emph[".len();
        }

        self.pos += 1;
        Some(Inline::Text("#".to_string()))
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
        // Check for explicit line break: \
        if self.pos >= 1 && self.chars[self.pos - 1] == '\\' {
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
                '*' | '_' | '`' | '#' | '\\' | '\n' => break,
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
}

// --- Helper functions ---

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

fn extract_bracket_content(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    if !rest.starts_with('[') {
        return None;
    }
    let inner = &rest[1..];
    let mut depth = 1;
    let mut content = Vec::new();
    for ch in inner.chars() {
        match ch {
            '[' => {
                depth += 1;
                content.push(ch);
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push(ch);
            }
            _ => content.push(ch),
        }
    }
    if depth == 0 {
        Some(content.iter().collect())
    } else {
        None
    }
}

fn extract_function_args(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    if !rest.starts_with('(') {
        return None;
    }
    let inner = &rest[1..];
    let mut depth = 1;
    let mut content = Vec::new();
    for ch in inner.chars() {
        match ch {
            '(' => {
                depth += 1;
                content.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push(ch);
            }
            _ => content.push(ch),
        }
    }
    if depth == 0 {
        Some(content.iter().collect())
    } else {
        None
    }
}

fn parse_string_arg(args: &str) -> (String, String) {
    let trimmed = args.trim();
    if trimmed.starts_with('"') {
        let end = trimmed[1..].find('"').unwrap_or(trimmed.len() - 1);
        let url = trimmed[1..end + 1].to_string();
        let rest = trimmed[end + 2..].to_string();
        (url, rest)
    } else {
        (trimmed.to_string(), String::new())
    }
}

fn parse_table_columns(text: &str) -> usize {
    // Look for columns: N or columns: (...)
    if let Some(idx) = text.find("columns:") {
        let after = text[idx + 8..].trim_start();
        if after.starts_with('(') {
            // Count entries
            if let Some(close) = after.find(')') {
                let inner = &after[1..close];
                return inner.split(',').count();
            }
        } else {
            // Simple number
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<usize>() {
                return n;
            }
        }
    }
    // Fallback: count cells and guess
    let cell_infos = extract_table_cells(text);
    if cell_infos.len() >= 2 {
        // Heuristic: assume square-ish table
        for n in 2..=cell_infos.len() {
            if cell_infos.len().is_multiple_of(n) {
                return n;
            }
        }
    }
    cell_infos.len()
}

struct CellInfo {
    content: String,
    colspan: u32,
    rowspan: u32,
}

fn extract_table_cells(text: &str) -> Vec<CellInfo> {
    let mut cells = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    // Skip past the #table( prefix to find cell content
    // First, find the opening ( of #table(
    let mut found_table_start = false;
    while i < chars.len() {
        if chars[i] == '(' && !found_table_start {
            found_table_start = true;
            i += 1;
            break;
        }
        i += 1;
    }
    if !found_table_start {
        return cells;
    }

    // Skip past keyword arguments like "columns: 2," before looking for cells
    // Find position after "columns: <value>,"
    let remaining: String = chars[i..].iter().collect();
    if let Some(col_idx) = remaining.find("columns:") {
        let after_key = &remaining[col_idx + 8..];
        let after_key_trimmed = after_key.trim_start();
        // Skip the value (number or parenthesized expression)
        if after_key_trimmed.starts_with('(') {
            // columns: (...), skip to closing paren
            if let Some(close) = after_key_trimmed.find(')') {
                let skip_to = col_idx + 8 + (after_key.len() - after_key_trimmed.len()) + close + 1;
                i += skip_to;
                // Skip past comma
                while i < chars.len() && (chars[i] == ',' || chars[i].is_whitespace()) {
                    i += 1;
                }
            }
        } else {
            // columns: N, skip past number and comma
            let num_end = after_key_trimmed
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after_key_trimmed.len());
            let skip_to = col_idx + 8 + (after_key.len() - after_key_trimmed.len()) + num_end;
            i += skip_to;
            // Skip past comma and whitespace
            while i < chars.len() && (chars[i] == ',' || chars[i].is_whitespace()) {
                i += 1;
            }
        }
    }

    // Now extract cells: either table.cell(...)[content] or plain [content]
    while i < chars.len() {
        let remaining_str: String = chars[i..].iter().collect();

        // Check for table.cell(...)[content]
        if remaining_str.starts_with("table.cell(") {
            i += "table.cell(".len();
            // Parse arguments inside (...)
            let mut colspan = 1u32;
            let mut rowspan = 1u32;
            let mut paren_depth = 1;
            let args_start = i;
            while i < chars.len() && paren_depth > 0 {
                match chars[i] {
                    '(' => paren_depth += 1,
                    ')' => paren_depth -= 1,
                    _ => {}
                }
                if paren_depth > 0 {
                    i += 1;
                }
            }
            let args: String = chars[args_start..i].iter().collect();
            i += 1; // skip closing )

            // Parse colspan and rowspan from args
            for part in args.split(',') {
                let part = part.trim();
                if let Some(val) = part.strip_prefix("colspan:") {
                    if let Ok(n) = val.trim().parse::<u32>() {
                        colspan = n;
                    }
                } else if let Some(val) = part.strip_prefix("rowspan:")
                    && let Ok(n) = val.trim().parse::<u32>()
                {
                    rowspan = n;
                }
            }

            // Now expect [content]
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '[' {
                i += 1;
                let mut depth = 1;
                let mut content = Vec::new();
                while i < chars.len() && depth > 0 {
                    match chars[i] {
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
                        _ => content.push(chars[i]),
                    }
                    i += 1;
                }
                if depth == 0 {
                    cells.push(CellInfo {
                        content: content.iter().collect::<String>().trim().to_string(),
                        colspan,
                        rowspan,
                    });
                }
            }
        } else if chars[i] == '[' {
            // Plain [content] cell
            i += 1;
            let mut depth = 1;
            let mut content = Vec::new();
            while i < chars.len() && depth > 0 {
                match chars[i] {
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
                    _ => content.push(chars[i]),
                }
                i += 1;
            }
            if depth == 0 {
                cells.push(CellInfo {
                    content: content.iter().collect::<String>().trim().to_string(),
                    colspan: 1,
                    rowspan: 1,
                });
            }
        } else {
            i += 1;
        }
    }
    cells
}
