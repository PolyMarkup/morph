use crate::ast::Document;
use crate::error::EmitError;
use crate::format::Emitter;
use crate::formats::lightweight::{Flavor, emit_document};

pub struct DjotEmitter;

impl Emitter for DjotEmitter {
    fn emit(&self, doc: &Document) -> Result<String, EmitError> {
        emit_document(doc, Flavor::Djot)
    }
}
