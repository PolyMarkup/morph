use crate::ast::*;
use crate::error::{EmitError, ParseError};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Flavor {
    Djot,
    Org,
    Textile,
}

pub(crate) fn parse_document(input: &str, flavor: Flavor) -> Result<Document, ParseError> {
    let mut parser = LightweightParser {
        lines: input.lines().map(str::to_string).collect(),
        pos: 0,
        flavor,
    };
    Ok(Document {
        children: parser.parse_blocks()?,
    })
}

pub(crate) fn emit_document(doc: &Document, flavor: Flavor) -> Result<String, EmitError> {
    let mut emitter = LightweightEmitter {
        output: String::new(),
        flavor,
    };
    emitter.emit_blocks(&doc.children)?;
    Ok(format!("{}\n", emitter.output.trim_end_matches('\n')))
}

struct LightweightParser {
    lines: Vec<String>,
    pos: usize,
    flavor: Flavor,
}

impl LightweightParser {
    fn parse_blocks(&mut self) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        while self.pos < self.lines.len() {
            if self.lines[self.pos].trim().is_empty() || self.is_ignored_line() {
                self.pos += 1;
                continue;
            }
            if let Some(block) = self.parse_heading() {
                blocks.push(block);
            } else if self.is_horizontal_rule() {
                self.pos += 1;
                blocks.push(Block::HorizontalRule);
            } else if self.is_code_start() {
                blocks.push(self.parse_code()?);
            } else if self.is_quote_start() {
                blocks.push(self.parse_quote()?);
            } else if self.is_table_start() {
                blocks.push(self.parse_table()?);
            } else if self.is_description_start() {
                blocks.push(self.parse_description_list()?);
            } else if self.flavor == Flavor::Textile && self.textile_list_marker().is_some() {
                blocks.push(self.parse_textile_list(1)?);
            } else if let Some((indent, ordered, _, _)) = self.indented_list_marker() {
                blocks.push(self.parse_indented_list(indent, ordered)?);
            } else {
                blocks.push(self.parse_paragraph());
            }
        }
        Ok(blocks)
    }

    fn is_ignored_line(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Org => {
                line.starts_with("#+")
                    && !line.eq_ignore_ascii_case("#+begin_src")
                    && !line.to_ascii_lowercase().starts_with("#+begin_src ")
                    && !line.eq_ignore_ascii_case("#+begin_quote")
            }
            Flavor::Textile => line.starts_with("###."),
            Flavor::Djot => line.starts_with("{%") && line.ends_with("%}"),
        }
    }

    fn parse_heading(&mut self) -> Option<Block> {
        let line = self.lines[self.pos].trim().to_string();
        let (level, content) = match self.flavor {
            Flavor::Djot => {
                let level = line.chars().take_while(|c| *c == '#').count();
                if level == 0 || level > 6 || !line[level..].starts_with(' ') {
                    return None;
                }
                (
                    level as u8,
                    line[level..].trim().trim_end_matches('#').trim(),
                )
            }
            Flavor::Org => {
                let level = line.chars().take_while(|c| *c == '*').count();
                if level == 0 || level > 6 || !line[level..].starts_with(' ') {
                    return None;
                }
                (level as u8, line[level..].trim())
            }
            Flavor::Textile => {
                if !line.starts_with('h') {
                    return None;
                }
                let (prefix, content) = line.split_once(". ")?;
                let level = prefix.strip_prefix('h')?.parse::<u8>().ok()?;
                if !(1..=6).contains(&level) {
                    return None;
                }
                (level, content)
            }
        };
        self.pos += 1;
        Some(Block::Heading {
            level,
            content: parse_inlines(content, self.flavor),
        })
    }

    fn is_horizontal_rule(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Djot => {
                let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
                compact.len() >= 4
                    && (compact.chars().all(|c| c == '-') || compact.chars().all(|c| c == '*'))
            }
            Flavor::Org => line.len() >= 5 && line.chars().all(|c| c == '-'),
            Flavor::Textile => line == "<hr>" || line == "<hr />",
        }
    }

    fn is_code_start(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Djot => line.starts_with("```"),
            Flavor::Org => line.to_ascii_lowercase().starts_with("#+begin_src"),
            Flavor::Textile => {
                line.starts_with("bc. ")
                    || line.starts_with("bc..")
                    || line.starts_with("bc(")
                    || line.starts_with("pre. ")
                    || line.starts_with("pre..")
            }
        }
    }

    fn parse_code(&mut self) -> Result<Block, ParseError> {
        match self.flavor {
            Flavor::Djot => {
                let opening = self.lines[self.pos].trim().to_string();
                let fence_len = opening.chars().take_while(|c| *c == '`').count();
                let info = opening[fence_len..].trim();
                let raw_format = info.strip_prefix('=').map(str::to_string);
                let language = if info.is_empty() || raw_format.is_some() {
                    None
                } else {
                    Some(info.to_string())
                };
                self.pos += 1;
                let mut content = Vec::new();
                while self.pos < self.lines.len() {
                    let line = &self.lines[self.pos];
                    if line.trim().chars().all(|c| c == '`')
                        && line.trim().chars().count() >= fence_len
                    {
                        self.pos += 1;
                        break;
                    }
                    content.push(line.clone());
                    self.pos += 1;
                }
                if let Some(format) = raw_format {
                    Ok(Block::RawBlock {
                        format: Some(format),
                        content: content.join("\n"),
                    })
                } else {
                    Ok(Block::CodeBlock {
                        language,
                        content: content.join("\n"),
                    })
                }
            }
            Flavor::Org => {
                let opening = self.lines[self.pos].trim().to_string();
                let language = opening
                    .split_whitespace()
                    .nth(1)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                self.pos += 1;
                let mut content = Vec::new();
                while self.pos < self.lines.len()
                    && !self.lines[self.pos]
                        .trim()
                        .eq_ignore_ascii_case("#+end_src")
                {
                    content.push(self.lines[self.pos].clone());
                    self.pos += 1;
                }
                if self.pos == self.lines.len() {
                    return Err(ParseError::InvalidInput(
                        "unterminated Org source block".to_string(),
                    ));
                }
                self.pos += 1;
                Ok(Block::CodeBlock {
                    language,
                    content: content.join("\n"),
                })
            }
            Flavor::Textile => {
                let opening = self.lines[self.pos].trim().to_string();
                let extended = opening.starts_with("bc..")
                    || opening.starts_with("pre..")
                    || opening.ends_with(")..");
                let language = opening
                    .strip_prefix("bc(")
                    .and_then(|rest| rest.split_once(')'))
                    .map(|(lang, _)| lang.to_string());
                if !extended {
                    let content = opening
                        .split_once(". ")
                        .map(|(_, value)| value)
                        .unwrap_or("")
                        .to_string();
                    self.pos += 1;
                    return Ok(Block::CodeBlock { language, content });
                }
                self.pos += 1;
                let mut content = Vec::new();
                while self.pos < self.lines.len() {
                    let line = self.lines[self.pos].clone();
                    if line == "p." || line.starts_with("p. ") {
                        self.pos += 1;
                        break;
                    }
                    if line.trim().is_empty()
                        && self
                            .lines
                            .get(self.pos + 1)
                            .is_some_and(|next| next == "p." || next.starts_with("p. "))
                    {
                        self.pos += 2;
                        break;
                    }
                    content.push(line);
                    self.pos += 1;
                }
                Ok(Block::CodeBlock {
                    language,
                    content: content.join("\n").trim_end().to_string(),
                })
            }
        }
    }

    fn is_quote_start(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Djot => line == ">" || line.starts_with("> "),
            Flavor::Org => line.eq_ignore_ascii_case("#+begin_quote"),
            Flavor::Textile => line.starts_with("bq. ") || line == "bq..",
        }
    }

    fn parse_quote(&mut self) -> Result<Block, ParseError> {
        let mut inner = Vec::new();
        match self.flavor {
            Flavor::Djot => {
                while self.pos < self.lines.len() {
                    let line = self.lines[self.pos].trim_start();
                    if let Some(content) = line.strip_prefix("> ") {
                        inner.push(content.to_string());
                    } else if line == ">" {
                        inner.push(String::new());
                    } else {
                        break;
                    }
                    self.pos += 1;
                }
            }
            Flavor::Org => {
                self.pos += 1;
                while self.pos < self.lines.len()
                    && !self.lines[self.pos]
                        .trim()
                        .eq_ignore_ascii_case("#+end_quote")
                {
                    inner.push(self.lines[self.pos].clone());
                    self.pos += 1;
                }
                if self.pos == self.lines.len() {
                    return Err(ParseError::InvalidInput(
                        "unterminated Org quote block".to_string(),
                    ));
                }
                self.pos += 1;
            }
            Flavor::Textile => {
                let line = self.lines[self.pos].trim().to_string();
                self.pos += 1;
                if let Some(content) = line.strip_prefix("bq. ") {
                    inner.push(content.to_string());
                } else {
                    while self.pos < self.lines.len() {
                        let line = self.lines[self.pos].clone();
                        if line == "p." || line.starts_with("p. ") {
                            self.pos += 1;
                            break;
                        }
                        if line.trim().is_empty()
                            && self
                                .lines
                                .get(self.pos + 1)
                                .is_some_and(|next| next == "p." || next.starts_with("p. "))
                        {
                            self.pos += 2;
                            break;
                        }
                        inner.push(line);
                        self.pos += 1;
                    }
                }
            }
        }
        let mut parser = LightweightParser {
            lines: inner,
            pos: 0,
            flavor: self.flavor,
        };
        Ok(Block::BlockQuote {
            children: parser.parse_blocks()?,
        })
    }

    fn is_table_start(&self) -> bool {
        let line = self.lines[self.pos].trim();
        line.starts_with('|') && line.ends_with('|')
    }

    fn parse_table(&mut self) -> Result<Block, ParseError> {
        let mut raw_rows = Vec::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].trim();
            if !line.starts_with('|') || !line.ends_with('|') {
                break;
            }
            raw_rows.push(split_pipe_row(line));
            self.pos += 1;
        }
        if raw_rows.is_empty() {
            return Err(ParseError::InvalidInput("empty table".to_string()));
        }

        let mut alignments = Vec::new();
        let mut header_index = None;
        let textile_header_index = (self.flavor == Flavor::Textile)
            .then(|| {
                raw_rows
                    .iter()
                    .position(|row| row.iter().all(|cell| cell.starts_with("_. ")))
            })
            .flatten();
        if self.flavor == Flavor::Textile
            && let Some(index) = textile_header_index
        {
            alignments = raw_rows[index]
                .iter()
                .map(|cell| parse_textile_alignment(cell))
                .collect();
        }
        for (index, row) in raw_rows.iter().enumerate() {
            if row.iter().all(|cell| is_table_separator(cell, self.flavor)) {
                header_index = index.checked_sub(1);
                alignments = row.iter().map(|cell| parse_alignment(cell)).collect();
                break;
            }
        }

        let mut data_rows = Vec::new();
        for (index, row) in raw_rows.into_iter().enumerate() {
            if row.iter().all(|cell| is_table_separator(cell, self.flavor)) {
                continue;
            }
            let cells: Vec<TableCell> = row
                .into_iter()
                .map(|cell| parse_table_cell(&cell, self.flavor))
                .collect();
            data_rows.push((index, cells));
        }

        let header_pos = header_index.or(textile_header_index).unwrap_or(0);
        let mut headers = Vec::new();
        let mut rows = Vec::new();
        for (index, cells) in data_rows {
            if index == header_pos {
                headers = cells;
            } else {
                rows.push(cells);
            }
        }
        if alignments.is_empty() {
            alignments = vec![ColumnAlignment::Default; headers.len()];
        }
        Ok(Block::Table {
            headers,
            alignments,
            rows,
        })
    }

    fn indented_list_marker(&self) -> Option<(usize, bool, u32, usize)> {
        if self.flavor == Flavor::Textile {
            return None;
        }
        parse_indented_marker(&self.lines[self.pos], self.flavor)
    }

    fn parse_indented_list(
        &mut self,
        base_indent: usize,
        ordered: bool,
    ) -> Result<Block, ParseError> {
        let mut items: Vec<ListItem> = Vec::new();
        let mut start = 1;
        while self.pos < self.lines.len() {
            let Some((indent, item_ordered, item_start, marker_len)) =
                parse_indented_marker(&self.lines[self.pos], self.flavor)
            else {
                break;
            };
            if indent != base_indent || item_ordered != ordered {
                break;
            }
            if items.is_empty() {
                start = item_start;
            }
            let line = self.lines[self.pos].clone();
            let content = line[indent + marker_len..].trim_start();
            self.pos += 1;
            let mut blocks = vec![Block::Paragraph {
                content: parse_inlines(content, self.flavor),
            }];

            while self.pos < self.lines.len() {
                let Some((nested_indent, nested_ordered, _, _)) =
                    parse_indented_marker(&self.lines[self.pos], self.flavor)
                else {
                    break;
                };
                if nested_indent <= base_indent {
                    break;
                }
                blocks.push(self.parse_indented_list(nested_indent, nested_ordered)?);
            }
            items.push(ListItem { content: blocks });
        }
        Ok(if ordered {
            Block::OrderedList { start, items }
        } else {
            Block::UnorderedList { items }
        })
    }

    fn textile_list_marker(&self) -> Option<(char, usize, u32, &str)> {
        parse_textile_marker(&self.lines[self.pos])
    }

    fn parse_textile_list(&mut self, level: usize) -> Result<Block, ParseError> {
        let Some((kind, _, start, _)) = self.textile_list_marker() else {
            return Err(ParseError::InvalidInput("invalid Textile list".to_string()));
        };
        let ordered = kind == '#';
        let mut items: Vec<ListItem> = Vec::new();
        while self.pos < self.lines.len() {
            let Some((item_kind, item_level, _, content)) =
                parse_textile_marker(&self.lines[self.pos])
            else {
                break;
            };
            if item_level < level {
                break;
            }
            if item_level > level {
                let nested = self.parse_textile_list(item_level)?;
                if let Some(item) = items.last_mut() {
                    item.content.push(nested);
                }
                continue;
            }
            if item_kind != kind {
                break;
            }
            items.push(ListItem {
                content: vec![Block::Paragraph {
                    content: parse_inlines(content, self.flavor),
                }],
            });
            self.pos += 1;
        }
        Ok(if ordered {
            Block::OrderedList { start, items }
        } else {
            Block::UnorderedList { items }
        })
    }

    fn is_description_start(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Djot => line.starts_with(": "),
            Flavor::Org => line.starts_with("- ") && line.contains(" :: "),
            Flavor::Textile => line.starts_with("- ") && line.contains(" := "),
        }
    }

    fn parse_description_list(&mut self) -> Result<Block, ParseError> {
        let mut items = Vec::new();
        while self.pos < self.lines.len() && self.is_description_start() {
            let line = self.lines[self.pos].trim().to_string();
            let (term, definition) = match self.flavor {
                Flavor::Djot => {
                    let term = line.trim_start_matches(": ").to_string();
                    self.pos += 1;
                    while self.pos < self.lines.len() && self.lines[self.pos].trim().is_empty() {
                        self.pos += 1;
                    }
                    let definition = self
                        .lines
                        .get(self.pos)
                        .map(|line| line.trim().to_string())
                        .unwrap_or_default();
                    if self.pos < self.lines.len() {
                        self.pos += 1;
                    }
                    (term, definition)
                }
                Flavor::Org => {
                    self.pos += 1;
                    let content = line.trim_start_matches("- ");
                    let (term, definition) = content.split_once(" :: ").ok_or_else(|| {
                        ParseError::InvalidInput("invalid Org description item".to_string())
                    })?;
                    (term.to_string(), definition.to_string())
                }
                Flavor::Textile => {
                    self.pos += 1;
                    let content = line
                        .trim_start_matches("- ")
                        .trim_end_matches(" =-")
                        .trim_end_matches("=-");
                    let (term, definition) = content.split_once(" := ").ok_or_else(|| {
                        ParseError::InvalidInput("invalid Textile definition item".to_string())
                    })?;
                    (term.to_string(), definition.to_string())
                }
            };
            items.push(DescriptionItem {
                term: parse_inlines(&term, self.flavor),
                definitions: vec![vec![Block::Paragraph {
                    content: parse_inlines(&definition, self.flavor),
                }]],
            });
        }
        Ok(Block::DescriptionList { items })
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut lines = Vec::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].clone();
            if line.trim().is_empty() || (!lines.is_empty() && self.starts_block()) {
                break;
            }
            let content = if self.flavor == Flavor::Textile {
                line.strip_prefix("p. ").unwrap_or(&line).to_string()
            } else {
                line
            };
            lines.push(content);
            self.pos += 1;
        }
        Block::Paragraph {
            content: parse_inlines(&lines.join("\n"), self.flavor),
        }
    }

    fn starts_block(&self) -> bool {
        self.parse_heading_preview()
            || self.is_horizontal_rule()
            || self.is_code_start()
            || self.is_quote_start()
            || self.is_table_start()
            || self.indented_list_marker().is_some()
            || self.textile_list_marker().is_some()
            || self.is_description_start()
    }

    fn parse_heading_preview(&self) -> bool {
        let line = self.lines[self.pos].trim();
        match self.flavor {
            Flavor::Djot => line.starts_with("# "),
            Flavor::Org => {
                let count = line.chars().take_while(|c| *c == '*').count();
                count > 0 && count <= 6 && line[count..].starts_with(' ')
            }
            Flavor::Textile => {
                line.starts_with('h')
                    && line
                        .get(1..)
                        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
                    && line.contains(". ")
            }
        }
    }
}

