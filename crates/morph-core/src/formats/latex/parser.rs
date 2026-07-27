use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;

pub struct LatexParser;

impl Parser for LatexParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let mut state = LatexParserState::new(input);
        let children = state.parse_blocks()?;
        Ok(Document { children })
    }
}

struct LatexParserState {
    lines: Vec<String>,
    pos: usize,
}

impl LatexParserState {
    fn new(input: &str) -> Self {
        Self {
            lines: input.lines().map(str::to_string).collect(),
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
            let line = strip_latex_comment(self.current_line());
            let trimmed = line.trim();

            if trimmed.is_empty() || is_ignored_document_command(trimmed) {
                self.advance();
                continue;
            }

            if let Some(block) = self.try_parse_heading(trimmed) {
                blocks.push(block);
                self.advance();
                continue;
            }

            if let Some(environment) = begin_environment(trimmed) {
                match environment.as_str() {
                    "lstlisting" | "verbatim" | "Verbatim" | "minted" => {
                        blocks.push(self.parse_code_block(&environment)?);
                    }
                    "quote" | "quotation" => {
                        blocks.push(self.parse_quote(&environment)?);
                    }
                    "itemize" | "enumerate" | "description" => {
                        blocks.push(self.parse_list(&environment)?);
                    }
                    "tabular" | "tabularx" | "longtable" => {
                        blocks.push(self.parse_table(&environment)?);
                    }
                    "document" | "table" | "table*" | "center" | "figure" | "figure*" => {
                        let inner = self.collect_environment_body(&environment);
                        let mut inner_state = LatexParserState::new(&inner);
                        blocks.extend(inner_state.parse_blocks()?);
                    }
                    _ => blocks.push(self.parse_raw_environment(&environment)),
                }
                continue;
            }

            if is_horizontal_rule(trimmed) {
                blocks.push(Block::HorizontalRule);
                self.advance();
                continue;
            }

            blocks.push(self.parse_paragraph()?);
        }

        Ok(blocks)
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        const HEADINGS: &[(&str, u8)] = &[
            ("part", 1),
            ("chapter", 1),
            ("section", 1),
            ("subsection", 2),
            ("subsubsection", 3),
            ("paragraph", 4),
            ("subparagraph", 5),
        ];

        let (command, mut offset) = parse_command(line, 0)?;
        let command = command.trim_end_matches('*');
        let level = HEADINGS
            .iter()
            .find_map(|(name, level)| (*name == command).then_some(*level))?;

        offset = skip_whitespace(line, offset);
        if line[offset..].starts_with('[') {
            let (_, end) = extract_delimited(line, offset, '[', ']')?;
            offset = skip_whitespace(line, end);
        }
        let (content, _) = extract_group(line, offset)?;
        Some(Block::Heading {
            level,
            content: parse_latex_inlines(&content),
        })
    }

    fn parse_code_block(&mut self, environment: &str) -> Result<Block, ParseError> {
        let opening = self.current_line().trim().to_string();
        let language = if environment == "minted" {
            groups_after_environment(&opening)
                .last()
                .filter(|value| !value.is_empty())
                .cloned()
        } else {
            extract_option(&opening).and_then(|options| {
                options.split(',').find_map(|option| {
                    let (key, value) = option.split_once('=')?;
                    key.trim()
                        .eq_ignore_ascii_case("language")
                        .then(|| value.trim().to_string())
                })
            })
        };
        self.advance();

        let mut content = Vec::new();
        while !self.at_end() {
            let line = self.current_line().to_string();
            if end_environment(line.trim()).as_deref() == Some(environment) {
                self.advance();
                break;
            }
            content.push(line);
            self.advance();
        }

        Ok(Block::CodeBlock {
            language,
            content: content.join("\n"),
        })
    }

    fn parse_quote(&mut self, environment: &str) -> Result<Block, ParseError> {
        let inner = self.collect_environment_body(environment);
        let mut state = LatexParserState::new(&inner);
        Ok(Block::BlockQuote {
            children: state.parse_blocks()?,
        })
    }

