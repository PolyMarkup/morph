pub mod ast;
pub mod error;
pub mod format;
pub mod formats;

use ast::Document;
use error::{ConvertError, EmitError, ParseError};
use format::Format;

pub fn convert(input: &str, from: Format, to: Format) -> Result<String, ConvertError> {
    let doc = parse(input, from)?;
    let output = emit(&doc, to)?;
    Ok(output)
}

pub fn parse(input: &str, format: Format) -> Result<Document, ParseError> {
    use format::Parser;
    match format {
        Format::Markdown => formats::markdown::MarkdownParser.parse(input),
        Format::AsciiDoc => formats::asciidoc::AsciiDocParser.parse(input),
        Format::Rst => formats::rst::RstParser.parse(input),
        Format::Typst => formats::typst::TypstParser.parse(input),
        Format::Latex => formats::latex::LatexParser.parse(input),
        Format::Djot => formats::djot::DjotParser.parse(input),
        Format::Org => formats::org::OrgParser.parse(input),
        Format::Textile => formats::textile::TextileParser.parse(input),
        Format::Html => formats::html::HtmlParser.parse(input),
        Format::DocBook => formats::docbook::DocBookParser.parse(input),
    }
}

pub fn emit(doc: &Document, format: Format) -> Result<String, EmitError> {
    use format::Emitter;
    match format {
        Format::Markdown => formats::markdown::MarkdownEmitter.emit(doc),
        Format::AsciiDoc => formats::asciidoc::AsciiDocEmitter.emit(doc),
        Format::Rst => formats::rst::RstEmitter.emit(doc),
        Format::Typst => formats::typst::TypstEmitter.emit(doc),
        Format::Latex => formats::latex::LatexEmitter.emit(doc),
        Format::Djot => formats::djot::DjotEmitter.emit(doc),
        Format::Org => formats::org::OrgEmitter.emit(doc),
        Format::Textile => formats::textile::TextileEmitter.emit(doc),
        Format::Html => formats::html::HtmlEmitter.emit(doc),
        Format::DocBook => formats::docbook::DocBookEmitter.emit(doc),
    }
}
