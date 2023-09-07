use std::{collections::HashMap, fmt::Display};

#[derive(Debug, PartialEq, Clone)]
pub struct HttpHeader {
    headers: HashMap<String, String>,
}

impl HttpHeader {
    pub fn new() -> Self {
        HttpHeader {
            headers: HashMap::new(),
        }
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|v| v.as_str())
    }
    pub fn insert(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }
    pub fn concat(&mut self, other: Self) {
        self.headers.extend(other.headers);
    }
    pub fn from_line(line: &str) -> Result<Self, HttpHeaderError> {
        let mut headers = HashMap::new();
        let mut iter = line.splitn(2, ":");
        let key = iter.next().ok_or(HttpHeaderError::InvalidFormat(format!(
            "Invalid format: {}",
            line,
        )))?;
        let value = iter.next().ok_or(HttpHeaderError::InvalidFormat(format!(
            "Invalid format: {}",
            line,
        )))?;
        headers.insert(key.trim().to_string(), value.trim().to_string());
        Ok(HttpHeader { headers })
    }
    pub fn to_string(&self) -> String {
        self.headers.iter().fold(String::new(), |acc, (k, v)| {
            format!("{}{}: {}\r\n", acc, k, v)
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum HttpHeaderError {
    InvalidFormat(String),
}
impl Display for HttpHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpHeaderError::InvalidFormat(s) => write!(f, "Invalid format: {}", s),
        }
    }
}
impl std::error::Error for HttpHeaderError {}
#[cfg(test)]
mod header_tests {
    use super::*;
    #[test]
    fn headerの文字列から構造体を生成可能() {
        let header = "Content-Type: text/html";
        let sut = HttpHeader::from_line(header).unwrap();

        assert_eq!(sut.get("Content-Type").unwrap(), "text/html");
        assert_eq!(sut.get("Set-Cookie"), None);
        assert_eq!(sut.to_string(), format!("{}{}", header, "\r\n"));
    }
    #[test]
    fn 不正な文字列の場合はエラー() {
        let status_line = "HTTP/1.1 200 OK\n\r";
        let sut = HttpHeader::from_line(status_line);

        assert!(sut.is_err());
    }
    #[test]
    fn 改行文字含む文字列に変換可能() {
        let header = "Content-Type: text/html";
        let sut = HttpHeader::from_line(header).unwrap();

        assert_eq!(sut.to_string(), format!("{}{}", header, "\r\n"));
    }
    #[test]
    fn header同士で結合可能() {
        let mut header = HttpHeader::new();

        let other = HttpHeader::from_line("Content-Type: text/html").unwrap();
        header.concat(other);

        assert_eq!(header.get("Content-Type").unwrap(), "text/html");
    }
}
