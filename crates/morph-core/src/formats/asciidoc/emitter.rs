use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;

pub struct AsciiDocEmitter;

impl Emitter for AsciiDocEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut ctx = EmitContext::new();
        ctx.emit_blocks(&doc.children)?;
        Ok(ctx.finish())
    }
}

struct EmitContext {
    output: String,
    list_depth: usize,
    blockquote_depth: usize,
}

impl EmitContext {
    fn new() -> Self {
        EmitContext {
            output: String::new(),
            list_depth: 0,
            blockquote_depth: 0,
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
            match block {
                Block::Heading { level, content } => {
                    if i > 0 {
                        self.ensure_blank_line();
                    }
                    let marker: String = "=".repeat(*level as usize);
                    self.push(&marker);
                    self.push(" ");
                    self.emit_inlines(content)?;
                    self.push_newline();
                }
                Block::Paragraph { content } => {
                    if content.is_empty() {
                        // Empty paragraph = separator in loose lists
                        continue;
                    }
                    if i > 0 {
                        match &blocks[i - 1] {
                            Block::Paragraph {
                                content: prev_content,
                            } if !prev_content.is_empty() => {
                                self.ensure_blank_line();
                            }
                            Block::Heading { .. }
                            | Block::CodeBlock { .. }
                            | Block::BlockQuote { .. }
                            | Block::HorizontalRule
                            | Block::Table { .. }
                            | Block::DescriptionList { .. } => {
                                self.ensure_blank_line();
                            }
                            Block::UnorderedList { .. } | Block::OrderedList { .. } => {
                                if self.list_depth == 0 {
                                    self.ensure_blank_line();
                                }
                            }
                            _ => {}
                        }
                    }
                    self.emit_inlines(content)?;
                    self.push_newline();
                }
                Block::CodeBlock { language, content } => {
                    if i > 0 {
                        self.ensure_blank_line();
                    }
                    if let Some(lang) = language {
                        self.push(&format!("[source,{lang}]\n"));
                    }
                    self.push("----\n");
                    self.push(content);
                    self.push("\n----\n");
                }
                Block::BlockQuote { children } => {
                    if i > 0 {
                        self.ensure_blank_line();
                    }
                    self.blockquote_depth += 1;
                    let delim_len = self.blockquote_depth * 4;
                    let delim: String = "_".repeat(delim_len);
                    self.push(&delim);
                    self.push_newline();
                    self.push_newline();
                    self.emit_blocks(children)?;
                    self.push_newline();
                    self.push(&delim);
                    self.push_newline();
                    self.blockquote_depth -= 1;
                }
                Block::UnorderedList { items } => {
                    self.emit_unordered_list(items, i > 0 && self.list_depth == 0)?;
                }
                Block::OrderedList { items, .. } => {
                    self.emit_ordered_list(items, i > 0 && self.list_depth == 0)?;
                }
                Block::DescriptionList { items } => {
                    if i > 0 {
                        self.ensure_blank_line();
                    }
                    for item in items {
                        self.emit_inlines(&item.term)?;
                        self.push("::\n");
                        for def_blocks in &item.definitions {
                            for def_block in def_blocks {
                                if let Block::Paragraph { content } = def_block {
                                    self.push("  ");
                                    self.emit_inlines_with_indent(content, "  ")?;
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
                    if i > 0 {
                        match &blocks[i - 1] {
                            Block::Paragraph { .. }
                            | Block::Heading { .. }
                            | Block::CodeBlock { .. }
                            | Block::BlockQuote { .. } => {
                                self.ensure_blank_line();
                            }
                            _ => {}
                        }
                    }
                    self.emit_table(headers, alignments, rows)?;
                }
                Block::HorizontalRule => {
                    if i > 0 && !self.output.ends_with('\n') {
                        self.push_newline();
                    }
                    self.push("'''\n");
                }
                Block::RawBlock { content, .. } => {
                    self.push(content);
                    self.push_newline();
                }
            }
        }
        Ok(())
    }

    fn emit_unordered_list(
        &mut self,
        items: &[ListItem],
        needs_blank_before: bool,
    ) -> Result<(), EmitError> {
        if needs_blank_before && self.list_depth == 0 {
            self.ensure_blank_line();
        }
        self.list_depth += 1;

        for item in items {
            // Check for loose list separator (empty paragraph used as spacing marker)
            // Only treat as separator if there are multiple items (it's between real items)
            if items.len() > 1
                && item.content.len() == 1
                && let Block::Paragraph { content } = &item.content[0]
                && content.is_empty()
            {
                self.push_newline();
                continue;
            }

            let marker: String = "*".repeat(self.list_depth);
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if j == 0 {
                            if content.is_empty() {
                                self.push(&format!("{marker}\n"));
                            } else {
                                self.push(&format!("{marker} "));
                                self.emit_inlines(content)?;
                                self.push_newline();
                            }
                        } else {
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        self.emit_unordered_list(sub_items, false)?;
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        self.emit_ordered_list(sub_items, false)?;
                    }
                    _ => {
                        self.emit_blocks(std::slice::from_ref(block))?;
                    }
                }
            }
        }

        self.list_depth -= 1;
        Ok(())
    }

    fn emit_ordered_list(
        &mut self,
        items: &[ListItem],
        needs_blank_before: bool,
    ) -> Result<(), EmitError> {
        if needs_blank_before && self.list_depth == 0 {
            self.ensure_blank_line();
        }
        self.list_depth += 1;

        for item in items {
            if item.content.len() == 1
                && let Block::Paragraph { content } = &item.content[0]
                && content.is_empty()
            {
                self.push_newline();
                continue;
            }

            let marker: String = ".".repeat(self.list_depth);
            for (j, block) in item.content.iter().enumerate() {
                match block {
                    Block::Paragraph { content } => {
                        if content.is_empty() && j > 0 {
                            // Empty paragraph inside item = blank line separator
                            self.push_newline();
                        } else if j == 0 {
                            self.push(&format!("{marker} "));
                            self.emit_inlines(content)?;
                            self.push_newline();
                        } else {
                            self.emit_inlines(content)?;
                            self.push_newline();
                        }
                    }
                    Block::UnorderedList { items: sub_items } => {
                        self.emit_unordered_list(sub_items, false)?;
                    }
                    Block::OrderedList {
                        items: sub_items, ..
                    } => {
                        self.emit_ordered_list(sub_items, false)?;
                    }
                    _ => {
                        self.emit_blocks(std::slice::from_ref(block))?;
                    }
                }
            }
        }

        self.list_depth -= 1;
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        let needs_cols = alignments
            .iter()
            .any(|a| matches!(a, ColumnAlignment::Center | ColumnAlignment::Right));

        if needs_cols {
            let cols: Vec<&str> = alignments
                .iter()
                .map(|a| match a {
                    ColumnAlignment::Left | ColumnAlignment::Default => "<",
                    ColumnAlignment::Center => "^",
                    ColumnAlignment::Right => ">",
                })
                .collect();
            self.push(&format!("[cols=\"{}\"]\n", cols.join(",")));
        }
        self.push("|===\n");
        // Headers
        for (i, header) in headers.iter().enumerate() {
            self.emit_cell_span_prefix(header);
            self.push("|");
            self.emit_inlines(&header.content)?;
            if i + 1 < headers.len() {
                self.push(" ");
            }
        }
        self.push_newline();
        self.push_newline();
        // Rows
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                self.emit_cell_span_prefix(cell);
                self.push("|");
                self.emit_inlines(&cell.content)?;
                if i + 1 < row.len() {
                    self.push(" ");
                }
            }
            self.push_newline();
        }
        self.push("|===\n");
        Ok(())
    }

