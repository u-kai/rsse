#[derive(Debug, PartialEq, Clone)]
pub enum SseResponse {
    Event(String),
    Data(String),
}

impl SseResponse {
    pub fn from_line(line: &str) -> Self {
        if line.starts_with("data:") {
            return Self::Data(Self::trim(line, "data:"));
        }
        if line.starts_with("event:") {
            return Self::Event(Self::trim(line, "event:"));
        }
        Self::Event(line[6..].to_string().trim_end().to_string())
    }
    fn trim(line: &str, res_type: &str) -> String {
        line.replace(res_type, "").trim().to_string()
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
