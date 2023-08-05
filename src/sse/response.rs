use crate::http::HttpStatusLine;

#[derive(Debug, PartialEq, Clone)]
pub enum SseResponse {
    Event(String),
    HttpStatusLine(HttpStatusLine),
}

impl SseResponse {
    pub fn from_line(line: &str) -> Self {
        if let Ok(status_line) = HttpStatusLine::from_str(line) {
            return Self::HttpStatusLine(status_line);
        }
        Self::Event(line[6..].to_string().trim_end().to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{HttpStatusCode, HttpStatusLine, HttpVersion};

    use super::*;
    #[test]
    fn http_status_lineの場合() {
        let ok_data = "HTTP/1.1 200 OK\n";

        let sut = SseResponse::from_line(ok_data);

        let expected = HttpStatusLine::new(HttpVersion::V1_1, HttpStatusCode::OK);
        assert_eq!(SseResponse::HttpStatusLine(expected), sut);
    }
    #[test]
    fn sse_のデータの場合() {
        let sse_data = "data: hello world\n\n";

        let sut = SseResponse::from_line(sse_data);

        assert_eq!(SseResponse::Event("hello world".to_string()), sut);
    }
}
