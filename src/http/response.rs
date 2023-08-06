use super::{body::HttpBody, header::HttpHeader, status_line::HttpStatusLine};

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    status_line: HttpStatusLine,
    header: HttpHeader,
    body: HttpBody,
}

impl HttpResponse {
    pub fn new(status_line: HttpStatusLine, header: HttpHeader, body: HttpBody) -> Self {
        HttpResponse {
            status_line,
            header,
            body,
        }
    }
    pub fn status_code(&self) -> u32 {
        self.status_line.status_code().num()
    }
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.header.get(key)
    }
    pub fn body_str(&self) -> &str {
        self.body.to_str()
    }
    pub fn is_error(&self) -> bool {
        self.status_code() >= 400
    }
    pub fn to_string(&self) -> String {
        format!(
            "{}{}{}{}{}",
            self.status_line.to_string(),
            self.header.to_string(),
            "\r\n",
            self.body.to_str(),
            "\r\n"
        )
    }
}

#[cfg(test)]

mod tests {
    use crate::http::{
        body::HttpBody, header::HttpHeader, response::HttpResponse, status_line::HttpStatusLine,
    };

    #[test]
    fn errorかどうか判断できる() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 404 Not Found").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header, body);

        assert!(sut.is_error());
    }
    #[test]
    fn bodyの文字列を返すことができる() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 200 OK").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header, body);

        assert_eq!(sut.body_str(), "Hello, World!",);
    }
    #[test]
    #[allow(non_snake_case)]
    fn 存在しないheaderはNoneを返す() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 200 OK").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header, body);

        assert_eq!(sut.get_header("Content-Length"), None);
    }
    #[test]
    fn headerの要素を返すことができる() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 200 OK").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header.clone(), body);

        assert_eq!(sut.get_header("Content-Type"), Some("text/event-stream"));
    }
    #[test]
    fn ステータスコードを返すことができる() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 200 OK").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header, body);

        assert_eq!(sut.status_code(), 200);
    }
    #[test]
    fn 文字列に変換可能() {
        let status_line = HttpStatusLine::from_str("HTTP/1.1 200 OK").unwrap();
        let header = HttpHeader::from_line("Content-Type: text/event-stream").unwrap();
        let body = HttpBody::from_line("Hello, World!");

        let sut = HttpResponse::new(status_line, header, body);

        assert_eq!(
            sut.to_string(),
            format!(
                "{}{}{}{}{}",
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "\r\n",
                "Hello, World!",
                "\r\n"
            )
        );
    }
}
