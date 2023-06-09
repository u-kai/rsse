use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub enum SseResponseError {
    InvalidLine(SseResponse, String),
    InvalidStartLine(String),
    InvalidHeader(String),
    InvalidStatusCode(u32),
}
impl SseResponseError {
    pub fn invalid_start_line(s: &str) -> Self {
        Self::InvalidStartLine(format!("Invalid start line: {}", s))
    }
    pub fn invalid_status_code(code: u32) -> Self {
        Self::InvalidStatusCode(code)
    }
    pub fn invalid_header(s: &str) -> Self {
        Self::InvalidHeader(format!("Invalid header: {}", s))
    }
    pub fn invalid_line(response: SseResponse, line: &str) -> Self {
        Self::InvalidLine(response, line.to_string())
    }
}
impl Display for SseResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SseResponseError::InvalidStartLine(s) => {
                write!(f, "Invalid start line: {}", s)
            }
            SseResponseError::InvalidStatusCode(code) => {
                write!(f, "Invalid status code: {}", code)
            }
            Self::InvalidHeader(s) => write!(f, "Invalid header: {}", s),
            Self::InvalidLine(state, line) => {
                write!(f, "Invalid line: {} in response: {:?}", line, state)
            }
        }
    }
}
impl std::error::Error for SseResponseError {}

pub type Result<T> = std::result::Result<T, SseResponseError>;

pub trait FromLine {
    fn from_line(line: &str) -> Result<Self>
    where
        Self: Sized;
}
#[derive(Debug)]
pub struct SseResponseStore {
    response: Option<SseResponse>,
}

