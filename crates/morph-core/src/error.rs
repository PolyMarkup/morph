use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    InvalidInput(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidInput(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub enum EmitError {
    UnsupportedFeature(String),
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::UnsupportedFeature(msg) => write!(f, "Emit error: {msg}"),
        }
    }
}

impl std::error::Error for EmitError {}

#[derive(Debug)]
pub enum ConvertError {
    Parse(ParseError),
    Emit(EmitError),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::Parse(e) => write!(f, "{e}"),
            ConvertError::Emit(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConvertError {}

impl From<ParseError> for ConvertError {
    fn from(e: ParseError) -> Self {
        ConvertError::Parse(e)
    }
}

impl From<EmitError> for ConvertError {
    fn from(e: EmitError) -> Self {
        ConvertError::Emit(e)
    }
}