struct LightweightEmitter {
    output: String,
    flavor: Flavor,
}

impl LightweightEmitter {
    fn emit_blocks(&mut self, blocks: &[Block]) -> Result<(), EmitError> {
        for block in blocks {
            if !self.output.is_empty() && !self.output.ends_with("\n\n") {
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push('\n');
            }
            match block {
                Block::Heading { level, content } => {
                    match self.flavor {
                        Flavor::Djot => self.output.push_str(&"#".repeat(*level as usize)),
                        Flavor::Org => self.output.push_str(&"*".repeat(*level as usize)),
                        Flavor::Textile => {
                            self.output.push('h');
                            self.output.push_str(&level.to_string());
                            self.output.push('.');
                        }
                    }
                    self.output.push(' ');
                    self.emit_inlines(content);
                    self.output.push('\n');
                }
                Block::Paragraph { content } => {
                    if !content.is_empty() {
                        self.emit_inlines(content);
                        self.output.push('\n');
                    }
                }
                Block::CodeBlock { language, content } => self.emit_code(language, content),
                Block::BlockQuote { children } => self.emit_quote(children)?,
                Block::UnorderedList { items } => self.emit_list(items, false, 1, 0)?,
                Block::OrderedList { start, items } => self.emit_list(items, true, *start, 0)?,
                Block::DescriptionList { items } => self.emit_descriptions(items)?,
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => self.emit_table(headers, alignments, rows),
                Block::HorizontalRule => match self.flavor {
                    Flavor::Djot => self.output.push_str("----\n"),
                    Flavor::Org => self.output.push_str("-----\n"),
                    Flavor::Textile => self.output.push_str("<hr />\n"),
                },
                Block::RawBlock { format, content } => match self.flavor {
                    Flavor::Djot => {
                        self.output.push_str("``` =");
                        self.output.push_str(format.as_deref().unwrap_or("text"));
                        self.output.push('\n');
                        self.output.push_str(content);
                        self.output.push_str("\n```\n");
                    }
                    Flavor::Org => {
                        self.output.push_str("#+begin_export ");
                        self.output.push_str(format.as_deref().unwrap_or("text"));
                        self.output.push('\n');
                        self.output.push_str(content);
                        self.output.push_str("\n#+end_export\n");
                    }
                    Flavor::Textile => {
                        self.output.push_str("pre..\n");
                        self.output.push_str(content);
                        self.output.push_str("\n\np.\n");
                    }
                },
            }
        }
        Ok(())
    }

