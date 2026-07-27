use crate::ast::Document;
use crate::error::ParseError;
use crate::format::Parser;
use crate::formats::lightweight::{Flavor, parse_document};

pub struct TextileParser;

impl Parser for TextileParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        parse_document(input, Flavor::Textile)
    }
}
