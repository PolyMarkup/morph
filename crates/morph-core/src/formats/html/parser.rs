use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;
use crate::formats::strict_xml::{XmlElement, XmlNode, parse_xml};

pub struct HtmlParser;

impl Parser for HtmlParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let nodes = parse_xml(input, true)?;
        let body = unwrap_html_document(&nodes)?;
        Ok(Document {
            children: parse_block_nodes(body)?,
        })
    }
}

fn unwrap_html_document(nodes: &[XmlNode]) -> Result<&[XmlNode], ParseError> {
    let elements: Vec<&XmlElement> = nodes
        .iter()
        .filter_map(|node| match node {
            XmlNode::Element(element) => Some(element),
            XmlNode::Text(text) if text.trim().is_empty() => None,
            XmlNode::Text(_) => None,
        })
        .collect();
    if elements.len() == 1 && elements[0].name == "html" {
        validate_attrs(elements[0], &["lang"])?;
        let body = elements[0]
            .element_children()
            .find(|child| child.name == "body")
            .ok_or_else(|| invalid("<html> must contain <body>"))?;
        validate_attrs(body, &[])?;
        for child in elements[0].element_children() {
            if child.name != "head" && child.name != "body" {
                return Err(invalid(format!(
                    "unsupported <{}> child of <html>",
                    child.name
                )));
            }
            if child.name == "head" && has_meaningful_content(&child.children) {
                return Err(invalid(
                    "strict HTML does not accept document metadata in <head>",
                ));
            }
        }
        return Ok(&body.children);
    }
    Ok(nodes)
}

fn parse_block_nodes(nodes: &[XmlNode]) -> Result<Vec<Block>, ParseError> {
    let mut blocks = Vec::new();
    for node in nodes {
        match node {
            XmlNode::Text(text) if text.trim().is_empty() => {}
            XmlNode::Text(_) => {
                return Err(invalid(
                    "block-level text must be wrapped in a supported element such as <p>",
                ));
            }
            XmlNode::Element(element) => blocks.push(parse_block(element)?),
        }
    }
    Ok(blocks)
}

fn parse_block(element: &XmlElement) -> Result<Block, ParseError> {
    match element.name.as_str() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            validate_attrs(element, &[])?;
            Ok(Block::Heading {
                level: element.name[1..].parse().unwrap(),
                content: parse_inline_nodes(&element.children)?,
            })
        }
        "p" => {
            validate_attrs(element, &[])?;
            Ok(Block::Paragraph {
                content: parse_inline_nodes(&element.children)?,
            })
        }
        "pre" if element.attr("data-morph-raw-format").is_some() => {
            validate_attrs(element, &["data-morph-raw-format"])?;
            Ok(Block::RawBlock {
                format: nonempty(element.attr("data-morph-raw-format")),
                content: text_only(element)?,
            })
        }
        "pre" => parse_code_block(element),
        "blockquote" => {
            validate_attrs(element, &[])?;
            Ok(Block::BlockQuote {
                children: parse_block_nodes(&element.children)?,
            })
        }
        "ul" => parse_list(element, false),
        "ol" => parse_list(element, true),
        "dl" => parse_description_list(element),
        "table" => parse_table(element),
        "hr" => {
            validate_attrs(element, &[])?;
            require_empty(element)?;
            Ok(Block::HorizontalRule)
        }
        name => Err(invalid(format!(
            "unsupported strict HTML block element <{name}>"
        ))),
    }
}

fn parse_code_block(element: &XmlElement) -> Result<Block, ParseError> {
    validate_attrs(element, &[])?;
    let children: Vec<&XmlElement> = element.element_children().collect();
    if children.len() != 1 || children[0].name != "code" {
        return Err(invalid("<pre> must contain exactly one <code> element"));
    }
    let code = children[0];
    validate_attrs(code, &["class"])?;
    let language = code
        .attr("class")
        .map(|class| {
            class
                .strip_prefix("language-")
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| invalid("<code> class must use 'language-NAME'"))
        })
        .transpose()?;
    Ok(Block::CodeBlock {
        language,
        content: text_only(code)?,
    })
}

fn parse_list(element: &XmlElement, ordered: bool) -> Result<Block, ParseError> {
    validate_attrs(element, if ordered { &["start"] } else { &[] })?;
    let start = element
        .attr("start")
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| invalid("ordered-list start must be a positive integer"))
        })
        .transpose()?
        .unwrap_or(1);
    let mut items = Vec::new();
    for node in &element.children {
        match node {
            XmlNode::Text(text) if text.trim().is_empty() => {}
            XmlNode::Element(item) if item.name == "li" => {
                validate_attrs(item, &[])?;
                items.push(ListItem {
                    content: parse_block_nodes(&item.children)?,
                });
            }
            _ => return Err(invalid("lists may only contain <li> elements")),
        }
    }
    Ok(if ordered {
        Block::OrderedList { start, items }
    } else {
        Block::UnorderedList { items }
    })
}

