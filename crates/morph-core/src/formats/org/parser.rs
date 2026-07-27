use crate::ast::Document;
use crate::error::ParseError;
use crate::format::Parser;
use crate::formats::lightweight::{Flavor, parse_document};

pub struct OrgParser;

impl Parser for OrgParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        parse_document(input, Flavor::Org)
    }
}
