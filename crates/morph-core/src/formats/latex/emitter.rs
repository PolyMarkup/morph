use crate::ast::*;
use crate::error::EmitError;
use crate::format::Emitter;

pub struct LatexEmitter;

impl Emitter for LatexEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        let mut context = LatexEmitContext::new();
        context.emit_blocks(&doc.children)?;
        Ok(context.finish())
    }
}

struct LatexEmitContext {
    output: String,
}

impl LatexEmitContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn finish(self) -> String {
        let trimmed = self.output.trim_end_matches('\n');
        format!("{trimmed}\n")
    }

    fn push(&mut self, value: &str) {
        self.output.push_str(value);
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
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 {
                match block {
                    Block::Paragraph { content } if content.is_empty() => continue,
                    _ => self.ensure_blank_line(),
                }
            }

            match block {
                Block::Heading { level, content } => {
                    let command = match level {
                        1 => "section",
                        2 => "subsection",
                        3 => "subsubsection",
                        4 => "paragraph",
                        _ => "subparagraph",
                    };
                    self.push("\\");
                    self.push(command);
                    self.push("{");
                    self.emit_inlines(content)?;
                    self.push("}\n");
                }
                Block::Paragraph { content } => {
                    if content.is_empty() {
                        continue;
                    }
                    self.emit_inlines(content)?;
                    self.push_newline();
                }
                Block::CodeBlock { language, content } => {
                    self.push("\\begin{lstlisting}");
                    if let Some(language) = language {
                        self.push("[language=");
                        self.push(language);
                        self.push("]");
                    }
                    self.push_newline();
                    self.push(content);
                    self.push_newline();
                    self.push("\\end{lstlisting}\n");
                }
                Block::BlockQuote { children } => {
                    self.push("\\begin{quote}\n");
                    let inner = emit_blocks_to_string(children)?;
                    self.push(inner.trim_end_matches('\n'));
                    self.push("\n\\end{quote}\n");
                }
                Block::UnorderedList { items } => {
                    self.emit_list("itemize", items, 0)?;
                }
                Block::OrderedList { items, .. } => {
                    self.emit_list("enumerate", items, 0)?;
                }
                Block::DescriptionList { items } => {
                    self.emit_description_list(items, 0)?;
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                } => {
                    self.emit_table(headers, alignments, rows)?;
                }
                Block::HorizontalRule => self.push("\\hrule\n"),
                Block::RawBlock { content, .. } => {
                    self.push(content);
                    self.push_newline();
                }
            }
        }

        Ok(())
    }

    fn emit_list(
        &mut self,
        environment: &str,
        items: &[ListItem],
        indent: usize,
    ) -> Result<(), EmitError> {
        let prefix = " ".repeat(indent);
        self.push(&prefix);
        self.push("\\begin{");
        self.push(environment);
        self.push("}\n");

        for item in items {
            let mut emitted_first_paragraph = false;
            for block in &item.content {
                match block {
                    Block::Paragraph { content } if !emitted_first_paragraph => {
                        self.push(&prefix);
                        self.push("  \\item ");
                        self.emit_inlines(content)?;
                        self.push_newline();
                        emitted_first_paragraph = true;
                    }
                    Block::Paragraph { content } => {
                        self.push(&prefix);
                        self.push("    ");
                        self.emit_inlines(content)?;
                        self.push_newline();
                    }
                    Block::UnorderedList { items } => {
                        self.emit_list("itemize", items, indent + 4)?;
                    }
                    Block::OrderedList { items, .. } => {
                        self.emit_list("enumerate", items, indent + 4)?;
                    }
                    Block::DescriptionList { items } => {
                        self.emit_description_list(items, indent + 4)?;
                    }
                    other => {
                        let inner = emit_blocks_to_string(std::slice::from_ref(other))?;
                        for line in inner.lines() {
                            self.push(&prefix);
                            self.push("    ");
                            self.push(line);
                            self.push_newline();
                        }
                    }
                }
            }
        }

        self.push(&prefix);
        self.push("\\end{");
        self.push(environment);
        self.push("}\n");
        Ok(())
    }

    fn emit_description_list(
        &mut self,
        items: &[DescriptionItem],
        indent: usize,
    ) -> Result<(), EmitError> {
        let prefix = " ".repeat(indent);
        self.push(&prefix);
        self.push("\\begin{description}\n");
        for item in items {
            self.push(&prefix);
            self.push("  \\item[");
            self.emit_inlines(&item.term)?;
            self.push("] ");

            let mut first = true;
            for definition in &item.definitions {
                for block in definition {
                    if let Block::Paragraph { content } = block {
                        if !first {
                            self.push(" ");
                        }
                        self.emit_inlines(content)?;
                        first = false;
                    }
                }
            }
            self.push_newline();
        }
        self.push(&prefix);
        self.push("\\end{description}\n");
        Ok(())
    }

    fn emit_table(
        &mut self,
        headers: &[TableCell],
        alignments: &[ColumnAlignment],
        rows: &[Vec<TableCell>],
    ) -> Result<(), EmitError> {
        let column_count = logical_column_count(headers).max(
            rows.iter()
                .map(|row| logical_column_count(row))
                .max()
                .unwrap_or(0),
        );
        let mut column_spec = String::new();
        for index in 0..column_count {
            column_spec.push(
                match alignments.get(index).unwrap_or(&ColumnAlignment::Default) {
                    ColumnAlignment::Center => 'c',
                    ColumnAlignment::Right => 'r',
                    ColumnAlignment::Left | ColumnAlignment::Default => 'l',
                },
            );
        }
        if column_spec.is_empty() {
            column_spec.push('l');
        }

        self.push("\\begin{tabular}{");
        self.push(&column_spec);
        self.push("}\n");
        self.emit_table_row(headers)?;
        self.push("\\hline\n");
        for row in rows {
            self.emit_table_row(row)?;
        }
        self.push("\\end{tabular}\n");
        Ok(())
    }

    fn emit_table_row(&mut self, cells: &[TableCell]) -> Result<(), EmitError> {
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                self.push(" & ");
            }
            self.emit_table_cell(cell)?;
        }
        self.push(" \\\\\n");
        Ok(())
    }

    fn emit_table_cell(&mut self, cell: &TableCell) -> Result<(), EmitError> {
        if cell.rowspan > 1 {
            self.push("\\multirow{");
            self.push(&cell.rowspan.to_string());
            self.push("}{*}{");
        }
        if cell.colspan > 1 {
            self.push("\\multicolumn{");
            self.push(&cell.colspan.to_string());
            self.push("}{l}{");
        }
        self.emit_inlines(&cell.content)?;
        if cell.colspan > 1 {
            self.push("}");
        }
        if cell.rowspan > 1 {
            self.push("}");
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
            Inline::Text(text) => self.push(&escape_latex(text)),
            Inline::Bold(content) => self.emit_wrapped("textbf", content)?,
            Inline::Italic(content) => self.emit_wrapped("textit", content)?,
            Inline::BoldItalic(content) => {
                self.push("\\textbf{\\textit{");
                self.emit_inlines(content)?;
                self.push("}}");
            }
            Inline::Strikethrough(content) => self.emit_wrapped("sout", content)?,
            Inline::Superscript(content) => self.emit_wrapped("textsuperscript", content)?,
            Inline::Subscript(content) => self.emit_wrapped("textsubscript", content)?,
            Inline::InlineCode(code) => {
                self.push("\\texttt{");
                self.push(&escape_latex(code));
                self.push("}");
            }
            Inline::Link { url, text, .. } => {
                self.push("\\href{");
                self.push(&escape_latex_url(url));
                self.push("}{");
                self.emit_inlines(text)?;
                self.push("}");
            }
            Inline::Image { url, link, .. } => {
                if let Some(link) = link {
                    self.push("\\href{");
                    self.push(&escape_latex_url(link));
                    self.push("}{");
                }
                self.push("\\includegraphics{");
                self.push(&escape_latex_url(url));
                self.push("}");
                if link.is_some() {
                    self.push("}");
                }
            }
            Inline::HardLineBreak => self.push("\\\\\n"),
            Inline::SoftLineBreak => self.push_newline(),
            Inline::RawInline { content, .. } => self.push(content),
        }
        Ok(())
    }

    fn emit_wrapped(&mut self, command: &str, content: &[Inline]) -> Result<(), EmitError> {
        self.push("\\");
        self.push(command);
        self.push("{");
        self.emit_inlines(content)?;
        self.push("}");
        Ok(())
    }
}

fn logical_column_count(cells: &[TableCell]) -> usize {
    cells.iter().map(|cell| cell.colspan as usize).sum()
}

fn escape_latex(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '#' => escaped.push_str("\\#"),
            '$' => escaped.push_str("\\$"),
            '%' => escaped.push_str("\\%"),
            '&' => escaped.push_str("\\&"),
            '_' => escaped.push_str("\\_"),
            '^' => escaped.push_str("\\textasciicircum{}"),
            '~' => escaped.push_str("\\textasciitilde{}"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn escape_latex_url(url: &str) -> String {
    let mut escaped = String::with_capacity(url.len());
    for character in url.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '#' => escaped.push_str("\\#"),
            '$' => escaped.push_str("\\$"),
            '%' => escaped.push_str("\\%"),
            '&' => escaped.push_str("\\&"),
            '_' => escaped.push_str("\\_"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn emit_blocks_to_string(blocks: &[Block]) -> Result<String, EmitError> {
    let mut context = LatexEmitContext::new();
    context.emit_blocks(blocks)?;
    Ok(context.finish())
}