fn parse_description_list(element: &XmlElement) -> Result<Block, ParseError> {
    validate_attrs(element, &[])?;
    let children: Vec<&XmlElement> = element.element_children().collect();
    let mut items = Vec::new();
    let mut index = 0;
    while index < children.len() {
        let term = children[index];
        if term.name != "dt" {
            return Err(invalid("<dl> entries must begin with <dt>"));
        }
        validate_attrs(term, &[])?;
        index += 1;
        let mut definitions = Vec::new();
        while index < children.len() && children[index].name == "dd" {
            let definition = children[index];
            validate_attrs(definition, &[])?;
            definitions.push(parse_block_nodes(&definition.children)?);
            index += 1;
        }
        if definitions.is_empty() {
            return Err(invalid("<dt> must be followed by at least one <dd>"));
        }
        items.push(DescriptionItem {
            term: parse_inline_nodes(&term.children)?,
            definitions,
        });
    }
    Ok(Block::DescriptionList { items })
}

fn parse_table(element: &XmlElement) -> Result<Block, ParseError> {
    validate_attrs(element, &[])?;
    let thead = element
        .element_children()
        .find(|child| child.name == "thead")
        .ok_or_else(|| invalid("strict HTML tables require <thead>"))?;
    let tbody = element
        .element_children()
        .find(|child| child.name == "tbody")
        .ok_or_else(|| invalid("strict HTML tables require <tbody>"))?;
    if element
        .element_children()
        .any(|child| child.name != "thead" && child.name != "tbody")
    {
        return Err(invalid("unsupported child in <table>"));
    }
    validate_attrs(thead, &[])?;
    validate_attrs(tbody, &[])?;
    let head_rows: Vec<&XmlElement> = thead.element_children().collect();
    if head_rows.len() != 1 || head_rows[0].name != "tr" {
        return Err(invalid("<thead> must contain exactly one <tr>"));
    }
    let (headers, alignments) = parse_table_row(head_rows[0], "th", true)?;
    let mut rows = Vec::new();
    for row in tbody.element_children() {
        if row.name != "tr" {
            return Err(invalid("<tbody> may only contain <tr>"));
        }
        rows.push(parse_table_row(row, "td", false)?.0);
    }
    Ok(Block::Table {
        headers,
        alignments,
        rows,
    })
}

fn parse_table_row(
    element: &XmlElement,
    expected_cell: &str,
    collect_alignments: bool,
) -> Result<(Vec<TableCell>, Vec<ColumnAlignment>), ParseError> {
    validate_attrs(element, &[])?;
    let mut cells = Vec::new();
    let mut alignments = Vec::new();
    for cell in element.element_children() {
        if cell.name != expected_cell {
            return Err(invalid(format!(
                "<tr> expected <{expected_cell}>, found <{}>",
                cell.name
            )));
        }
        validate_attrs(cell, &["colspan", "rowspan", "align"])?;
        let colspan = positive_attr(cell, "colspan")?.unwrap_or(1);
        let rowspan = positive_attr(cell, "rowspan")?.unwrap_or(1);
        if collect_alignments {
            alignments.push(match cell.attr("align") {
                Some("left") => ColumnAlignment::Left,
                Some("center") => ColumnAlignment::Center,
                Some("right") => ColumnAlignment::Right,
                Some(value) => return Err(invalid(format!("unsupported alignment '{value}'"))),
                None => ColumnAlignment::Default,
            });
        }
        cells.push(TableCell::with_span(
            parse_inline_nodes(&cell.children)?,
            colspan,
            rowspan,
        ));
    }
    Ok((cells, alignments))
}

fn parse_inline_nodes(nodes: &[XmlNode]) -> Result<Vec<Inline>, ParseError> {
    let mut inlines = Vec::new();
    for node in nodes {
        match node {
            XmlNode::Text(text) => append_text(&mut inlines, text),
            XmlNode::Element(element) => inlines.push(parse_inline(element)?),
        }
    }
    Ok(inlines)
}