    fn parse_list(&mut self, environment: &str) -> Result<Block, ParseError> {
        self.advance();
        let mut raw_items: Vec<String> = Vec::new();
        let mut current: Option<Vec<String>> = None;
        let mut nested_depth = 0usize;

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = strip_latex_comment(&line).trim().to_string();

            if nested_depth == 0 && end_environment(&trimmed).as_deref() == Some(environment) {
                if let Some(lines) = current.take() {
                    raw_items.push(lines.join("\n"));
                }
                self.advance();
                break;
            }

            if nested_depth == 0
                && let Some(item_content) = strip_item_command(&trimmed)
            {
                if let Some(lines) = current.replace(vec![item_content.to_string()]) {
                    raw_items.push(lines.join("\n"));
                }
                self.advance();
                continue;
            }

            if let Some(lines) = current.as_mut() {
                lines.push(line.clone());
            }

            if begin_environment(&trimmed).is_some() {
                nested_depth += 1;
            } else if end_environment(&trimmed).is_some() && nested_depth > 0 {
                nested_depth -= 1;
            }
            self.advance();
        }

        if environment == "description" {
            let mut items = Vec::new();
            for raw in raw_items {
                let (term, definition) = split_description_item(&raw);
                let mut state = LatexParserState::new(&definition);
                let blocks = state.parse_blocks()?;
                items.push(DescriptionItem {
                    term: parse_latex_inlines(&term),
                    definitions: vec![blocks],
                });
            }
            return Ok(Block::DescriptionList { items });
        }

        let mut items = Vec::new();
        for raw in raw_items {
            let mut state = LatexParserState::new(raw.trim());
            let content = state.parse_blocks()?;
            items.push(ListItem { content });
        }

        if environment == "itemize" {
            Ok(Block::UnorderedList { items })
        } else {
            Ok(Block::OrderedList { start: 1, items })
        }
    }

    fn parse_table(&mut self, environment: &str) -> Result<Block, ParseError> {
        let opening = self.current_line().trim().to_string();
        let alignments = parse_table_alignments(&opening, environment);
        let body = self.collect_environment_body(environment);
        let mut parsed_rows = Vec::new();

        for raw_row in split_latex_rows(&body) {
            let cleaned = strip_table_rules(&raw_row);
            if cleaned.trim().is_empty() {
                continue;
            }
            let cells = split_top_level(&cleaned, '&')
                .into_iter()
                .map(|cell| parse_table_cell(cell.trim()))
                .collect::<Vec<_>>();
            if !cells.is_empty() {
                parsed_rows.push(cells);
            }
        }

        let headers = if parsed_rows.is_empty() {
            Vec::new()
        } else {
            parsed_rows.remove(0)
        };

        Ok(Block::Table {
            headers,
            alignments,
            rows: parsed_rows,
        })
    }

    fn parse_raw_environment(&mut self, environment: &str) -> Block {
        let opening = self.current_line().to_string();
        let body = self.collect_environment_body(environment);
        let mut content = opening;
        if !body.is_empty() {
            content.push('\n');
            content.push_str(&body);
        }
        content.push_str("\n\\end{");
        content.push_str(environment);
        content.push('}');
        Block::RawBlock {
            format: Some("latex".to_string()),
            content,
        }
    }

    fn collect_environment_body(&mut self, environment: &str) -> String {
        self.advance();
        let mut depth = 0usize;
        let mut lines = Vec::new();

        while !self.at_end() {
            let line = self.current_line().to_string();
            let trimmed = strip_latex_comment(&line).trim().to_string();
            if begin_environment(&trimmed).as_deref() == Some(environment) {
                depth += 1;
                lines.push(line);
                self.advance();
                continue;
            }
            if end_environment(&trimmed).as_deref() == Some(environment) {
                if depth == 0 {
                    self.advance();
                    break;
                }
                depth -= 1;
                lines.push(line);
                self.advance();
                continue;
            }
            lines.push(line);
            self.advance();
        }

        lines.join("\n")
    }

    fn parse_paragraph(&mut self) -> Result<Block, ParseError> {
        let mut lines = Vec::new();

        while !self.at_end() {
            let clean = strip_latex_comment(self.current_line());
            let trimmed = clean.trim();
            if trimmed.is_empty() {
                break;
            }
            if !lines.is_empty() && is_block_start(trimmed) {
                break;
            }
            if is_ignored_document_command(trimmed) {
                if lines.is_empty() {
                    self.advance();
                    continue;
                }
                break;
            }
            lines.push(trimmed.to_string());
            self.advance();
        }

        let mut joined = String::new();
        for (index, line) in lines.iter().enumerate() {
            if index > 0 && !joined.ends_with("\\\\") {
                joined.push(' ');
            }
            joined.push_str(line);
        }

        Ok(Block::Paragraph {
            content: parse_latex_inlines(&joined),
        })
    }
}

