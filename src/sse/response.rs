use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub enum SseResponse {
    Event(String),
    Data(String),
    Id(String),
    Retry(u32),
}

impl SseResponse {
    pub fn from_line(line: &str) -> Result<Self, SseResponseError> {
        if line.starts_with("data:") {
            return Ok(Self::Data(Self::trim(line, "data:")));
        }
        if line.starts_with("event:") {
            return Ok(Self::Event(Self::trim(line, "event:")));
        }
        if line.starts_with("id:") {
            return Ok(Self::Id(Self::trim(line, "id:")));
        }
        if line.starts_with("retry:") {
            let Ok(retry) = 
                Self::trim(line, "retry:")
                    .parse::<u32>() else {
                return Err(SseResponseError::InvalidRetry(format!("Invalid retry : {}", line)))
                    };
            return Ok(Self::Retry(retry));
        }
        Err(SseResponseError::InvalidFormat(format!(
            "Invalid format: {}",
            line
        )))
    }
    fn trim(line: &str, res_type: &str) -> String {
        line.replace(res_type, "").trim().to_string()
    }
}

#[derive(Debug, PartialEq)]
pub enum SseResponseError {
    InvalidFormat(String),
    InvalidRetry(String)
}
impl Display for SseResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(message) => write!(f, "InvalidFormat: {}", message),
            Self::InvalidRetry(message) => write!(f, "InvalidRetry: {}", message),
        }
    }
}
impl std::error::Error for SseResponseError {}
#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn sseのdataの場合() {
        let sse_data = "data: hello world\n\n";

        let sut = SseResponse::from_line(sse_data).unwrap();

        assert_eq!(SseResponse::Data("hello world".to_string()), sut);
    }
    #[test]
    fn sseのeventの場合() {
        let sse_data = "event: hello world\n\n";

        let sut = SseResponse::from_line(sse_data).unwrap();

        assert_eq!(SseResponse::Event("hello world".to_string()), sut);
    }
    #[test]
    fn sseのidの場合() {
        let sse_data = "id: hello world\n\n";

        let sut = SseResponse::from_line(sse_data).unwrap();

        assert_eq!(SseResponse::Id("hello world".to_string()), sut);
    }
    #[test]
    fn sseのretryの場合() {
        let retry = 111111111;
        let sse_data = format!("retry: {}\n\n", retry);

        let sut = SseResponse::from_line(sse_data.as_str()).unwrap();

        assert_eq!(SseResponse::Retry(retry), sut);
    }
    #[test]
    fn sse以外のデータの場合はエラーを返す() {
        let sse_data = "hello world\n\n";

        let sut = SseResponse::from_line(sse_data);

        assert!(sut.is_err());
    }
    #[test]
    fn sseのretry値が整数ではない場合はエラーを返す() {
        let invalid_retry = "retry: hello world\n\n";

        let sut = SseResponse::from_line(invalid_retry);

        assert!(sut.is_err());
    }
}
