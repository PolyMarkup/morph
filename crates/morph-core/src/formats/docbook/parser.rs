use crate::ast::*;
use crate::error::ParseError;
use crate::format::Parser;
use crate::formats::strict_xml::{XmlElement, XmlNode, parse_xml};

pub struct DocBookParser;

impl Parser for DocBookParser {
    fn parse(&self, input: &str) -> Result<Document, ParseError> {
        let nodes = parse_xml(input, false)?;
        let root = single_root(&nodes)?;
        if root.name != "article" {
            return Err(invalid("strict DocBook input must have an <article> root"));
        }
        validate_attrs(root, &["xmlns", "xmlns:xlink", "version"])?;
        if root.attr("xmlns") != Some("http://docbook.org/ns/docbook") {
            return Err(invalid("DocBook <article> requires the DocBook namespace"));
        }
        Ok(Document {
            children: parse_block_nodes(&root.children)?,
        })
    }
}

fn parse_block_nodes(nodes: &[XmlNode]) -> Result<Vec<Block>, ParseError> {
    let mut blocks = Vec::new();
    for node in nodes {
        match node {
            XmlNode::Text(text) if text.trim().is_empty() => {}
            XmlNode::Text(_) => {
                return Err(invalid(
                    "DocBook block text must be wrapped in a supported element",
                ));
            }
            XmlNode::Element(element) => blocks.push(parse_block(element)?),
        }
    }
    Ok(blocks)
}

fn parse_block(element: &XmlElement) -> Result<Block, ParseError> {
    match element.name.as_str() {
        "bridgehead" => {
            validate_attrs(element, &["renderas"])?;
            let render = element
                .attr("renderas")
                .and_then(|value| value.strip_prefix("sect"))
                .ok_or_else(|| invalid("<bridgehead> renderas must be sect1 through sect6"))?;
            let level = render
                .parse::<u8>()
                .ok()
                .filter(|level| (1..=6).contains(level))
                .ok_or_else(|| invalid("<bridgehead> renderas must be sect1 through sect6"))?;
            Ok(Block::Heading {
                level,
                content: parse_inline_nodes(&element.children)?,
            })
        }
        "para" if element.attr("role") == Some("morph-horizontal-rule") => {
            validate_attrs(element, &["role"])?;
            require_empty(element)?;
            Ok(Block::HorizontalRule)
        }
        "para" => {
            validate_attrs(element, &[])?;
            Ok(Block::Paragraph {
                content: parse_inline_nodes(&element.children)?,
            })
        }
        "programlisting" if element.attr("role") == Some("morph-raw") => {
            validate_attrs(element, &["role", "remap"])?;
            Ok(Block::RawBlock {
                format: element.attr("remap").map(str::to_string),
                content: text_only(element)?,
            })
        }
        "programlisting" => {
            validate_attrs(element, &["language"])?;
            Ok(Block::CodeBlock {
                language: element.attr("language").map(str::to_string),
                content: text_only(element)?,
            })
        }
        "blockquote" => {
            validate_attrs(element, &[])?;
            Ok(Block::BlockQuote {
                children: parse_block_nodes(&element.children)?,
            })
        }
        "itemizedlist" => parse_list(element, false),
        "orderedlist" => parse_list(element, true),
        "variablelist" => parse_variable_list(element),
        "informaltable" => parse_table(element),
        name => Err(invalid(format!(
            "unsupported strict DocBook block element <{name}>"
        ))),
    }
}

fn parse_list(element: &XmlElement, ordered: bool) -> Result<Block, ParseError> {
    validate_attrs(element, if ordered { &["startingnumber"] } else { &[] })?;
    let start = element
        .attr("startingnumber")
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| invalid("startingnumber must be a positive integer"))
        })
        .transpose()?
        .unwrap_or(1);
    let mut items = Vec::new();
    for child in element.element_children() {
        if child.name != "listitem" {
            return Err(invalid("lists may only contain <listitem>"));
        }
        validate_attrs(child, &[])?;
        items.push(ListItem {
            content: parse_block_nodes(&child.children)?,
        });
    }
    Ok(if ordered {
        Block::OrderedList { start, items }
    } else {
        Block::UnorderedList { items }
    })
}