    fn emit_cell_span_prefix(&mut self, cell: &TableCell) {
        if cell.has_span() {
            if cell.colspan > 1 {
                self.push(&cell.colspan.to_string());
            }
            if cell.rowspan > 1 {
                self.push(&format!(".{}", cell.rowspan));
            }
            self.push("+");
        }
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) -> Result<(), EmitError> {
        for (i, inline) in inlines.iter().enumerate() {
            self.emit_inline(inline, i, inlines)?;
        }
        Ok(())
    }

    fn emit_inlines_with_indent(
        &mut self,
        inlines: &[Inline],
        indent: &str,
    ) -> Result<(), EmitError> {
        for (i, inline) in inlines.iter().enumerate() {
            match inline {
                Inline::SoftLineBreak => {
                    self.push_newline();
                    self.push(indent);
                }
                _ => self.emit_inline(inline, i, inlines)?,
            }
        }
        Ok(())
    }

    fn emit_inline(
        &mut self,
        inline: &Inline,
        idx: usize,
        siblings: &[Inline],
    ) -> Result<(), EmitError> {
        match inline {
            Inline::Text(t) => {
                let t = apply_smart_typography(t);
                self.push(&t);
            }
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
                self.push("[line-through]#");
                self.emit_inlines(content)?;
                self.push("#");
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
                self.emit_inline_code(code, idx, siblings);
            }
            Inline::Link { url, text, title } => {
                self.emit_link(url, text, title.as_deref())?;
            }
            Inline::Image {
                url,
                alt,
                title,
                link,
            } => {
                self.emit_image(url, alt, title.as_deref(), link.as_deref())?;
            }
            Inline::HardLineBreak => {
                // Remove trailing spaces before the hard line break marker
                let trimmed = self.output.trim_end_matches(' ');
                self.output.truncate(trimmed.len());
                self.push(" +\n");
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

    fn emit_inline_code(&mut self, code: &str, idx: usize, siblings: &[Inline]) {
        let needs_pass_macro = code.contains("++");
        // Only apply passthrough for actual AsciiDoc-significant patterns:
        // - "..." (ellipsis in code)
        // - "->" (arrow)
        // - "{attr}" (attribute reference pattern: { followed by alphanumeric then })
        let has_attr_ref = has_attribute_reference(code);
        let has_dots = code.contains("...");
        let has_arrow = code.contains("->");
        let needs_plus_passthrough = has_dots || has_arrow || has_attr_ref;

        // Check if code is adjacent to text (no space after closing backtick)
        let is_adjacent = is_code_adjacent_to_text(idx, siblings);

        if needs_pass_macro {
            self.push(&format!("`pass:c[{code}]`"));
        } else if needs_plus_passthrough {
            self.push(&format!("`+{code}+`"));
        } else if is_adjacent {
            self.push(&format!("``{code}``"));
        } else {
            self.push(&format!("`{code}`"));
        }
    }

    fn emit_link(
        &mut self,
        url: &str,
        text: &[Inline],
        _title: Option<&str>,
    ) -> Result<(), EmitError> {
        if let Some(anchor) = url.strip_prefix('#') {
            self.push("<<");
            self.push(anchor);
            self.push(",");
            self.emit_inlines(text)?;
            self.push(">>");
            return Ok(());
        }

        let text_str = inlines_to_plain_text(text);
        if text_str == url
            || text_str == url.trim_end_matches('/')
            || url == text_str.trim_end_matches('/')
        {
            self.push(url);
            return Ok(());
        }

        let is_relative = !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("ftp://");
        let needs_quoting = text_str.contains(',');

        if is_relative {
            self.push("link:");
            self.push(url);
        } else {
            self.push(url);
        }
        self.push("[");
        if needs_quoting {
            self.push("\"");
            self.emit_inlines(text)?;
            self.push("\"");
        } else {
            self.emit_inlines(text)?;
        }
        self.push("]");
        Ok(())
    }

    fn emit_image(
        &mut self,
        url: &str,
        alt: &[Inline],
        _title: Option<&str>,
        link: Option<&str>,
    ) -> Result<(), EmitError> {
        let alt_text = inlines_to_plain_text(alt);
        let needs_quoting = alt_text.contains(',');
        self.push("image:");
        self.push(url);
        self.push("[");
        if needs_quoting {
            self.push("\"");
            self.push(&alt_text);
            self.push("\"");
        } else {
            self.push(&alt_text);
        }
        if let Some(link_url) = link {
            self.push(",link=");
            self.push(link_url);
        }
        self.push("]");
        Ok(())
    }
}

/// Check if an inline code element is followed by a word character (no space)
fn is_code_adjacent_to_text(idx: usize, siblings: &[Inline]) -> bool {
    if idx + 1 < siblings.len()
        && let Inline::Text(next_text) = &siblings[idx + 1]
        && let Some(ch) = next_text.chars().next()
    {
        return ch.is_alphanumeric();
    }
    false
}

/// Check if code contains an AsciiDoc attribute reference like {foo}
fn has_attribute_reference(code: &str) -> bool {
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            let start = i + 1;
            i += 1;
            // Look for closing }
            while i < chars.len() && chars[i] != '}' {
                i += 1;
            }
            if i < chars.len() && i > start {
                // Check that content between braces is non-empty and looks like an attribute
                let content: String = chars[start..i].iter().collect();
                if content
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Apply smart typography conversions (SmartyPants-style)
/// Converts: ... → …, --- → —, -- → –, <<text>> → «text»
fn apply_smart_typography(text: &str) -> String {
    let mut t = text.to_string();

    // Normalize spaced dots first: ". . ." → "..."
    t = t.replace(". . .", "...");

    // Ellipsis: "..." → "…" (U+2026)
    t = t.replace("...", "\u{2026}");

    // Em dash: "---" → "—" (U+2014) - must come before en dash
    t = t.replace("---", "\u{2014}");

    // En dash: "--" → "–" (U+2013)
    t = t.replace("--", "\u{2013}");

    // Guillemets with nbsp: "<< " → "«\u{a0}" and " >>" → "\u{a0}»"
    t = t
        .replace("<< ", "\u{ab}\u{a0}")
        .replace(" >>", "\u{a0}\u{bb}");

    // Guillemets without space: "<<" → "«" and ">>" → "»"
    t = t.replace("<<", "\u{ab}").replace(">>", "\u{bb}");

    // Non-breaking space → {nbsp} for AsciiDoc
    t = t.replace('\u{a0}', "{nbsp}");

    t
}

fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => s.push_str(t),
            Inline::Bold(content)
            | Inline::Italic(content)
            | Inline::BoldItalic(content)
            | Inline::Strikethrough(content)
            | Inline::Superscript(content)
            | Inline::Subscript(content) => {
                s.push_str(&inlines_to_plain_text(content));
            }
            Inline::InlineCode(code) => s.push_str(code),
            Inline::Link { text, .. } => s.push_str(&inlines_to_plain_text(text)),
            Inline::SoftLineBreak | Inline::HardLineBreak => s.push(' '),
            _ => {}
        }
    }
    s
}
