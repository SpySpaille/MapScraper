use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KVValue {
    String(String),
    Block(Vec<(String, KVValue)>),
}

impl KVValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            KVValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_block(&self) -> Option<&[(String, KVValue)]> {
        match self {
            KVValue::Block(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&KVValue> {
        let key_lower = key.to_lowercase();
        match self {
            KVValue::Block(entries) => {
                for (k, v) in entries {
                    if k.to_lowercase() == key_lower {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_all(&self, key: &str) -> Vec<&KVValue> {
        let key_lower = key.to_lowercase();
        let mut results = Vec::new();
        if let KVValue::Block(entries) = self {
            for (k, v) in entries {
                if k.to_lowercase() == key_lower {
                    results.push(v);
                }
            }
        }
        results
    }

    pub fn get_all_strings(&self, key: &str) -> Vec<&str> {
        self.get_all(key)
            .into_iter()
            .filter_map(|v| v.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KVDocument {
    pub root_entries: Vec<(String, KVValue)>,
}

impl KVDocument {
    pub fn get(&self, key: &str) -> Option<&KVValue> {
        let key_lower = key.to_lowercase();
        for (k, v) in &self.root_entries {
            if k.to_lowercase() == key_lower {
                return Some(v);
            }
        }
        None
    }

    pub fn get_all(&self, key: &str) -> Vec<&KVValue> {
        let key_lower = key.to_lowercase();
        let mut res = Vec::new();
        for (k, v) in &self.root_entries {
            if k.to_lowercase() == key_lower {
                res.push(v);
            }
        }
        res
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    String(String),
    OpenBrace,
    CloseBrace,
}

#[derive(Debug, Clone)]
pub struct KVParseError {
    pub message: String,
    pub line: usize,
}

impl fmt::Display for KVParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KV parse error at line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for KVParseError {}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            line: 1,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, KVParseError> {
        loop {
            match self.chars.peek().copied() {
                None => return Ok(None),
                Some('\n') => {
                    self.line += 1;
                    self.chars.next();
                }
                Some(c) if c.is_whitespace() => {
                    self.chars.next();
                }
                Some('{') => {
                    self.chars.next();
                    return Ok(Some(Token::OpenBrace));
                }
                Some('}') => {
                    self.chars.next();
                    return Ok(Some(Token::CloseBrace));
                }
                Some('/') => {
                    self.chars.next();
                    match self.chars.peek().copied() {
                        Some('/') => {
                            // Single line comment: consume until '\n' or EOF
                            while let Some(&ch) = self.chars.peek() {
                                if ch == '\n' {
                                    break;
                                }
                                self.chars.next();
                            }
                        }
                        Some('*') => {
                            // Block comment: consume until '*/'
                            self.chars.next();
                            loop {
                                match self.chars.next() {
                                    None => break,
                                    Some('\n') => self.line += 1,
                                    Some('*') => {
                                        if let Some('/') = self.chars.peek() {
                                            self.chars.next();
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            // Bare slash as start of unquoted token
                            let mut s = String::from("/");
                            self.read_unquoted(&mut s);
                            return Ok(Some(Token::String(s)));
                        }
                    }
                }
                Some('"') => {
                    self.chars.next();
                    let s = self.read_quoted()?;
                    return Ok(Some(Token::String(s)));
                }
                Some(_) => {
                    let mut s = String::new();
                    self.read_unquoted(&mut s);
                    return Ok(Some(Token::String(s)));
                }
            }
        }
    }

    fn read_quoted(&mut self) -> Result<String, KVParseError> {
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            match c {
                '"' => return Ok(s),
                '\\' => {
                    if let Some(next) = self.chars.next() {
                        match next {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            other => {
                                s.push('\\');
                                s.push(other);
                            }
                        }
                    }
                }
                '\n' => {
                    self.line += 1;
                    s.push('\n');
                }
                other => s.push(other),
            }
        }
        Err(KVParseError {
            message: "Unterminated double quoted string".to_string(),
            line: self.line,
        })
    }

    fn read_unquoted(&mut self, s: &mut String) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                break;
            }
            if c == '/' {
                break;
            }
            s.push(c);
            self.chars.next();
        }
    }
}

pub fn parse_keyvalues(input: &str) -> Result<KVDocument, KVParseError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token()? {
        tokens.push(token);
    }

    let mut cursor = 0;
    let mut root_entries = Vec::new();

    while cursor < tokens.len() {
        if let Some((key, val)) = parse_entry(&tokens, &mut cursor)? {
            root_entries.push((key, val));
        } else {
            break;
        }
    }

    Ok(KVDocument { root_entries })
}

fn parse_entry(
    tokens: &[Token],
    cursor: &mut usize,
) -> Result<Option<(String, KVValue)>, KVParseError> {
    if *cursor >= tokens.len() {
        return Ok(None);
    }

    let key = match &tokens[*cursor] {
        Token::String(s) => {
            *cursor += 1;
            s.clone()
        }
        Token::CloseBrace => {
            return Ok(None);
        }
        Token::OpenBrace => {
            return Err(KVParseError {
                message: "Unexpected opening brace '{' without a key".to_string(),
                line: 0,
            });
        }
    };

    if *cursor >= tokens.len() {
        return Err(KVParseError {
            message: format!("Key '{}' has no value or block", key),
            line: 0,
        });
    }

    match &tokens[*cursor] {
        Token::OpenBrace => {
            *cursor += 1; // Consume '{'
            let mut block_entries = Vec::new();
            while *cursor < tokens.len() {
                if let Token::CloseBrace = &tokens[*cursor] {
                    *cursor += 1; // Consume '}'
                    return Ok(Some((key, KVValue::Block(block_entries))));
                }
                if let Some(entry) = parse_entry(tokens, cursor)? {
                    block_entries.push(entry);
                } else if *cursor < tokens.len() && tokens[*cursor] == Token::CloseBrace {
                    *cursor += 1;
                    return Ok(Some((key, KVValue::Block(block_entries))));
                } else {
                    break;
                }
            }
            Ok(Some((key, KVValue::Block(block_entries))))
        }
        Token::String(val) => {
            let val_str = val.clone();
            *cursor += 1;
            Ok(Some((key, KVValue::String(val_str))))
        }
        Token::CloseBrace => Err(KVParseError {
            message: format!("Key '{}' unexpectedly followed by closing brace", key),
            line: 0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_comments_in_quotes() {
        let kv = r#"
            "VertexLitGeneric"
            {
                "$basetexture" "models/props/crate"
                "$description" "This is // not a comment inside string"
                // This is a real line comment
                "$bumpmap" "models/props/crate_normal"
            }
        "#;
        let doc = parse_keyvalues(kv).unwrap();
        let root = doc.get("VertexLitGeneric").unwrap();
        assert_eq!(
            root.get_string("$description").unwrap(),
            "This is // not a comment inside string"
        );
        assert_eq!(
            root.get_string("$basetexture").unwrap(),
            "models/props/crate"
        );
        assert_eq!(
            root.get_string("$bumpmap").unwrap(),
            "models/props/crate_normal"
        );
    }

    #[test]
    fn test_kv_nested_blocks() {
        let vmf = r#"
            world
            {
                "id" "1"
                "classname" "worldspawn"
                solid
                {
                    "id" "2"
                    side
                    {
                        "material" "brick/brickwall01"
                    }
                }
            }
            entity
            {
                "id" "3"
                "classname" "func_instance"
                editor
                {
                    "color" "220 30 220"
                }
                "file" "instances/door.vmf"
                "targetname" "door_inst"
            }
        "#;
        let doc = parse_keyvalues(vmf).unwrap();
        let entities = doc.get_all("entity");
        assert_eq!(entities.len(), 1);
        let ent = entities[0];
        assert_eq!(ent.get_string("classname").unwrap(), "func_instance");
        assert_eq!(ent.get_string("file").unwrap(), "instances/door.vmf");
        assert_eq!(ent.get_string("targetname").unwrap(), "door_inst");
    }
}