    fn emit_code(&mut self, language: &Option<String>, content: &str) {
        match self.flavor {
            Flavor::Djot => {
                self.output.push_str("```");
                if let Some(language) = language {
                    self.output.push(' ');
                    self.output.push_str(language);
                }
                self.output.push('\n');
                self.output.push_str(content);
                self.output.push_str("\n```\n");
            }
            Flavor::Org => {
                self.output.push_str("#+begin_src");
                if let Some(language) = language {
                    self.output.push(' ');
                    self.output.push_str(language);
                }
                self.output.push('\n');
                self.output.push_str(content);
                self.output.push_str("\n#+end_src\n");
            }
            Flavor::Textile => {
                if let Some(language) = language {
                    self.output.push_str("bc(");
                    self.output.push_str(language);
                    self.output.push_str(")..\n");
                } else {
                    self.output.push_str("bc..\n");
                }
                self.output.push_str(content);
                self.output.push_str("\n\np.\n");
            }
        }
    }

    fn emit_quote(&mut self, children: &[Block]) -> Result<(), EmitError> {
        let inner = emit_document(
            &Document {
                children: children.to_vec(),
            },
            self.flavor,
        )?;
        match self.flavor {
            Flavor::Djot => {
                for line in inner.trim_end().lines() {
                    self.output.push_str("> ");
                    self.output.push_str(line);
                    self.output.push('\n');
                }
            }
            Flavor::Org => {
                self.output.push_str("#+begin_quote\n");
                self.output.push_str(inner.trim_end());
                self.output.push_str("\n#+end_quote\n");
            }
            Flavor::Textile => {
                self.output.push_str("bq..\n");
                self.output.push_str(inner.trim_end());
                self.output.push_str("\n\np.\n");
            }
        }
        Ok(())
    }

