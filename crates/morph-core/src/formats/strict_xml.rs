use crate::error::ParseError;

#[derive(Debug, Clone)]
pub(crate) enum XmlNode {
    Element(XmlElement),
    Text(String),
}

#[derive(Debug, Clone)]
pub(crate) struct XmlElement {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<XmlNode>,
}

impl XmlElement {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }

    pub fn element_children(&self) -> impl Iterator<Item = &XmlElement> {
        self.children.iter().filter_map(|node| match node {
            XmlNode::Element(element) => Some(element),
            XmlNode::Text(_) => None,
        })
    }
}

pub(crate) fn parse_xml(input: &str, html_void_elements: bool) -> Result<Vec<XmlNode>, ParseError> {
    let mut parser = XmlParser {
        input,
        pos: 0,
        html_void_elements,
    };
    parser.parse_nodes(None)
}

struct XmlParser<'a> {
    input: &'a str,
    pos: usize,
    html_void_elements: bool,
}

impl XmlParser<'_> {
    fn parse_nodes(&mut self, closing: Option<&str>) -> Result<Vec<XmlNode>, ParseError> {
        let mut nodes = Vec::new();
        loop {
            if self.pos >= self.input.len() {
                if let Some(tag) = closing {
                    return Err(self.error(format!("unterminated <{tag}> element")));
                }
                break;
            }
            if self.rest().starts_with("<!--") {
                self.skip_until("-->", "unterminated comment")?;
                continue;
            }
            if self.rest().starts_with("<?") {
                self.skip_until("?>", "unterminated processing instruction")?;
                continue;
            }
            if self.rest().starts_with("<!DOCTYPE") || self.rest().starts_with("<!doctype") {
                self.skip_until(">", "unterminated doctype")?;
                continue;
            }
            if self.rest().starts_with("</") {
                let end_name = self.parse_end_tag()?;
                match closing {
                    Some(expected) if end_name == expected => break,
                    Some(expected) => {
                        return Err(
                            self.error(format!("expected </{expected}>, found </{end_name}>"))
                        );
                    }
                    None => return Err(self.error(format!("unexpected </{end_name}>"))),
                }
            }
            if self.rest().starts_with('<') {
                nodes.push(XmlNode::Element(self.parse_element()?));
            } else {
                let end = self.rest().find('<').unwrap_or(self.rest().len()) + self.pos;
                let text = decode_entities(&self.input[self.pos..end])
                    .map_err(|message| self.error(message))?;
                self.pos = end;
                if !text.is_empty() {
                    nodes.push(XmlNode::Text(text));
                }
            }
        }
        Ok(nodes)
    }

    fn parse_element(&mut self) -> Result<XmlElement, ParseError> {
        self.expect("<")?;
        let name = self.parse_name()?;
        let mut attributes = Vec::new();
        loop {
            self.skip_whitespace();
            if self.rest().starts_with("/>") {
                self.pos += 2;
                return Ok(XmlElement {
                    name,
                    attributes,
                    children: Vec::new(),
                });
            }
            if self.rest().starts_with('>') {
                self.pos += 1;
                break;
            }
            let key = self.parse_name()?;
            if attributes.iter().any(|(existing, _)| existing == &key) {
                return Err(self.error(format!("duplicate '{key}' attribute")));
            }
            self.skip_whitespace();
            self.expect("=")?;
            self.skip_whitespace();
            let quote = self
                .rest()
                .chars()
                .next()
                .filter(|character| *character == '"' || *character == '\'')
                .ok_or_else(|| self.error("attribute values must be quoted".to_string()))?;
            self.pos += quote.len_utf8();
            let end = self
                .rest()
                .find(quote)
                .ok_or_else(|| self.error("unterminated attribute value".to_string()))?;
            let value =
                decode_entities(&self.rest()[..end]).map_err(|message| self.error(message))?;
            self.pos += end + quote.len_utf8();
            attributes.push((key, value));
        }

        if self.html_void_elements && matches!(name.as_str(), "br" | "hr" | "img" | "meta" | "link")
        {
            return Ok(XmlElement {
                name,
                attributes,
                children: Vec::new(),
            });
        }
        let children = self.parse_nodes(Some(&name))?;
        Ok(XmlElement {
            name,
            attributes,
            children,
        })
    }

    fn parse_end_tag(&mut self) -> Result<String, ParseError> {
        self.expect("</")?;
        let name = self.parse_name()?;
        self.skip_whitespace();
        self.expect(">")?;
        Ok(name)
    }

    fn parse_name(&mut self) -> Result<String, ParseError> {
        let length = self
            .rest()
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
            })
            .map(char::len_utf8)
            .sum::<usize>();
        if length == 0 {
            return Err(self.error("expected an element or attribute name".to_string()));
        }
        let name = self.rest()[..length].to_string();
        self.pos += length;
        Ok(name)
    }

    fn skip_whitespace(&mut self) {
        let count = self
            .rest()
            .chars()
            .take_while(|character| character.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
        self.pos += count;
    }

    fn skip_until(&mut self, delimiter: &str, message: &str) -> Result<(), ParseError> {
        let end = self
            .rest()
            .find(delimiter)
            .ok_or_else(|| self.error(message.to_string()))?;
        self.pos += end + delimiter.len();
        Ok(())
    }

    fn expect(&mut self, value: &str) -> Result<(), ParseError> {
        if !self.rest().starts_with(value) {
            return Err(self.error(format!("expected '{value}'")));
        }
        self.pos += value.len();
        Ok(())
    }

    fn rest(&self) -> &str {
        &self.input[self.pos..]
    }

    fn error(&self, message: String) -> ParseError {
        ParseError::InvalidInput(format!("{message} at byte {}", self.pos))
    }
}

fn decode_entities(input: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut pos = 0;
    while let Some(offset) = input[pos..].find('&') {
        let start = pos + offset;
        output.push_str(&input[pos..start]);
        let end = input[start..]
            .find(';')
            .map(|value| start + value)
            .ok_or_else(|| "unterminated entity".to_string())?;
        let entity = &input[start + 1..end];
        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ if entity.starts_with("#x") => {
                let value = u32::from_str_radix(&entity[2..], 16)
                    .map_err(|_| format!("invalid entity '&{entity};'"))?;
                char::from_u32(value).ok_or_else(|| format!("invalid entity '&{entity};'"))?
            }
            _ if entity.starts_with('#') => {
                let value = entity[1..]
                    .parse::<u32>()
                    .map_err(|_| format!("invalid entity '&{entity};'"))?;
                char::from_u32(value).ok_or_else(|| format!("invalid entity '&{entity};'"))?
            }
            _ => return Err(format!("unsupported entity '&{entity};'")),
        };
        output.push(character);
        pos = end + 1;
    }
    output.push_str(&input[pos..]);
    Ok(output)
}

pub(crate) fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn escape_xml_attr(input: &str) -> String {
    escape_xml(input)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