fn parse_inline(element: &XmlElement) -> Result<Inline, ParseError> {
    match element.name.as_str() {
        "strong" | "b" => {
            validate_attrs(element, &[])?;
            let content = parse_inline_nodes(&element.children)?;
            if let [Inline::Italic(inner)] = content.as_slice() {
                Ok(Inline::BoldItalic(inner.clone()))
            } else {
                Ok(Inline::Bold(content))
            }
        }
        "em" | "i" => {
            validate_attrs(element, &[])?;
            let content = parse_inline_nodes(&element.children)?;
            if let [Inline::Bold(inner)] = content.as_slice() {
                Ok(Inline::BoldItalic(inner.clone()))
            } else {
                Ok(Inline::Italic(content))
            }
        }
        "del" | "s" => wrapped_inline(element, Inline::Strikethrough),
        "sup" => wrapped_inline(element, Inline::Superscript),
        "sub" => wrapped_inline(element, Inline::Subscript),
        "code" => {
            validate_attrs(element, &[])?;
            Ok(Inline::InlineCode(text_only(element)?))
        }
        "a" => parse_link(element),
        "img" => parse_image(element, None),
        "br" => {
            validate_attrs(element, &[])?;
            require_empty(element)?;
            Ok(Inline::HardLineBreak)
        }
        "span" if element.attr("data-morph-raw-format").is_some() => {
            validate_attrs(element, &["data-morph-raw-format"])?;
            Ok(Inline::RawInline {
                format: nonempty(element.attr("data-morph-raw-format")),
                content: text_only(element)?,
            })
        }
        name => Err(invalid(format!(
            "unsupported strict HTML inline element <{name}>"
        ))),
    }
}

fn parse_link(element: &XmlElement) -> Result<Inline, ParseError> {
    validate_attrs(element, &["href", "title"])?;
    let url = element
        .attr("href")
        .ok_or_else(|| invalid("<a> requires href"))?
        .to_string();
    let content = parse_inline_nodes(&element.children)?;
    if let [
        Inline::Image {
            url: image_url,
            alt,
            title,
            link: None,
        },
    ] = content.as_slice()
    {
        return Ok(Inline::Image {
            url: image_url.clone(),
            alt: alt.clone(),
            title: title.clone(),
            link: Some(url),
        });
    }
    Ok(Inline::Link {
        url,
        text: content,
        title: element.attr("title").map(str::to_string),
    })
}

fn parse_image(element: &XmlElement, link: Option<String>) -> Result<Inline, ParseError> {
    validate_attrs(element, &["src", "alt", "title"])?;
    require_empty(element)?;
    Ok(Inline::Image {
        url: element
            .attr("src")
            .ok_or_else(|| invalid("<img> requires src"))?
            .to_string(),
        alt: element
            .attr("alt")
            .map(|alt| vec![Inline::Text(alt.to_string())])
            .unwrap_or_default(),
        title: element.attr("title").map(str::to_string),
        link,
    })
}

fn wrapped_inline(
    element: &XmlElement,
    constructor: fn(Vec<Inline>) -> Inline,
) -> Result<Inline, ParseError> {
    validate_attrs(element, &[])?;
    Ok(constructor(parse_inline_nodes(&element.children)?))
}

fn append_text(inlines: &mut Vec<Inline>, text: &str) {
    for (index, part) in text.split('\n').enumerate() {
        if index > 0 {
            inlines.push(Inline::SoftLineBreak);
        }
        if !part.is_empty() {
            inlines.push(Inline::Text(part.to_string()));
        }
    }
}

fn text_only(element: &XmlElement) -> Result<String, ParseError> {
    let mut text = String::new();
    for child in &element.children {
        match child {
            XmlNode::Text(content) => text.push_str(content),
            XmlNode::Element(child) => {
                return Err(invalid(format!(
                    "<{}> may only contain text, found <{}>",
                    element.name, child.name
                )));
            }
        }
    }
    Ok(text)
}

fn validate_attrs(element: &XmlElement, allowed: &[&str]) -> Result<(), ParseError> {
    for (name, _) in &element.attributes {
        if !allowed.contains(&name.as_str()) {
            return Err(invalid(format!(
                "unsupported '{name}' attribute on <{}>",
                element.name
            )));
        }
    }
    Ok(())
}

fn positive_attr(element: &XmlElement, name: &str) -> Result<Option<u32>, ParseError> {
    element
        .attr(name)
        .map(|value| {
            value
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid(format!("{name} must be a positive integer")))
        })
        .transpose()
}

fn require_empty(element: &XmlElement) -> Result<(), ParseError> {
    if has_meaningful_content(&element.children) {
        return Err(invalid(format!("<{}> must be empty", element.name)));
    }
    Ok(())
}

fn has_meaningful_content(nodes: &[XmlNode]) -> bool {
    nodes.iter().any(|node| match node {
        XmlNode::Element(_) => true,
        XmlNode::Text(text) => !text.trim().is_empty(),
    })
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value.filter(|value| !value.is_empty()).map(str::to_string)
}

fn invalid(message: impl Into<String>) -> ParseError {
    ParseError::InvalidInput(message.into())
}