    fn emit_list(
        &mut self,
        items: &[ListItem],
        ordered: bool,
        start: u32,
        depth: usize,
    ) -> Result<(), EmitError> {
        for (index, item) in items.iter().enumerate() {
            match self.flavor {
                Flavor::Textile => {
                    self.output
                        .push_str(&if ordered { "#" } else { "*" }.repeat(depth + 1));
                    if ordered && index == 0 && start != 1 {
                        self.output.push_str(&start.to_string());
                    }
                    self.output.push(' ');
                }
                _ => {
                    self.output.push_str(&"  ".repeat(depth));
                    if ordered {
                        self.output.push_str(&format!("{}. ", start + index as u32));
                    } else {
                        self.output.push_str("- ");
                    }
                }
            }
            if let Some(Block::Paragraph { content }) = item.content.first() {
                self.emit_inlines(content);
            }
            self.output.push('\n');
            for child in item.content.iter().skip(1) {
                match child {
                    Block::UnorderedList { items } => self.emit_list(items, false, 1, depth + 1)?,
                    Block::OrderedList { start, items } => {
                        self.emit_list(items, true, *start, depth + 1)?
                    }
                    other => self.emit_blocks(std::slice::from_ref(other))?,
                }
            }
        }
        Ok(())
    }

    fn emit_descriptions(&mut self, items: &[DescriptionItem]) -> Result<(), EmitError> {
        for item in items {
            match self.flavor {
                Flavor::Djot => {
                    self.output.push_str(": ");
                    self.emit_inlines(&item.term);
                    self.output.push_str("\n\n  ");
                }
                Flavor::Org => {
                    self.output.push_str("- ");
                    self.emit_inlines(&item.term);
                    self.output.push_str(" :: ");
                }
                Flavor::Textile => {
                    self.output.push_str("- ");
                    self.emit_inlines(&item.term);
                    self.output.push_str(" := ");
                }
            }
            if let Some(definition) = item.definitions.first()
                && let Some(Block::Paragraph { content }) = definition.first()
            {
                self.emit_inlines(content);
            }
            if self.flavor == Flavor::Textile {
                self.output.push_str(" =-");
            }
            self.output.push('\n');
        }
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) {
        self.output.push('|');
        for (index, cell) in headers.iter().enumerate() {
            self.output.push(' ');
            if self.flavor == Flavor::Textile {
                self.output.push_str("_. ");
                match alignments.get(index).unwrap_or(&ColumnAlignment::Default) {
                    ColumnAlignment::Left => self.output.push_str("<. "),
                    ColumnAlignment::Center => self.output.push_str("=. "),
                    ColumnAlignment::Right => self.output.push_str(">. "),
                    ColumnAlignment::Default => {}
                }
            }
            self.emit_table_cell(cell);
            self.output.push_str(" |");
        }
        self.output.push('\n');
        if self.flavor != Flavor::Textile {
            self.output.push('|');
            for (index, _) in headers.iter().enumerate() {
                let marker = match alignments.get(index).unwrap_or(&ColumnAlignment::Default) {
                    ColumnAlignment::Left => ":---",
                    ColumnAlignment::Center => ":---:",
                    ColumnAlignment::Right => "---:",
                    ColumnAlignment::Default => "---",
                };
                self.output.push(' ');
                self.output.push_str(marker);
                self.output.push_str(" |");
            }
            self.output.push('\n');
        }
        for row in rows {
            self.output.push('|');
            for cell in row {
                self.output.push(' ');
                self.emit_table_cell(cell);
                self.output.push_str(" |");
            }
            self.output.push('\n');
        }
    }

