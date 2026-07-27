#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph {
        content: Vec<Inline>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    BlockQuote {
        children: Vec<Block>,
    },
    UnorderedList {
        items: Vec<ListItem>,
    },
    OrderedList {
        start: u32,
        items: Vec<ListItem>,
    },
    DescriptionList {
        items: Vec<DescriptionItem>,
    },
    Table {
        headers: Vec<TableCell>,
        alignments: Vec<ColumnAlignment>,
        rows: Vec<Vec<TableCell>>,
    },
    HorizontalRule,
    RawBlock {
        format: Option<String>,
        content: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DescriptionItem {
    pub term: Vec<Inline>,
    pub definitions: Vec<Vec<Block>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub content: Vec<Inline>,
    pub colspan: u32,
    pub rowspan: u32,
}

impl TableCell {
    pub fn new(content: Vec<Inline>) -> Self {
        TableCell {
            content,
            colspan: 1,
            rowspan: 1,
        }
    }

    pub fn with_span(content: Vec<Inline>, colspan: u32, rowspan: u32) -> Self {
        TableCell {
            content,
            colspan,
            rowspan,
        }
    }

    pub fn has_span(&self) -> bool {
        self.colspan > 1 || self.rowspan > 1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnAlignment {
    Left,
    Center,
    Right,
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    BoldItalic(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    InlineCode(String),
    Link {
        url: String,
        text: Vec<Inline>,
        title: Option<String>,
    },
    Image {
        url: String,
        alt: Vec<Inline>,
        title: Option<String>,
        link: Option<String>,
    },
    HardLineBreak,
    SoftLineBreak,
    RawInline {
        format: Option<String>,
        content: String,
    },
}
