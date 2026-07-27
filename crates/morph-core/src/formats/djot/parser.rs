use crate::ast::Document;
use crate::error::ParseError;
use crate::format::Parser;
use crate::formats::lightweight::{Flavor, parse_document};

pub struct DjotParser;

impl Parser for DjotParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        parse_document(input, Flavor::Djot)
    }
}