    fn emit_table_cell(&mut self, cell: &TableCell) {
        if self.flavor == Flavor::Textile {
            if cell.colspan > 1 {
                self.output.push('\\');
                self.output.push_str(&cell.colspan.to_string());
                self.output.push_str(". ");
            }
            if cell.rowspan > 1 {
                self.output.push('/');
                self.output.push_str(&cell.rowspan.to_string());
                self.output.push_str(". ");
            }
        }
        self.emit_inlines(&cell.content);
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            match inline {
                Inline::Text(text) => self.output.push_str(&escape_text(text, self.flavor)),
                Inline::Bold(content) => self.emit_wrapped("*", content),
                Inline::Italic(content) => {
                    self.emit_wrapped(if self.flavor == Flavor::Org { "/" } else { "_" }, content)
                }
                Inline::BoldItalic(content) => {
                    self.output.push('*');
                    let italic = if self.flavor == Flavor::Org { "/" } else { "_" };
                    self.output.push_str(italic);
                    self.emit_inlines(content);
                    self.output.push_str(italic);
                    self.output.push('*');
                }
                Inline::Strikethrough(content) => match self.flavor {
                    Flavor::Djot => {
                        self.output.push_str("{-");
                        self.emit_inlines(content);
                        self.output.push_str("-}");
                    }
                    Flavor::Org => self.emit_wrapped("+", content),
                    Flavor::Textile => self.emit_wrapped("-", content),
                },
                Inline::Superscript(content) => match self.flavor {
                    Flavor::Org => {
                        self.output.push_str("^{");
                        self.emit_inlines(content);
                        self.output.push('}');
                    }
                    _ => self.emit_wrapped("^", content),
                },
                Inline::Subscript(content) => match self.flavor {
                    Flavor::Org => {
                        self.output.push_str("_{");
                        self.emit_inlines(content);
                        self.output.push('}');
                    }
                    _ => self.emit_wrapped("~", content),
                },
                Inline::InlineCode(code) => {
                    let delimiter = match self.flavor {
                        Flavor::Djot => "`",
                        Flavor::Org => "~",
                        Flavor::Textile => "@",
                    };
                    self.output.push_str(delimiter);
                    self.output.push_str(code);
                    self.output.push_str(delimiter);
                }
                Inline::Link {
                    url,
                    text,
                    title: _,
                } => match self.flavor {
                    Flavor::Djot => {
                        self.output.push('[');
                        self.emit_inlines(text);
                        self.output.push_str("](");
                        self.output.push_str(url);
                        self.output.push(')');
                    }
                    Flavor::Org => {
                        self.output.push_str("[[");
                        self.output.push_str(url);
                        self.output.push_str("][");
                        self.emit_inlines(text);
                        self.output.push_str("]]");
                    }
                    Flavor::Textile => {
                        self.output.push('"');
                        self.emit_inlines(text);
                        self.output.push_str("\":");
                        self.output.push_str(url);
                    }
                },
                Inline::Image {
                    url,
                    alt,
                    title,
                    link,
                } => {
                    if let Some(link) = link {
                        self.emit_linked_image(url, alt, title, link);
                    } else {
                        self.emit_image(url, alt, title);
                    }
                }
                Inline::HardLineBreak => match self.flavor {
                    Flavor::Djot => self.output.push_str("\\\n"),
                    Flavor::Org => self.output.push_str("\\\\\n"),
                    Flavor::Textile => self.output.push_str("<br />\n"),
                },
                Inline::SoftLineBreak => self.output.push('\n'),
                Inline::RawInline { format, content } => match self.flavor {
                    Flavor::Djot => {
                        self.output.push('`');
                        self.output.push_str(content);
                        self.output.push_str("`{=");
                        self.output.push_str(format.as_deref().unwrap_or("text"));
                        self.output.push('}');
                    }
                    _ => self.output.push_str(content),
                },
            }
        }
    }

    fn emit_wrapped(&mut self, delimiter: &str, content: &[Inline]) {
        self.output.push_str(delimiter);
        self.emit_inlines(content);
        self.output.push_str(delimiter);
    }

    fn emit_image(&mut self, url: &str, alt: &[Inline], title: &Option<String>) {
        match self.flavor {
            Flavor::Djot => {
                self.output.push_str("![");
                self.emit_inlines(alt);
                self.output.push_str("](");
                self.output.push_str(url);
                self.output.push(')');
            }
            Flavor::Org => {
                self.output.push_str("[[");
                self.output.push_str(url);
                if !alt.is_empty() {
                    self.output.push_str("][");
                    self.emit_inlines(alt);
                }
                self.output.push_str("]]");
            }
            Flavor::Textile => {
                self.output.push('!');
                self.output.push_str(url);
                if let Some(title) = title {
                    self.output.push('(');
                    self.output.push_str(title);
                    self.output.push(')');
                }
                self.output.push('!');
            }
        }
    }

    fn emit_linked_image(&mut self, url: &str, alt: &[Inline], title: &Option<String>, link: &str) {
        match self.flavor {
            Flavor::Djot => {
                self.output.push('[');
                self.emit_image(url, alt, title);
                self.output.push_str("](");
                self.output.push_str(link);
                self.output.push(')');
            }
            Flavor::Org => {
                self.output.push_str("[[");
                self.output.push_str(link);
                self.output.push_str("][");
                self.emit_image(url, alt, title);
                self.output.push_str("]]");
            }
            Flavor::Textile => {
                self.emit_image(url, alt, title);
                self.output.push(':');
                self.output.push_str(link);
            }
        }
    }
}

