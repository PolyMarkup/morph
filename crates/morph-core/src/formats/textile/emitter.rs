use crate::ast::Document;
use crate::error::EmitError;
use crate::format::Emitter;
use crate::formats::lightweight::{Flavor, emit_document};

pub struct TextileEmitter;

impl Emitter for TextileEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        emit_document(doc, Flavor::Textile)
    }
}
