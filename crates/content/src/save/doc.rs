//! A lossless tree view of a RON document, used by save migrations.
//!
//! `ron::Value` drops enum variant names (`Province(3)` becomes `[3]`), so it
//! cannot round-trip a save. This tree keeps names and leaves literals as the
//! exact source text, so `parse` followed by `to_ron` changes nothing that a
//! migration did not touch.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Number, string, char, `true`/`false` or a bare identifier (`None`, a
    /// unit variant), kept verbatim.
    Atom(String),
    /// `(field: value, ...)`, optionally named: `Name(field: value)`.
    Struct {
        name: Option<String>,
        fields: Vec<(String, Node)>,
    },
    /// `(a, b)`, `Some(x)`, `Province(3)`, `Variant()`.
    Tuple {
        name: Option<String>,
        items: Vec<Node>,
    },
    List(Vec<Node>),
    Map(Vec<(Node, Node)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for DocError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} (byte {})", self.message, self.offset)
    }
}

impl std::error::Error for DocError {}

impl Node {
    pub fn atom(text: impl Into<String>) -> Self {
        Node::Atom(text.into())
    }

    pub fn field(&self, name: &str) -> Option<&Node> {
        match self {
            Node::Struct { fields, .. } => fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, node)| node),
            _ => None,
        }
    }

    pub fn field_mut(&mut self, name: &str) -> Option<&mut Node> {
        match self {
            Node::Struct { fields, .. } => fields
                .iter_mut()
                .find(|(field, _)| field == name)
                .map(|(_, node)| node),
            _ => None,
        }
    }

    /// Replaces the field if present, otherwise appends it. Returns `false` if
    /// this node is not a struct.
    pub fn set_field(&mut self, name: &str, value: Node) -> bool {
        let Node::Struct { fields, .. } = self else {
            return false;
        };
        match fields.iter_mut().find(|(field, _)| field == name) {
            Some((_, node)) => *node = value,
            None => fields.push((name.to_string(), value)),
        }
        true
    }

    pub fn remove_field(&mut self, name: &str) -> Option<Node> {
        let Node::Struct { fields, .. } = self else {
            return None;
        };
        let index = fields.iter().position(|(field, _)| field == name)?;
        Some(fields.remove(index).1)
    }

    pub fn rename_field(&mut self, from: &str, to: &str) -> bool {
        let Node::Struct { fields, .. } = self else {
            return false;
        };
        match fields.iter_mut().find(|(field, _)| field == from) {
            Some((field, _)) => {
                *field = to.to_string();
                true
            }
            None => false,
        }
    }

    pub fn items_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::List(items) | Node::Tuple { items, .. } => Some(items),
            _ => None,
        }
    }

    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Node::Atom(text) => Some(text),
            _ => None,
        }
    }

    pub fn to_ron(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Node::Atom(text) => out.push_str(text),
            Node::Struct { name, fields } => {
                if let Some(name) = name {
                    out.push_str(name);
                }
                out.push('(');
                for (index, (field, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(field);
                    out.push(':');
                    value.write(out);
                }
                out.push(')');
            }
            Node::Tuple { name, items } => {
                if let Some(name) = name {
                    out.push_str(name);
                }
                out.push('(');
                write_separated(out, items);
                out.push(')');
            }
            Node::List(items) => {
                out.push('[');
                write_separated(out, items);
                out.push(']');
            }
            Node::Map(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    key.write(out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_separated(out: &mut String, items: &[Node]) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        item.write(out);
    }
}

pub fn parse(source: &str) -> Result<Node, DocError> {
    let mut parser = Parser {
        bytes: source.as_bytes(),
        source,
        pos: 0,
    };
    let node = parser.value()?;
    parser.skip_trivia()?;
    if parser.pos != parser.bytes.len() {
        return Err(parser.error("conteúdo extra depois do documento"));
    }
    Ok(node)
}

struct Parser<'a> {
    bytes: &'a [u8],
    source: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, message: &'static str) -> DocError {
        DocError {
            offset: self.pos,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_trivia(&mut self) -> Result<(), DocError> {
        loop {
            match (self.peek(), self.bytes.get(self.pos + 1).copied()) {
                (Some(byte), _) if byte.is_ascii_whitespace() => self.pos += 1,
                (Some(b'/'), Some(b'/')) => {
                    while self.peek().is_some_and(|byte| byte != b'\n') {
                        self.pos += 1;
                    }
                }
                (Some(b'/'), Some(b'*')) => {
                    self.pos += 2;
                    loop {
                        match (self.peek(), self.bytes.get(self.pos + 1).copied()) {
                            (Some(b'*'), Some(b'/')) => {
                                self.pos += 2;
                                break;
                            }
                            (Some(_), _) => self.pos += 1,
                            (None, _) => return Err(self.error("comentário sem fim")),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn expect(&mut self, byte: u8, message: &'static str) -> Result<(), DocError> {
        self.skip_trivia()?;
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn value(&mut self) -> Result<Node, DocError> {
        self.skip_trivia()?;
        match self.peek() {
            Some(b'(') => self.parenthesized(None),
            Some(b'[') => self.list(),
            Some(b'{') => self.map(),
            Some(b'"') => self.quoted(b'"'),
            Some(b'\'') => self.quoted(b'\''),
            Some(b'r') if matches!(self.bytes.get(self.pos + 1), Some(b'"' | b'#')) => {
                self.raw_string()
            }
            Some(byte) if byte.is_ascii_digit() || byte == b'-' || byte == b'+' => {
                Ok(self.number())
            }
            Some(byte) if is_ident_start(byte) => {
                let name = self.identifier();
                self.skip_trivia()?;
                if self.peek() == Some(b'(') {
                    self.parenthesized(Some(name))
                } else {
                    Ok(Node::Atom(name))
                }
            }
            Some(_) => Err(self.error("valor inesperado")),
            None => Err(self.error("documento terminou cedo")),
        }
    }

    fn identifier(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.pos += 1;
        }
        self.source[start..self.pos].to_string()
    }

    fn number(&mut self) -> Node {
        let start = self.pos;
        self.pos += 1;
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
        {
            self.pos += 1;
        }
        Node::Atom(self.source[start..self.pos].to_string())
    }

    fn quoted(&mut self, quote: u8) -> Result<Node, DocError> {
        let start = self.pos;
        self.pos += 1;
        loop {
            match self.peek() {
                Some(b'\\') => self.pos += 2,
                Some(byte) if byte == quote => {
                    self.pos += 1;
                    return Ok(Node::Atom(self.source[start..self.pos].to_string()));
                }
                Some(_) => self.pos += 1,
                None => return Err(self.error("texto sem aspas de fechamento")),
            }
        }
    }

    fn raw_string(&mut self) -> Result<Node, DocError> {
        let start = self.pos;
        self.pos += 1;
        let mut hashes = 0;
        while self.peek() == Some(b'#') {
            hashes += 1;
            self.pos += 1;
        }
        if self.peek() != Some(b'"') {
            return Err(self.error("texto bruto malformado"));
        }
        self.pos += 1;
        loop {
            match self.peek() {
                Some(b'"')
                    if self.bytes[self.pos + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|&&byte| byte == b'#')
                        .count()
                        == hashes =>
                {
                    self.pos += 1 + hashes;
                    return Ok(Node::Atom(self.source[start..self.pos].to_string()));
                }
                Some(_) => self.pos += 1,
                None => return Err(self.error("texto bruto sem fechamento")),
            }
        }
    }

    /// Decides between struct and tuple by looking for `identifier:` (but not
    /// `identifier::`) right after the opening parenthesis.
    fn parenthesized(&mut self, name: Option<String>) -> Result<Node, DocError> {
        self.expect(b'(', "esperava '('")?;
        self.skip_trivia()?;
        if self.looks_like_field() {
            let mut fields = Vec::new();
            loop {
                self.skip_trivia()?;
                if self.peek() == Some(b')') {
                    self.pos += 1;
                    return Ok(Node::Struct { name, fields });
                }
                if !self.peek().is_some_and(is_ident_start) {
                    return Err(self.error("esperava nome de campo"));
                }
                let field = self.identifier();
                self.expect(b':', "esperava ':' depois do campo")?;
                fields.push((field, self.value()?));
                if !self.comma_or(b')')? {
                    self.pos += 1;
                    return Ok(Node::Struct { name, fields });
                }
            }
        }

        let mut items = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek() == Some(b')') {
                self.pos += 1;
                return Ok(Node::Tuple { name, items });
            }
            items.push(self.value()?);
            if !self.comma_or(b')')? {
                self.pos += 1;
                return Ok(Node::Tuple { name, items });
            }
        }
    }

    fn looks_like_field(&self) -> bool {
        let mut cursor = self.pos;
        if !self.bytes.get(cursor).copied().is_some_and(is_ident_start) {
            return false;
        }
        while self
            .bytes
            .get(cursor)
            .copied()
            .is_some_and(is_ident_continue)
        {
            cursor += 1;
        }
        while self
            .bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        self.bytes.get(cursor) == Some(&b':') && self.bytes.get(cursor + 1) != Some(&b':')
    }

    /// After an element: consumes a comma and returns `true`, or finds the
    /// closing byte (without consuming it) and returns `false`.
    fn comma_or(&mut self, close: u8) -> Result<bool, DocError> {
        self.skip_trivia()?;
        match self.peek() {
            Some(b',') => {
                self.pos += 1;
                Ok(true)
            }
            Some(byte) if byte == close => Ok(false),
            _ => Err(self.error("esperava ',' ou fechamento")),
        }
    }

    fn list(&mut self) -> Result<Node, DocError> {
        self.expect(b'[', "esperava '['")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(Node::List(items));
            }
            items.push(self.value()?);
            if !self.comma_or(b']')? {
                self.pos += 1;
                return Ok(Node::List(items));
            }
        }
    }

    fn map(&mut self) -> Result<Node, DocError> {
        self.expect(b'{', "esperava '{'")?;
        let mut entries = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(Node::Map(entries));
            }
            let key = self.value()?;
            self.expect(b':', "esperava ':' no mapa")?;
            entries.push((key, self.value()?));
            if !self.comma_or(b'}')? {
                self.pos += 1;
                return Ok(Node::Map(entries));
            }
        }
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_enum_variants_and_literals() {
        let source = r#"(a: Province(3), b: Some("olá, \"mundo\""), c: Faction, d: -12, e: 'x')"#;
        let node = parse(source).unwrap();
        assert_eq!(
            node.field("a"),
            Some(&Node::Tuple {
                name: Some("Province".to_string()),
                items: vec![Node::atom("3")],
            })
        );
        assert_eq!(node.field("c"), Some(&Node::atom("Faction")));
        assert_eq!(
            node.to_ron(),
            r#"(a:Province(3),b:Some("olá, \"mundo\""),c:Faction,d:-12,e:'x')"#
        );
    }

    #[test]
    fn handles_named_structs_lists_maps_comments_and_trailing_commas() {
        let source = r#"
            // comentário
            Save( /* bloco */ items: [1, 2,], map: {"k": (x: 1,)}, empty: (), unit: Variant(), )
        "#;
        let node = parse(source).unwrap();
        assert_eq!(
            node.to_ron(),
            r#"Save(items:[1,2],map:{"k":(x:1)},empty:(),unit:Variant())"#
        );
    }

    #[test]
    fn reparsing_output_is_stable() {
        let source = r##"(raw: r#"a "quoted" text"#, nested: [Some(None), (1, (2, 3))])"##;
        let once = parse(source).unwrap().to_ron();
        assert_eq!(parse(&once).unwrap().to_ron(), once);
    }

    #[test]
    fn edits_fields() {
        let mut node = parse("(a: 1, b: 2)").unwrap();
        assert!(node.rename_field("a", "alpha"));
        assert_eq!(node.remove_field("b"), Some(Node::atom("2")));
        assert!(node.set_field("c", Node::List(vec![])));
        assert!(node.set_field("alpha", Node::atom("5")));
        assert_eq!(node.to_ron(), "(alpha:5,c:[])");
    }

    #[test]
    fn rejects_malformed_documents() {
        for source in [
            "(a: 1",
            "(a 1)",
            "[1 2]",
            "\"sem fim",
            "(a: 1) extra",
            "/* aberto",
        ] {
            assert!(parse(source).is_err(), "deveria rejeitar {source:?}");
        }
    }
}
