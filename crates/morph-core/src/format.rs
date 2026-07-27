use crate::ast::Document;
use crate::error::{EmitError, ParseError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Markdown,
    AsciiDoc,
    Rst,
    Typst,
    Latex,
}

impl Format {
    pub const ALL: &'static [Format] = &[
        Format::Markdown,
        Format::AsciiDoc,
        Format::Rst,
        Format::Typst,
        Format::Latex,
    ];

    /// Short identifier used in the CLI and API (e.g. "md", "adoc").
    pub fn id(&self) -> &'static str {
        match self {
            Format::Markdown => "md",
            Format::AsciiDoc => "adoc",
            Format::Rst => "rst",
            Format::Typst => "typ",
            Format::Latex => "tex",
        }
    }

    /// Human-readable name (e.g. "Markdown", "reStructuredText").
    pub fn name(&self) -> &'static str {
        match self {
            Format::Markdown => "Markdown",
            Format::AsciiDoc => "AsciiDoc",
            Format::Rst => "reStructuredText",
            Format::Typst => "Typst",
            Format::Latex => "LaTeX",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Format> {
        Self::from_name(ext)
    }

    pub fn from_name(name: &str) -> Option<Format> {
        match name.to_lowercase().as_str() {
            "md" | "markdown" => Some(Format::Markdown),
            "adoc" | "asciidoc" | "asc" => Some(Format::AsciiDoc),
            "rst" | "restructuredtext" => Some(Format::Rst),
            "typ" | "typst" => Some(Format::Typst),
            "tex" | "latex" => Some(Format::Latex),
            _ => None,
        }
    }
}

pub trait Parser {
    fn parse(&self, input: &str) -> Result<Document, ParseError>;
}

pub trait Emitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError>;
}