fn parse_inlines(input: &str, flavor: Flavor) -> Vec<Inline> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut pos = 0;
    while pos < input.len() {
        let rest = &input[pos..];
        if rest.starts_with('\n') {
            flush_text(&mut result, &mut text);
            result.push(Inline::SoftLineBreak);
            pos += 1;
            continue;
        }
        if let Some((inline, consumed)) = parse_image_or_link(rest, flavor) {
            flush_text(&mut result, &mut text);
            result.push(inline);
            pos += consumed;
            continue;
        }
        if let Some((inline, consumed)) = parse_code_inline(rest, flavor) {
            flush_text(&mut result, &mut text);
            result.push(inline);
            pos += consumed;
            continue;
        }
        if let Some((inline, consumed)) = parse_styled_inline(rest, flavor) {
            flush_text(&mut result, &mut text);
            result.push(inline);
            pos += consumed;
            continue;
        }
        if rest.starts_with("\\\n") || rest.starts_with("\\\\\n") {
            flush_text(&mut result, &mut text);
            result.push(Inline::HardLineBreak);
            pos += if rest.starts_with("\\\\") { 3 } else { 2 };
            continue;
        }
        if rest.starts_with("<br>") || rest.starts_with("<br />") {
            flush_text(&mut result, &mut text);
            result.push(Inline::HardLineBreak);
            pos += if rest.starts_with("<br />") { 6 } else { 4 };
            continue;
        }
        if let Some(stripped) = rest.strip_prefix('\\')
            && let Some(character) = stripped.chars().next()
        {
            text.push(character);
            pos += 1 + character.len_utf8();
            continue;
        }
        let character = rest.chars().next().unwrap();
        text.push(character);
        pos += character.len_utf8();
    }
    flush_text(&mut result, &mut text);
    result
}

