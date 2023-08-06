#[derive(Debug, PartialEq, Clone)]
pub enum SseResponse {
    Event(String),
    Data(String),
}

impl SseResponse {
    pub fn from_line(line: &str) -> Self {
        if line.starts_with("data:") {
            return Self::Data(line.replace("data:", "").trim().to_string());
        }
        if line.starts_with("event:") {
            return Self::Event(line.replace("event:", "").trim().to_string());
        }
        Self::Event(line[6..].to_string().trim_end().to_string())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn sse_のdataの場合() {
        let sse_data = "data: hello world\n\n";

        let sut = SseResponse::from_line(sse_data);

        assert_eq!(SseResponse::Data("hello world".to_string()), sut);
    }
    #[test]
    fn sse_のeventの場合() {
        let sse_data = "event: hello world\n\n";

        let sut = SseResponse::from_line(sse_data);

        assert_eq!(SseResponse::Event("hello world".to_string()), sut);
    }
}