impl SseResponseStore {
    pub fn new() -> Self {
        Self { response: None }
    }
    pub fn evaluate_lines(&mut self, lines: &str) -> Result<&SseResponse> {
        if lines.lines().count() == 0 {
            return Err(SseResponseError::InvalidLine(
                self.response.as_ref().unwrap().clone(),
                lines.to_string(),
            ));
        }
        for line in lines.lines() {
            self.evaluate(line)?;
        }
        Ok(self.response.as_ref().unwrap())
    }
    pub fn evaluate(&mut self, line: &str) -> Result<&SseResponse> {
        let Some(response) = self.response.take() else {
            let response = SseResponse::from_line(line)?;
            self.response = Some(response);
            return Ok(self.response.as_ref().unwrap());
        };
        let response = response.add_line(line)?;
        self.response = Some(response);
        Ok(self.response.as_ref().unwrap())
    }
    #[allow(dead_code)]
    pub fn add_response(mut self, line: &str) -> Result<Self> {
        let Some(response) = self.response else {
            self.response = Some(SseResponse::from_line(line)?);
            return Ok(self);
        };
        self.response = Some(response.add_line(line)?);
        Ok(self)
    }
    #[allow(dead_code)]
    pub fn response(&self) -> Option<&SseResponse> {
        self.response.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct SseResponse {
    start_line: SseStartLine,
    headers: Option<SseHeaders>,
    body: Option<SseBody>,
    is_next_body: bool,
}
impl FromLine for SseResponse {
    fn from_line(line: &str) -> Result<Self> {
        let start_line = SseStartLine::from_line(line)?;
        Ok(Self {
            start_line,
            headers: None,
            body: None,
            is_next_body: false,
        })
    }
}
impl SseResponse {
    pub fn http_version(&self) -> &str {
        self.start_line.http_version()
    }
    pub fn status_code(&self) -> u32 {
        self.start_line.status_code()
    }
    pub fn status_text(&self) -> &str {
        self.start_line.status_text()
    }
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.as_ref().map(|h| h.header(key))?
    }
    pub fn new_event(&self) -> Option<&str> {
        self.body.as_ref().map(|b| b.new_event())?
    }
    pub fn is_ok(&self) -> bool {
        self.start_line.is_ok()
    }
    pub fn is_error(&self) -> bool {
        self.start_line.is_error()
    }
    pub fn body(&self) -> Option<&str> {
        self.body.as_ref().map(|b| b.other())
    }
    pub fn add_line(self, line: &str) -> Result<Self> {
        if let Ok(start_line) = SseStartLine::from_line(line) {
            return Ok(Self {
                start_line,
                headers: None,
                body: None,
                is_next_body: false,
            });
        }
        // It's mistakenly recognized ad "header"
        // so it should be evaluated before the header
        if self.is_next_body {
            let mut body = self.body.unwrap();
            body.add_line(line);
            return Ok(Self {
                start_line: self.start_line,
                headers: self.headers,
                body: Some(body),
                is_next_body: true,
            });
        }
        if let Ok(header) = SseHeader::from_line(line) {
            if let Some(mut headers) = self.headers {
                let header = SseHeader::from_line(line)?;
                headers.insert(header);
                return Ok(Self {
                    start_line: self.start_line,
                    headers: Some(headers),
                    body: None,
                    is_next_body: false,
                });
            } else {
                let mut headers = SseHeaders::new();
                headers.insert(header);
                return Ok(Self {
                    start_line: self.start_line,
                    headers: Some(headers),
                    body: None,
                    is_next_body: false,
                });
            }
        };
        if line == "\r\n\r\n" || line == "\r\n" || line == "" {
            return Ok(Self {
                start_line: self.start_line,
                headers: self.headers,
                body: Some(SseBody::from_line(line)?),
                is_next_body: true,
            });
        }
        Err(SseResponseError::invalid_line(self, line))
    }
}
#[derive(Debug, Clone)]
pub struct SseStartLine {
    status_code: StatusCode,
    status_text: StatusText,
    http_version: HttpVersion,
}
impl FromLine for SseStartLine {
    fn from_line(line: &str) -> Result<Self> {
        Ok(Self {
            http_version: HttpVersion::from_line(line)?,
            status_code: StatusCode::from_line(line)?,
            status_text: StatusText::from_line(line)?,
        })
    }
}
impl SseStartLine {
    pub fn status_code(&self) -> u32 {
        self.status_code.num()
    }
    pub fn status_text(&self) -> &str {
        self.status_text.0.as_str()
    }
    pub fn http_version(&self) -> &str {
        self.http_version.to_str()
    }
    pub fn is_error(&self) -> bool {
        self.status_code.is_error()
    }
    pub fn is_ok(&self) -> bool {
        self.status_code.is_ok()
    }
}
#[derive(Debug, Clone)]
pub struct SseHeaders {
    headers: HashMap<String, String>,
}
impl SseHeaders {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }
    pub fn insert(&mut self, header: SseHeader) {
        self.headers.insert(header.name, header.value);
    }
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct SseHeader {
    name: String,
    value: String,
}
impl FromLine for SseHeader {
    fn from_line(line: &str) -> Result<Self> {
        let mut split = line.splitn(2, ':');
        let name = split
            .next()
            .ok_or(SseResponseError::invalid_header(line))?
            .trim()
            .to_string();
        let value = split
            .next()
            .ok_or(SseResponseError::invalid_header(line))?
            .trim()
            .to_string();
        Ok(Self { name, value })
    }
}
impl SseHeader {
    #[allow(dead_code)]
    pub fn key(&self) -> &str {
        self.name.as_str()
    }
    #[allow(dead_code)]
    pub fn value(&self) -> &str {
        self.value.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct SseBody {
    events: Vec<String>,
    other: String,
    has_pushed: bool, //lines: Vec<String>,
}
impl FromLine for SseBody {
    fn from_line(line: &str) -> Result<Self> {
        let Some(event) = Self::to_event(line) else {
            return Ok(Self {
                has_pushed:false,
                events:Vec::new(),
                other: line.trim().to_string(),
            })
        };
        Ok(Self {
            has_pushed: true,
            events: vec![event.to_string()],
            other: String::new(),
        })
    }
}
impl SseBody {
    pub fn add_line(&mut self, line: &str) {
        let Some(event) = Self::to_event(line) else {
            self.has_pushed = false;
            self.other.push_str(line);
            return;
        };
        self.has_pushed = true;
        self.events.push(event.to_string());
    }
    pub fn other(&self) -> &str {
        self.other.as_str()
    }
    pub fn new_event(&self) -> Option<&str> {
        if self.has_pushed {
            self.events.last().map(String::as_str)
        } else {
            None
        }
    }
    fn to_event(s: &str) -> Option<&str> {
        s.splitn(2, "data:").skip(1).next().map(|s| s.trim())
    }
}
#[derive(Debug, Clone)]
pub struct StatusText(String);
impl FromLine for StatusText {
    fn from_line(line: &str) -> Result<Self> {
        line.split_whitespace()
            .skip(2)
            .next()
            .ok_or(SseResponseError::invalid_start_line(line))
            .map(|s| Self(s.to_string()))
    }
}
impl FromLine for StatusCode {
    fn from_line(line: &str) -> Result<Self> {
        let split = line
            .split_whitespace()
            .skip(1)
            .next()
            .ok_or(SseResponseError::invalid_start_line(line))?;

        let num = split
            .parse::<u32>()
            .map_err(|_| SseResponseError::invalid_start_line(line))?;
        Ok(Self::from_num(num)?)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum HttpVersion {
    Http1_0,
    Http1_1,
    Http2_0,
}

impl FromLine for HttpVersion {
    fn from_line(line: &str) -> Result<Self> {
        let version = line
            .split_whitespace()
            .nth(0)
            .ok_or(SseResponseError::invalid_start_line(line))?;
        match version {
            "HTTP/1.0" => Ok(Self::Http1_0),
            "HTTP/1.1" => Ok(Self::Http1_1),
            "HTTP/2.0" => Ok(Self::Http2_0),
            _ => Err(SseResponseError::invalid_start_line(line)),
        }
    }
}
impl HttpVersion {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Http1_0 => "HTTP/1.0",
            Self::Http1_1 => "HTTP/1.1",
            Self::Http2_0 => "HTTP/2.0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sse_response_test() {
        let start_line = "HTTP/1.1 200 OK\r\n";
        let response = SseResponse::from_line(start_line).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(response.header("Date"), None);
        assert_eq!(response.new_event(), None);
        let header_line = "Date: Thu, 18 May 2023 10:07:36 GMT";
        let response = response.add_line(header_line).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.header("Date"),
            Some("Thu, 18 May 2023 10:07:36 GMT")
        );
        assert_eq!(response.body(), None);
        assert_eq!(response.new_event(), None);
        let start_body = "\r\n\r\n";
        let response = response.add_line(start_body).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.header("Date"),
            Some("Thu, 18 May 2023 10:07:36 GMT")
        );
        assert_eq!(response.body(), Some(""));
        assert_eq!(response.new_event(), None);
        let not_event = "start event";
        let response = response.add_line(not_event).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.header("Date"),
            Some("Thu, 18 May 2023 10:07:36 GMT")
        );
        assert_eq!(response.body(), Some(not_event));
        assert_eq!(response.new_event(), None);
        let event = "data: event1\r\n";
        let response = response.add_line(event).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.header("Date"),
            Some("Thu, 18 May 2023 10:07:36 GMT")
        );
        assert_eq!(response.body(), Some(not_event));
        assert_eq!(response.new_event(), Some("event1"));
        let event = "data: event2\r\n";
        let response = response.add_line(event).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(
            response.header("Date"),
            Some("Thu, 18 May 2023 10:07:36 GMT")
        );
        assert_eq!(response.body(), Some(not_event));
        assert_eq!(response.new_event(), Some("event2"));
        let start_line = "HTTP/1.1 200 OK\r\n";
        let response = response.add_line(start_line).unwrap();
        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_text(), "OK");
        assert_eq!(response.header("Date"), None);
        assert_eq!(response.body(), None);
        assert_eq!(response.new_event(), None);
    }
    #[test]
    fn start_line_test() {
        let line = "HTTP/1.1 200 OK\r\n";
        let start_line = SseStartLine::from_line(line).unwrap();
        assert_eq!(start_line.http_version(), "HTTP/1.1");
        assert_eq!(start_line.status_code(), 200);
        assert_eq!(start_line.status_text(), "OK");
    }
    #[test]
    fn start_line_status_test() {
        let line = "HTTP/1.1 401 Unauthorized\r\n";
        let start_line = SseStartLine::from_line(line).unwrap();
        assert!(start_line.is_error());
        let line = "HTTP/1.1 200 OK\r\n";
        let start_line = SseStartLine::from_line(line).unwrap();
        assert!(start_line.is_ok());
    }
    #[test]
    fn sse_header_test() {
        let line = "Content-Type: text/event-stream\r\n";
        let header = SseHeader::from_line(line).unwrap();
        assert_eq!(header.key(), "Content-Type");
        assert_eq!(header.value(), "text/event-stream");
    }
    #[test]
    fn sse_data_test() {
        let line = "1a7\r\n";
        let mut body = SseBody::from_line(line).unwrap();
        assert_eq!(body.new_event(), None);
        body.add_line("data: {\"id\":0,\"name\":\"kai\"}\r\n");
        assert_eq!(body.new_event(), Some(r#"{"id":0,"name":"kai"}"#));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    Continue,
    SwitchingProtocols,
    Processing,
    EarlyHints,
    OK,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultiStatus,
    AlreadyReported,
    IMUsed,
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    SwitchProxy,
    TemporaryRedirect,
    PermanentRedirect,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    PayloadTooLarge,
    URITooLong,
    UnsupportedMediaType,
    RangeNotSatisfiable,
    ExpectationFailed,
    ImAteapot,
    MisdirectedRequest,
    UnprocessableEntity,
    Locked,
    FailedDependency,
    TooEarly,
    UpgradeRequired,
    PreconditionRequired,
    TooManyRequests,
    RequestHeaderFieldsTooLarge,
    UnavailableForLegalReasons,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HTTPVersionNotSupported,
    VariantAlsoNegotiates,
    InsufficientStorage,
    LoopDetected,
    NotExtended,
    NetworkAuthenticationRequired,
}
impl Into<u32> for StatusCode {
    fn into(self) -> u32 {
        match self {
            StatusCode::Continue => 100,
            StatusCode::SwitchingProtocols => 101,
            StatusCode::Processing => 102,
            StatusCode::EarlyHints => 103,
            StatusCode::OK => 200,
            StatusCode::Created => 201,
            StatusCode::Accepted => 202,
            StatusCode::NonAuthoritativeInformation => 203,
            StatusCode::NoContent => 204,
            StatusCode::ResetContent => 205,
            StatusCode::PartialContent => 206,
            StatusCode::MultiStatus => 207,
            StatusCode::AlreadyReported => 208,
            StatusCode::IMUsed => 226,
            StatusCode::MultipleChoices => 300,
            StatusCode::MovedPermanently => 301,
            StatusCode::Found => 302,
            StatusCode::SeeOther => 303,
            StatusCode::NotModified => 304,
            StatusCode::UseProxy => 305,
            StatusCode::SwitchProxy => 306,
            StatusCode::TemporaryRedirect => 307,
            StatusCode::PermanentRedirect => 308,
            StatusCode::BadRequest => 400,
            StatusCode::Unauthorized => 401,
            StatusCode::PaymentRequired => 402,
            StatusCode::Forbidden => 403,
            StatusCode::NotFound => 404,
            StatusCode::MethodNotAllowed => 405,
            StatusCode::NotAcceptable => 406,
            StatusCode::ProxyAuthenticationRequired => 407,
            StatusCode::RequestTimeout => 408,
            StatusCode::Conflict => 409,
            StatusCode::Gone => 410,
            StatusCode::LengthRequired => 411,
            StatusCode::PreconditionFailed => 412,
            StatusCode::PayloadTooLarge => 413,
            StatusCode::URITooLong => 414,
            StatusCode::UnsupportedMediaType => 415,
            StatusCode::RangeNotSatisfiable => 416,
            StatusCode::ExpectationFailed => 417,
            StatusCode::ImAteapot => 418,
            StatusCode::MisdirectedRequest => 421,
            StatusCode::UnprocessableEntity => 422,
            StatusCode::Locked => 423,
            StatusCode::FailedDependency => 424,
            StatusCode::TooEarly => 425,
            StatusCode::UpgradeRequired => 426,
            StatusCode::PreconditionRequired => 428,
            StatusCode::TooManyRequests => 429,
            StatusCode::RequestHeaderFieldsTooLarge => 431,
            StatusCode::UnavailableForLegalReasons => 451,
            StatusCode::InternalServerError => 500,
            StatusCode::NotImplemented => 501,
            StatusCode::BadGateway => 502,
            StatusCode::ServiceUnavailable => 503,
            StatusCode::GatewayTimeout => 504,
            StatusCode::HTTPVersionNotSupported => 505,
            StatusCode::VariantAlsoNegotiates => 506,
            StatusCode::InsufficientStorage => 507,
            StatusCode::LoopDetected => 508,
            StatusCode::NotExtended => 510,
            StatusCode::NetworkAuthenticationRequired => 511,
        }
    }
}
impl StatusCode {
    pub fn num(&self) -> u32 {
        self.clone().into()
    }
    pub fn is_ok(&self) -> bool {
        self.num() < 400
    }
    pub fn is_error(&self) -> bool {
        self.num() >= 400
    }
    fn from_num(num: u32) -> Result<Self> {
        match num {
            100 => Ok(StatusCode::Continue),
            101 => Ok(StatusCode::SwitchingProtocols),
            102 => Ok(StatusCode::Processing),
            103 => Ok(StatusCode::EarlyHints),
            200 => Ok(StatusCode::OK),
            201 => Ok(StatusCode::Created),
            202 => Ok(StatusCode::Accepted),
            203 => Ok(StatusCode::NonAuthoritativeInformation),
            204 => Ok(StatusCode::NoContent),
            205 => Ok(StatusCode::ResetContent),
            206 => Ok(StatusCode::PartialContent),
            207 => Ok(StatusCode::MultiStatus),
            208 => Ok(StatusCode::AlreadyReported),
            226 => Ok(StatusCode::IMUsed),
            300 => Ok(StatusCode::MultipleChoices),
            301 => Ok(StatusCode::MovedPermanently),
            302 => Ok(StatusCode::Found),
            303 => Ok(StatusCode::SeeOther),
            304 => Ok(StatusCode::NotModified),
            305 => Ok(StatusCode::UseProxy),
            306 => Ok(StatusCode::SwitchProxy),
            307 => Ok(StatusCode::TemporaryRedirect),
            308 => Ok(StatusCode::PermanentRedirect),
            400 => Ok(StatusCode::BadRequest),
            401 => Ok(StatusCode::Unauthorized),
            402 => Ok(StatusCode::PaymentRequired),
            403 => Ok(StatusCode::Forbidden),
            404 => Ok(StatusCode::NotFound),
            405 => Ok(StatusCode::MethodNotAllowed),
            406 => Ok(StatusCode::NotAcceptable),
            407 => Ok(StatusCode::ProxyAuthenticationRequired),
            408 => Ok(StatusCode::RequestTimeout),
            409 => Ok(StatusCode::Conflict),
            410 => Ok(StatusCode::Gone),
            411 => Ok(StatusCode::LengthRequired),
            412 => Ok(StatusCode::PreconditionFailed),
            413 => Ok(StatusCode::PayloadTooLarge),
            414 => Ok(StatusCode::URITooLong),
            415 => Ok(StatusCode::UnsupportedMediaType),
            416 => Ok(StatusCode::RangeNotSatisfiable),
            417 => Ok(StatusCode::ExpectationFailed),
            418 => Ok(StatusCode::ImAteapot),
            421 => Ok(StatusCode::MisdirectedRequest),
            422 => Ok(StatusCode::UnprocessableEntity),
            423 => Ok(StatusCode::Locked),
            424 => Ok(StatusCode::FailedDependency),
            425 => Ok(StatusCode::TooEarly),
            426 => Ok(StatusCode::UpgradeRequired),
            428 => Ok(StatusCode::PreconditionRequired),
            429 => Ok(StatusCode::TooManyRequests),
            431 => Ok(StatusCode::RequestHeaderFieldsTooLarge),
            451 => Ok(StatusCode::UnavailableForLegalReasons),
            500 => Ok(StatusCode::InternalServerError),
            501 => Ok(StatusCode::NotImplemented),
            502 => Ok(StatusCode::BadGateway),
            503 => Ok(StatusCode::ServiceUnavailable),
            504 => Ok(StatusCode::GatewayTimeout),
            505 => Ok(StatusCode::HTTPVersionNotSupported),
            506 => Ok(StatusCode::VariantAlsoNegotiates),
            507 => Ok(StatusCode::InsufficientStorage),
            508 => Ok(StatusCode::LoopDetected),
            510 => Ok(StatusCode::NotExtended),
            511 => Ok(StatusCode::NetworkAuthenticationRequired),
            _ => Err(SseResponseError::invalid_status_code(num)),
        }
    }
}