fn parse_variable_list(element: &XmlElement) -> Result<Block, ParseError> {
    validate_attrs(element, &[])?;
    let mut items = Vec::new();
    for entry in element.element_children() {
        if entry.name != "varlistentry" {
            return Err(invalid("<variablelist> may only contain <varlistentry>"));
        }
        validate_attrs(entry, &[])?;
        let term = entry
            .element_children()
            .find(|child| child.name == "term")
            .ok_or_else(|| invalid("<varlistentry> requires <term>"))?;
        validate_attrs(term, &[])?;
        let mut definitions = Vec::new();
        for definition in entry
            .element_children()
            .filter(|child| child.name == "listitem")
        {
            validate_attrs(definition, &[])?;
            definitions.push(parse_block_nodes(&definition.children)?);
        }
        if definitions.is_empty() {
            return Err(invalid("<varlistentry> requires <listitem>"));
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
    let tgroup = only_element_child(element, "tgroup")?;
    validate_attrs(tgroup, &["cols"])?;
    let columns =
        positive_attr(tgroup, "cols")?.ok_or_else(|| invalid("<tgroup> requires cols"))? as usize;
    let mut alignments = vec![ColumnAlignment::Default; columns];
    for colspec in tgroup
        .element_children()
        .filter(|child| child.name == "colspec")
    {
        validate_attrs(colspec, &["colname", "align"])?;
        require_empty(colspec)?;
        let index = colspec
            .attr("colname")
            .and_then(|name| name.strip_prefix('c'))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|index| *index > 0 && *index <= columns)
            .ok_or_else(|| invalid("colspec colname must be c1 through cN"))?;
        alignments[index - 1] = parse_alignment(colspec.attr("align"))?;
    }
    let thead = tgroup
        .element_children()
        .find(|child| child.name == "thead")
        .ok_or_else(|| invalid("<tgroup> requires <thead>"))?;
    let tbody = tgroup
        .element_children()
        .find(|child| child.name == "tbody")
        .ok_or_else(|| invalid("<tgroup> requires <tbody>"))?;
    validate_attrs(thead, &[])?;
    validate_attrs(tbody, &[])?;
    let head_rows: Vec<&XmlElement> = thead.element_children().collect();
    if head_rows.len() != 1 || head_rows[0].name != "row" {
        return Err(invalid("<thead> requires exactly one <row>"));
    }
    let headers = parse_cals_row(head_rows[0], columns)?;
    let mut rows = Vec::new();
    for row in tbody.element_children() {
        if row.name != "row" {
            return Err(invalid("<tbody> may only contain <row>"));
        }
        rows.push(parse_cals_row(row, columns)?);
    }
    Ok(Block::Table {
        headers,
        alignments,
        rows,
    })
}

fn parse_cals_row(element: &XmlElement, columns: usize) -> Result<Vec<TableCell>, ParseError> {
    validate_attrs(element, &[])?;
    let mut cells = Vec::new();
    let mut cursor = 1usize;
    for entry in element.element_children() {
        if entry.name != "entry" {
            return Err(invalid("<row> may only contain <entry>"));
        }
        validate_attrs(entry, &["namest", "nameend", "morerows"])?;
        let start = entry
            .attr("namest")
            .map(parse_colname)
            .transpose()?
            .unwrap_or(cursor);
        let end = entry
            .attr("nameend")
            .map(parse_colname)
            .transpose()?
            .unwrap_or(start);
        if start > end || end > columns {
            return Err(invalid("invalid CALS entry span"));
        }
        let rowspan = positive_or_zero_attr(entry, "morerows")?.unwrap_or(0) + 1;
        cells.push(TableCell::with_span(
            parse_inline_nodes(&entry.children)?,
            (end - start + 1) as u32,
            rowspan,
        ));
        cursor = end + 1;
    }
    Ok(cells)
}

fn parse_inline_nodes(nodes: &[XmlNode]) -> Result<Vec<Inline>, ParseError> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            XmlNode::Text(text) => {
                if !text.is_empty() {
                    result.push(Inline::Text(text.clone()));
                }
            }
            XmlNode::Element(element) => result.push(parse_inline(element)?),
        }
    }
    Ok(result)
}

fn parse_inline(element: &XmlElement) -> Result<Inline, ParseError> {
    match element.name.as_str() {
        "emphasis" if element.attr("role") == Some("strong") => {
            validate_attrs(element, &["role"])?;
            Ok(Inline::Bold(parse_inline_nodes(&element.children)?))
        }
        "emphasis" => {
            validate_attrs(element, &[])?;
            Ok(Inline::Italic(parse_inline_nodes(&element.children)?))
        }
        "phrase" if element.attr("role") == Some("bold-italic") => {
            validate_attrs(element, &["role"])?;
            Ok(Inline::BoldItalic(parse_inline_nodes(&element.children)?))
        }
        "phrase" if element.attr("role") == Some("strikethrough") => {
            validate_attrs(element, &["role"])?;
            Ok(Inline::Strikethrough(parse_inline_nodes(
                &element.children,
            )?))
        }
        "phrase" if element.attr("role") == Some("morph-hard-break") => {
            validate_attrs(element, &["role"])?;
            require_empty(element)?;
            Ok(Inline::HardLineBreak)
        }
        "phrase" if element.attr("role") == Some("morph-soft-break") => {
            validate_attrs(element, &["role"])?;
            require_empty(element)?;
            Ok(Inline::SoftLineBreak)
        }
        "phrase" if element.attr("role") == Some("morph-raw") => {
            validate_attrs(element, &["role", "remap"])?;
            Ok(Inline::RawInline {
                format: element.attr("remap").map(str::to_string),
                content: text_only(element)?,
            })
        }
        "superscript" => wrapped_inline(element, Inline::Superscript),
        "subscript" => wrapped_inline(element, Inline::Subscript),
        "code" => {
            validate_attrs(element, &[])?;
            Ok(Inline::InlineCode(text_only(element)?))
        }
        "link" => parse_link(element),
        "inlinemediaobject" => parse_image(element, None),
        name => Err(invalid(format!(
            "unsupported strict DocBook inline element <{name}>"
        ))),
    }
}