fn parse_image_or_link(input: &str, flavor: Flavor) -> Option<(Inline, usize)> {
    match flavor {
        Flavor::Djot => {
            let image = input.starts_with("![");
            let start = if image {
                2
            } else if input.starts_with('[') {
                1
            } else {
                return None;
            };
            let close = input[start..].find(']')? + start;
            if !input[close + 1..].starts_with('(') {
                return None;
            }
            let end = input[close + 2..].find(')')? + close + 2;
            let label = &input[start..close];
            let url = &input[close + 2..end];
            let inline = if image {
                Inline::Image {
                    url: url.to_string(),
                    alt: parse_inlines(label, flavor),
                    title: None,
                    link: None,
                }
            } else {
                Inline::Link {
                    url: url.to_string(),
                    text: parse_inlines(label, flavor),
                    title: None,
                }
            };
            Some((inline, end + 1))
        }
        Flavor::Org => {
            if !input.starts_with("[[") {
                return None;
            }
            let end = input.find("]]")?;
            let body = &input[2..end];
            let (url, label) = body.split_once("][")?;
            let is_image = is_image_url(url);
            let inline = if is_image {
                Inline::Image {
                    url: url.to_string(),
                    alt: parse_inlines(label, flavor),
                    title: None,
                    link: None,
                }
            } else {
                Inline::Link {
                    url: url.to_string(),
                    text: parse_inlines(label, flavor),
                    title: None,
                }
            };
            Some((inline, end + 2))
        }
        Flavor::Textile => {
            if let Some(body) = input.strip_prefix('!') {
                let end = body.find('!')?;
                let spec = &body[..end];
                let (url, title) = if spec.ends_with(')') {
                    let open = spec.rfind('(')?;
                    (
                        &spec[..open],
                        Some(spec[open + 1..spec.len() - 1].to_string()),
                    )
                } else {
                    (spec, None)
                };
                let consumed = end + 2;
                return Some((
                    Inline::Image {
                        url: url.to_string(),
                        alt: Vec::new(),
                        title,
                        link: None,
                    },
                    consumed,
                ));
            }
            if !input.starts_with('"') {
                return None;
            }
            let close = input[1..].find("\":")? + 1;
            let url_start = close + 2;
            let raw_url_end = input[url_start..]
                .find(char::is_whitespace)
                .map(|offset| url_start + offset)
                .unwrap_or(input.len());
            let url_end = input[url_start..raw_url_end]
                .trim_end_matches(['.', ',', ';'])
                .len()
                + url_start;
            Some((
                Inline::Link {
                    url: input[url_start..url_end].to_string(),
                    text: parse_inlines(&input[1..close], flavor),
                    title: None,
                },
                url_end,
            ))
        }
    }
}

fn parse_code_inline(input: &str, flavor: Flavor) -> Option<(Inline, usize)> {
    let delimiter = match flavor {
        Flavor::Djot => "`",
        Flavor::Org => {
            if input.starts_with('~') {
                "~"
            } else if input.starts_with('=') {
                "="
            } else {
                return None;
            }
        }
        Flavor::Textile => "@",
    };
    if !input.starts_with(delimiter) {
        return None;
    }
    let end = input[delimiter.len()..].find(delimiter)? + delimiter.len();
    if end == delimiter.len() {
        return None;
    }
    Some((
        Inline::InlineCode(input[delimiter.len()..end].to_string()),
        end + delimiter.len(),
    ))
}

