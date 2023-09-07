#[derive(Debug, Clone, PartialEq)]
pub struct HttpBody {
    body: String,
}

impl HttpBody {
    pub fn new() -> Self {
        HttpBody {
            body: String::new(),
        }
    }
    pub fn from_line(line: &str) -> Self {
        HttpBody {
            body: line.to_string(),
        }
    }
    pub fn concat(&mut self, other: Self) {
        self.body.push_str(other.to_str());
    }
    pub fn to_str(&self) -> &str {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn http_bodyの文字列から構造体を生成可能() {
        let body = "Hello, World!";
        let sut = HttpBody::from_line(body);
        assert_eq!(
            sut,
            HttpBody {
                body: body.to_string()
            }
        );
    }
    #[test]
    fn http_bodyは結合可能() {
        let body = "Hello, World!";
        let mut sut = HttpBody::from_line(body);
        let other = HttpBody::from_line("Good Bye World");

        sut.concat(other);

        assert_eq!(sut.to_str(), "Hello, World!Good Bye World");
    }
}