fn parse_link(element: &XmlElement) -> Result<Inline, ParseError> {
    validate_attrs(element, &["xlink:href", "xlink:title"])?;
    let url = element
        .attr("xlink:href")
        .ok_or_else(|| invalid("<link> requires xlink:href"))?
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
        title: element.attr("xlink:title").map(str::to_string),
    })
}

fn parse_image(element: &XmlElement, link: Option<String>) -> Result<Inline, ParseError> {
    validate_attrs(element, &[])?;
    let imageobject = element
        .element_children()
        .find(|child| child.name == "imageobject")
        .ok_or_else(|| invalid("<inlinemediaobject> requires <imageobject>"))?;
    validate_attrs(imageobject, &[])?;
    let data = only_element_child(imageobject, "imagedata")?;
    validate_attrs(data, &["fileref", "xlink:title"])?;
    require_empty(data)?;
    let alt = element
        .element_children()
        .find(|child| child.name == "textobject")
        .and_then(|textobject| {
            textobject
                .element_children()
                .find(|child| child.name == "phrase")
        })
        .map(text_only)
        .transpose()?
        .map(|text| vec![Inline::Text(text)])
        .unwrap_or_default();
    Ok(Inline::Image {
        url: data
            .attr("fileref")
            .ok_or_else(|| invalid("<imagedata> requires fileref"))?
            .to_string(),
        alt,
        title: data.attr("xlink:title").map(str::to_string),
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

fn single_root(nodes: &[XmlNode]) -> Result<&XmlElement, ParseError> {
    let mut root = None;
    for node in nodes {
        match node {
            XmlNode::Text(text) if text.trim().is_empty() => {}
            XmlNode::Text(_) => return Err(invalid("text is not allowed outside <article>")),
            XmlNode::Element(element) if root.is_none() => root = Some(element),
            XmlNode::Element(_) => return Err(invalid("DocBook requires a single root element")),
        }
    }
    root.ok_or_else(|| invalid("empty DocBook document"))
}

fn only_element_child<'a>(
    element: &'a XmlElement,
    name: &str,
) -> Result<&'a XmlElement, ParseError> {
    let matches: Vec<&XmlElement> = element
        .element_children()
        .filter(|child| child.name == name)
        .collect();
    if matches.len() != 1 {
        return Err(invalid(format!(
            "<{}> requires exactly one <{name}>",
            element.name
        )));
    }
    Ok(matches[0])
}

fn parse_alignment(value: Option<&str>) -> Result<ColumnAlignment, ParseError> {
    match value {
        Some("left") => Ok(ColumnAlignment::Left),
        Some("center") => Ok(ColumnAlignment::Center),
        Some("right") => Ok(ColumnAlignment::Right),
        Some(value) => Err(invalid(format!("unsupported CALS alignment '{value}'"))),
        None => Ok(ColumnAlignment::Default),
    }
}

fn parse_colname(value: &str) -> Result<usize, ParseError> {
    value
        .strip_prefix('c')
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid("CALS column names must use c1, c2, ..."))
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

fn positive_or_zero_attr(element: &XmlElement, name: &str) -> Result<Option<u32>, ParseError> {
    element
        .attr(name)
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| invalid(format!("{name} must be a non-negative integer")))
        })
        .transpose()
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

fn require_empty(element: &XmlElement) -> Result<(), ParseError> {
    if element.children.iter().any(|child| match child {
        XmlNode::Element(_) => true,
        XmlNode::Text(text) => !text.trim().is_empty(),
    }) {
        return Err(invalid(format!("<{}> must be empty", element.name)));
    }
    Ok(())
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

fn invalid(message: impl Into<String>) -> ParseError {
    ParseError::InvalidInput(message.into())
}