fn is_block_start(line: &str) -> bool {
    begin_environment(line).is_some()
        || is_horizontal_rule(line)
        || parse_command(line, 0)
            .map(|(command, _)| {
                matches!(
                    command.trim_end_matches('*'),
                    "part"
                        | "chapter"
                        | "section"
                        | "subsection"
                        | "subsubsection"
                        | "paragraph"
                        | "subparagraph"
                )
            })
            .unwrap_or(false)
}

fn is_ignored_document_command(line: &str) -> bool {
    let Some((command, _)) = parse_command(line, 0) else {
        return false;
    };
    matches!(
        command.as_str(),
        "documentclass"
            | "usepackage"
            | "title"
            | "author"
            | "date"
            | "maketitle"
            | "tableofcontents"
            | "centering"
            | "raggedright"
            | "newpage"
            | "clearpage"
            | "small"
            | "footnotesize"
            | "large"
            | "Large"
            | "LARGE"
    ) || line == "\\begin{document}"
        || line == "\\end{document}"
}

fn is_horizontal_rule(line: &str) -> bool {
    line == "\\hrule"
        || line == "\\hrulefill"
        || line == "\\noindent\\hrulefill"
        || line.starts_with("\\rule{\\linewidth}")
        || line.starts_with("\\rule{\\textwidth}")
}

fn begin_environment(line: &str) -> Option<String> {
    environment_command(line, "begin")
}

fn end_environment(line: &str) -> Option<String> {
    environment_command(line, "end")
}

fn environment_command(line: &str, expected: &str) -> Option<String> {
    let (command, offset) = parse_command(line, 0)?;
    if command != expected {
        return None;
    }
    let offset = skip_whitespace(line, offset);
    let (environment, _) = extract_group(line, offset)?;
    Some(environment)
}

fn groups_after_environment(line: &str) -> Vec<String> {
    let Some((_, offset)) = parse_command(line, 0) else {
        return Vec::new();
    };
    let offset = skip_whitespace(line, offset);
    let Some((_, mut offset)) = extract_group(line, offset) else {
        return Vec::new();
    };
    let mut groups = Vec::new();
    loop {
        offset = skip_whitespace(line, offset);
        if !line[offset..].starts_with('{') {
            break;
        }
        let Some((group, end)) = extract_group(line, offset) else {
            break;
        };
        groups.push(group);
        offset = end;
    }
    groups
}

fn extract_option(line: &str) -> Option<String> {
    let start = line.find('[')?;
    extract_delimited(line, start, '[', ']').map(|(value, _)| value)
}