fn parse_styled_inline(input: &str, flavor: Flavor) -> Option<(Inline, usize)> {
    type InlineConstructor = fn(Vec<Inline>) -> Inline;
    let candidates: &[(&str, InlineConstructor)] = match flavor {
        Flavor::Djot => &[
            ("{-", Inline::Strikethrough),
            ("*", Inline::Bold),
            ("_", Inline::Italic),
            ("^", Inline::Superscript),
            ("~", Inline::Subscript),
        ],
        Flavor::Org => &[
            ("*", Inline::Bold),
            ("/", Inline::Italic),
            ("+", Inline::Strikethrough),
        ],
        Flavor::Textile => &[
            ("*", Inline::Bold),
            ("_", Inline::Italic),
            ("-", Inline::Strikethrough),
            ("^", Inline::Superscript),
            ("~", Inline::Subscript),
        ],
    };
    for (open, constructor) in candidates {
        if !input.starts_with(open) {
            continue;
        }
        let close = if *open == "{-" { "-}" } else { *open };
        let end = input[open.len()..].find(close)? + open.len();
        if end == open.len() || input[open.len()..end].trim().is_empty() {
            continue;
        }
        return Some((
            constructor(parse_inlines(&input[open.len()..end], flavor)),
            end + close.len(),
        ));
    }
    if flavor == Flavor::Org && (input.starts_with("^{") || input.starts_with("_{")) {
        let end = input.find('}')?;
        let content = parse_inlines(&input[2..end], flavor);
        let inline = if input.starts_with("^{") {
            Inline::Superscript(content)
        } else {
            Inline::Subscript(content)
        };
        return Some((inline, end + 1));
    }
    None
}

fn parse_indented_marker(line: &str, flavor: Flavor) -> Option<(usize, bool, u32, usize)> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    let rest = &line[indent..];
    if let Some(marker) = ["- ", "+ "]
        .into_iter()
        .find(|marker| rest.starts_with(marker))
    {
        return Some((indent, false, 1, marker.len()));
    }
    if flavor == Flavor::Djot && rest.starts_with("* ") {
        return Some((indent, false, 1, 2));
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let suffix = rest.as_bytes().get(digits).copied()?;
        if (suffix == b'.' || suffix == b')')
            && rest.as_bytes().get(digits + 1).copied() == Some(b' ')
        {
            return Some((indent, true, rest[..digits].parse().ok()?, digits + 2));
        }
    }
    None
}

fn parse_textile_marker(line: &str) -> Option<(char, usize, u32, &str)> {
    let line = line.trim_start();
    let kind = line.chars().next()?;
    if kind != '*' && kind != '#' {
        return None;
    }
    let level = line.chars().take_while(|c| *c == kind).count();
    let digits = line[level..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .count();
    let start = if digits == 0 {
        1
    } else {
        line[level..level + digits].parse().ok()?
    };
    line[level + digits..]
        .strip_prefix(' ')
        .map(|content| (kind, level, start, content))
}

fn split_pipe_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_table_separator(cell: &str, flavor: Flavor) -> bool {
    if flavor == Flavor::Org {
        return cell
            .chars()
            .filter(|c| *c != '+')
            .all(|c| c == '-' || c == ':');
    }
    let trimmed = cell.trim_matches(':').trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-')
}

fn parse_alignment(cell: &str) -> ColumnAlignment {
    match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => ColumnAlignment::Center,
        (true, false) => ColumnAlignment::Left,
        (false, true) => ColumnAlignment::Right,
        (false, false) => ColumnAlignment::Default,
    }
}

fn parse_table_cell(cell: &str, flavor: Flavor) -> TableCell {
    let mut content = cell.trim();
    let mut colspan = 1;
    let mut rowspan = 1;
    if flavor == Flavor::Textile {
        if let Some(rest) = content.strip_prefix("_. ") {
            content = rest;
        }
        if let Some(rest) = content
            .strip_prefix("<. ")
            .or_else(|| content.strip_prefix("=. "))
            .or_else(|| content.strip_prefix(">. "))
        {
            content = rest;
        }
        loop {
            if let Some(rest) = content.strip_prefix('\\') {
                let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
                if let Ok(value) = rest[..digits].parse() {
                    colspan = value;
                    content = rest[digits..].trim_start_matches('.').trim();
                    continue;
                }
            }
            if let Some(rest) = content.strip_prefix('/') {
                let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
                if let Ok(value) = rest[..digits].parse() {
                    rowspan = value;
                    content = rest[digits..].trim_start_matches('.').trim();
                    continue;
                }
            }
            break;
        }
    }
    TableCell::with_span(parse_inlines(content, flavor), colspan, rowspan)
}

fn parse_textile_alignment(cell: &str) -> ColumnAlignment {
    let content = cell.strip_prefix("_. ").unwrap_or(cell);
    if content.starts_with("<. ") {
        ColumnAlignment::Left
    } else if content.starts_with("=. ") {
        ColumnAlignment::Center
    } else if content.starts_with(">. ") {
        ColumnAlignment::Right
    } else {
        ColumnAlignment::Default
    }
}

fn is_image_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn escape_text(text: &str, flavor: Flavor) -> String {
    let specials = match flavor {
        Flavor::Djot => "\\*_`[]",
        Flavor::Org => "\\*/+~=[]",
        Flavor::Textile => "\\*_@\"!",
    };
    let mut output = String::new();
    for character in text.chars() {
        if specials.contains(character) {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

fn flush_text(result: &mut Vec<Inline>, text: &mut String) {
    if !text.is_empty() {
        result.push(Inline::Text(std::mem::take(text)));
    }
}
