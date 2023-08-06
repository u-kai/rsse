#[derive(Debug, Clone, PartialEq)]
pub struct HttpBody {
    body: String,
}

impl HttpBody {
    pub fn from_line(line: &str) -> Self {
        HttpBody {
            body: line.to_string(),
        }
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
}