fn strip_item_command(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("\\item")?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with('[') {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn split_description_item(raw: &str) -> (String, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with('[') {
        return (String::new(), trimmed.to_string());
    }
    if let Some((term, end)) = extract_delimited(trimmed, 0, '[', ']') {
        return (term, trimmed[end..].trim_start().to_string());
    }
    (String::new(), trimmed.to_string())
}

fn parse_table_alignments(opening: &str, environment: &str) -> Vec<ColumnAlignment> {
    let groups = groups_after_environment(opening);
    let spec = if environment == "tabularx" {
        groups.get(1)
    } else {
        groups.first()
    };
    let Some(spec) = spec else {
        return Vec::new();
    };

    let mut alignments = Vec::new();
    let mut chars = spec.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            // A plain `l` is also the emitter's representation for the AST's
            // unspecified/default alignment, so keep it as Default for stable
            // round trips. Width-bearing columns are explicitly left aligned.
            'l' => alignments.push(ColumnAlignment::Default),
            'p' | 'm' | 'b' | 'X' => alignments.push(ColumnAlignment::Left),
            'c' => alignments.push(ColumnAlignment::Center),
            'r' => alignments.push(ColumnAlignment::Right),
            '@' | '!' | '>' | '<' => {
                if chars.peek() == Some(&'{') {
                    let mut depth = 0usize;
                    for next in chars.by_ref() {
                        match next {
                            '{' => depth += 1,
                            '}' if depth == 1 => break,
                            '}' => depth = depth.saturating_sub(1),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    alignments
}

fn split_latex_rows(body: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut index = 0usize;

    while index < body.len() {
        let rest = &body[index..];
        if rest.starts_with("\\\\") && depth == 0 {
            rows.push(current);
            current = String::new();
            index += 2;
            continue;
        }

        let character = rest.chars().next().unwrap();
        match character {
            '{' if !is_escaped(body, index) => depth += 1,
            '}' if !is_escaped(body, index) => depth = depth.saturating_sub(1),
            _ => {}
        }
        current.push(character);
        index += character.len_utf8();
    }
    if !current.trim().is_empty() {
        rows.push(current);
    }
    rows
}

fn strip_table_rules(row: &str) -> String {
    let mut result = row.to_string();
    for command in ["\\hline", "\\toprule", "\\midrule", "\\bottomrule"] {
        result = result.replace(command, "");
    }
    // \cline and \cmidrule carry a range argument.
    for command in ["cline", "cmidrule"] {
        loop {
            let needle = format!("\\{command}");
            let Some(start) = result.find(&needle) else {
                break;
            };
            let group_start = skip_whitespace(&result, start + needle.len());
            let end = extract_group(&result, group_start)
                .map(|(_, end)| end)
                .unwrap_or(start + needle.len());
            result.replace_range(start..end, "");
        }
    }
    result
}

fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for (index, character) in input.char_indices() {
        match character {
            '{' if !is_escaped(input, index) => depth += 1,
            '}' if !is_escaped(input, index) => depth = depth.saturating_sub(1),
            _ if character == delimiter && depth == 0 && !is_escaped(input, index) => {
                parts.push(current);
                current = String::new();
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    parts.push(current);
    parts
}

fn parse_table_cell(input: &str) -> TableCell {
    let mut content = input.trim().to_string();
    let mut colspan = 1u32;
    let mut rowspan = 1u32;

    loop {
        if let Some(arguments) = parse_exact_command_groups(&content, "multicolumn", 3) {
            colspan = arguments[0].trim().parse().unwrap_or(1);
            content = arguments[2].clone();
            continue;
        }
        if let Some(arguments) = parse_exact_command_groups(&content, "multirow", 3) {
            rowspan = arguments[0].trim().parse().unwrap_or(1);
            content = arguments[2].clone();
            continue;
        }
        break;
    }

    TableCell::with_span(parse_latex_inlines(content.trim()), colspan, rowspan)
}

fn parse_exact_command_groups(input: &str, expected: &str, count: usize) -> Option<Vec<String>> {
    let (command, mut offset) = parse_command(input, 0)?;
    if command != expected {
        return None;
    }
    let mut groups = Vec::new();
    for _ in 0..count {
        offset = skip_whitespace(input, offset);
        let (group, end) = extract_group(input, offset)?;
        groups.push(group);
        offset = end;
    }
    (input[offset..].trim().is_empty()).then_some(groups)
}

fn parse_latex_inlines(input: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut index = 0usize;

    while index < input.len() {
        let rest = &input[index..];

        if rest.starts_with("\\\\") {
            flush_text(&mut result, &mut text);
            result.push(Inline::HardLineBreak);
            index += 2;
            continue;
        }

        if let Some(math) = rest.strip_prefix('$')
            && let Some(relative_end) = find_unescaped(math, '$')
        {
            flush_text(&mut result, &mut text);
            let end = index + 1 + relative_end + 1;
            result.push(Inline::RawInline {
                format: Some("latex".to_string()),
                content: input[index..=end].to_string(),
            });
            index = end + 1;
            continue;
        }

        if rest.starts_with('~') {
            text.push(' ');
            index += 1;
            continue;
        }

        if !rest.starts_with('\\') {
            let character = rest.chars().next().unwrap();
            text.push(character);
            index += character.len_utf8();
            continue;
        }

        let Some((command, mut offset)) = parse_command(input, index) else {
            text.push('\\');
            index += 1;
            continue;
        };

        if let Some(literal) = escaped_literal(&command) {
            text.push(literal);
            index = offset;
            continue;
        }

        if matches!(
            command.as_str(),
            "textbackslash" | "textasciicircum" | "textasciitilde"
        ) {
            offset = skip_whitespace(input, offset);
            if let Some((_, end)) = extract_group(input, offset) {
                offset = end;
            }
            text.push(match command.as_str() {
                "textbackslash" => '\\',
                "textasciicircum" => '^',
                _ => '~',
            });
            index = offset;
            continue;
        }

        if command == "verb" {
            let Some(delimiter) = input[offset..].chars().next() else {
                index = offset;
                continue;
            };
            let content_start = offset + delimiter.len_utf8();
            if let Some(relative_end) = input[content_start..].find(delimiter) {
                flush_text(&mut result, &mut text);
                result.push(Inline::InlineCode(
                    input[content_start..content_start + relative_end].to_string(),
                ));
                index = content_start + relative_end + delimiter.len_utf8();
                continue;
            }
        }

        offset = skip_whitespace(input, offset);
        if command == "includegraphics" {
            if input[offset..].starts_with('[')
                && let Some((_, end)) = extract_delimited(input, offset, '[', ']')
            {
                offset = skip_whitespace(input, end);
            }
            if let Some((url, end)) = extract_group(input, offset) {
                flush_text(&mut result, &mut text);
                result.push(Inline::Image {
                    url: unescape_latex(&url),
                    alt: Vec::new(),
                    title: None,
                    link: None,
                });
                index = end;
                continue;
            }
        }

        if command == "href"
            && let Some((url, after_url)) = extract_group(input, offset)
            && let Some((label, end)) = extract_group(input, skip_whitespace(input, after_url))
        {
            flush_text(&mut result, &mut text);
            result.push(Inline::Link {
                url: unescape_latex(&url),
                text: parse_latex_inlines(&label),
                title: None,
            });
            index = end;
            continue;
        }

        if command == "url"
            && let Some((url, end)) = extract_group(input, offset)
        {
            let url = unescape_latex(&url);
            flush_text(&mut result, &mut text);
            result.push(Inline::Link {
                text: vec![Inline::Text(url.clone())],
                url,
                title: None,
            });
            index = end;
            continue;
        }

        let inline_kind = match command.as_str() {
            "textbf" | "bf" => Some("bold"),
            "textit" | "emph" | "em" => Some("italic"),
            "texttt" => Some("code"),
            "sout" | "st" => Some("strike"),
            "textsuperscript" => Some("superscript"),
            "textsubscript" => Some("subscript"),
            _ => None,
        };
        if let Some(kind) = inline_kind
            && let Some((content, end)) = extract_group(input, offset)
        {
            flush_text(&mut result, &mut text);
            let parsed = parse_latex_inlines(&content);
            result.push(match kind {
                "bold" => match parsed.as_slice() {
                    [Inline::Italic(inner)] => Inline::BoldItalic(inner.clone()),
                    _ => Inline::Bold(parsed),
                },
                "italic" => match parsed.as_slice() {
                    [Inline::Bold(inner)] => Inline::BoldItalic(inner.clone()),
                    _ => Inline::Italic(parsed),
                },
                "code" => Inline::InlineCode(unescape_latex(&content)),
                "strike" => Inline::Strikethrough(parsed),
                "superscript" => Inline::Superscript(parsed),
                "subscript" => Inline::Subscript(parsed),
                _ => unreachable!(),
            });
            index = end;
            continue;
        }

        // Preserve commands that have no shared AST equivalent.
        let mut end = offset;
        loop {
            end = skip_whitespace(input, end);
            let delimiter = input[end..].chars().next();
            let parsed = match delimiter {
                Some('{') => extract_group(input, end),
                Some('[') => extract_delimited(input, end, '[', ']'),
                _ => None,
            };
            let Some((_, next)) = parsed else {
                break;
            };
            end = next;
        }
        flush_text(&mut result, &mut text);
        result.push(Inline::RawInline {
            format: Some("latex".to_string()),
            content: input[index..end].to_string(),
        });
        index = end;
    }

    flush_text(&mut result, &mut text);
    merge_adjacent_text(result)
}

fn escaped_literal(command: &str) -> Option<char> {
    match command {
        "#" => Some('#'),
        "$" => Some('$'),
        "%" => Some('%'),
        "&" => Some('&'),
        "_" => Some('_'),
        "{" => Some('{'),
        "}" => Some('}'),
        "~" => Some('~'),
        "^" => Some('^'),
        _ => None,
    }
}

fn unescape_latex(input: &str) -> String {
    let inlines = parse_latex_inlines(input);
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::InlineCode(text) => result.push_str(&text),
            Inline::RawInline { content, .. } => result.push_str(&content),
            _ => {}
        }
    }
    result
}

fn parse_command(input: &str, start: usize) -> Option<(String, usize)> {
    if !input[start..].starts_with('\\') {
        return None;
    }
    let mut offset = start + 1;
    let first = input[offset..].chars().next()?;
    if first.is_alphabetic() || first == '@' {
        let mut command = String::new();
        for character in input[offset..].chars() {
            if character.is_alphabetic() || character == '@' {
                command.push(character);
                offset += character.len_utf8();
            } else {
                break;
            }
        }
        if input[offset..].starts_with('*') {
            command.push('*');
            offset += 1;
        }
        Some((command, offset))
    } else {
        Some((first.to_string(), offset + first.len_utf8()))
    }
}

fn extract_group(input: &str, start: usize) -> Option<(String, usize)> {
    extract_delimited(input, start, '{', '}')
}

fn extract_delimited(
    input: &str,
    start: usize,
    opening: char,
    closing: char,
) -> Option<(String, usize)> {
    if input[start..].chars().next()? != opening {
        return None;
    }
    let mut depth = 0usize;
    let content_start = start + opening.len_utf8();
    for (relative, character) in input[start..].char_indices() {
        let index = start + relative;
        if is_escaped(input, index) {
            continue;
        }
        if character == opening {
            depth += 1;
        } else if character == closing {
            depth -= 1;
            if depth == 0 {
                return Some((
                    input[content_start..index].to_string(),
                    index + closing.len_utf8(),
                ));
            }
        }
    }
    None
}

fn skip_whitespace(input: &str, mut offset: usize) -> usize {
    while offset < input.len() {
        let character = input[offset..].chars().next().unwrap();
        if !character.is_whitespace() {
            break;
        }
        offset += character.len_utf8();
    }
    offset
}

fn strip_latex_comment(line: &str) -> String {
    for (index, character) in line.char_indices() {
        if character == '%' && !is_escaped(line, index) {
            return line[..index].to_string();
        }
    }
    line.to_string()
}

fn is_escaped(input: &str, index: usize) -> bool {
    let preceding = input[..index]
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count();
    preceding % 2 == 1
}

fn find_unescaped(input: &str, needle: char) -> Option<usize> {
    input.char_indices().find_map(|(index, character)| {
        (character == needle && !is_escaped(input, index)).then_some(index)
    })
}

fn flush_text(result: &mut Vec<Inline>, text: &mut String) {
    if !text.is_empty() {
        result.push(Inline::Text(std::mem::take(text)));
    }
}

fn merge_adjacent_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut result: Vec<Inline> = Vec::new();
    for inline in inlines {
        if let Inline::Text(text) = inline {
            if let Some(Inline::Text(previous)) = result.last_mut() {
                previous.push_str(&text);
            } else {
                result.push(Inline::Text(text));
            }
        } else {
            result.push(inline);
        }
    }
    result
}
