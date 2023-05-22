use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub enum SseResponseError {
    InvalidStartLine(String),
    InvalidStatusCode(u32),
}
impl SseResponseError {
    pub fn invalid_start_line(s: &str) -> Self {
        Self::InvalidStartLine(format!("Invalid start line: {}", s))
    }
    pub fn invalid_status_code(code: u32) -> Self {
        Self::InvalidStatusCode(code)
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
}

#[derive(Debug)]
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

#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug)]
pub struct HttpResponse {
    status_code: u32,
    headers: HashMap<String, String>,
    body: Vec<String>,
    is_next_body: bool,
    is_next_header: bool,
    next_line_count: usize,
    all: String,
}
impl HttpResponse {
    pub fn new() -> Self {
        Self {
            all: String::new(),
            status_code: 0,
            headers: HashMap::new(),
            body: Vec::new(),
            is_next_body: false,
            is_next_header: false,
            next_line_count: 0,
        }
    }
    pub fn add_line(&mut self, line: &str) {
        self.all.push_str(line);
        if self.is_start_line() && line.starts_with("HTTP/") {
            self.status_code = line.split(" ").nth(1).unwrap().parse().unwrap();
            self.is_next_header = true;
            return;
        }
        if self.is_next_header || line == "\r\n" {
            self.next_line_count += 1;
            return;
        }
        if self.next_line_count == 2 {
            self.is_next_body = true;
            self.is_next_header = false;
            self.next_line_count = 0;
            return;
        }
        if self.is_next_header {
            //&& line.contains(":") {
            self.next_line_count = 0;
            let mut iter = line.split(":");
            let key = iter.next().unwrap().trim().to_string();
            let value = iter.next().unwrap().trim().to_string();
            self.headers.insert(key, value);
            return;
        }
        if self.is_next_body {
            self.body.push(line.into());
        }
    }
    pub fn status_code(&self) -> u32 {
        self.status_code
    }
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|v| v.as_str())
    }
    pub fn body(&self) -> String {
        self.body.iter().fold(String::new(), |mut acc, cur| {
            acc.push_str(cur.as_str());
            acc
        })
    }
    pub fn has_error(&self) -> bool {
        self.status_code >= 400 && self.status_code < 600
    }
    pub fn to_string(&self) -> String {
        self.all.clone()
    }
    fn is_start_line(&self) -> bool {
        self.status_code == 0 && self.headers.is_empty()
    }
    pub fn new_event(&self) -> Option<String> {
        for line in self.body.iter().rev() {
            if line.starts_with("data:") {
                let data = line.replacen("data: ", "", 1);
                return Some(data);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn start_line_test() {
        let line = "HTTP/1.1 200 OK\r\n";
        let start_line = SseStartLine::from_line(line).unwrap();
        assert_eq!(start_line.http_version(), "HTTP/1.1");
        assert_eq!(start_line.status_code(), 200);
        assert_eq!(start_line.status_text(), "OK");
    }
    //#[test]
    //fn sse用のhttp_responseは随時bodyにデータを追加できる() {
    //let mut http_response = HttpResponse::new();
    //http_response.add_line("HTTP/1.1 200 OK\r\n");
    //http_response.add_line("Content-Type: text/event-stream\r\n");
    //http_response.add_line("\r\n\r\n");
    //http_response.add_line("start\r\n");
    //http_response.add_line("data: 1\r\n");
    //http_response.add_line("data: 2\r\n");
    //http_response.add_line(r#"data: {"id":"chatcmpl-7HUPdLSLH82dsYD8nWD0gPqFFO8jU","object":"chat.completion.chunk","created":1684402837,"model":"gpt-3.5-turbo-0301","choices":[{"delta":{"role":"assistant"},"index":0,"finish_reason":null}]}\r\n"#);
    //assert_eq!(http_response.status_code(), 200);
    //assert_eq!(
    //http_response.get_header("Content-Type").unwrap(),
    //"text/event-stream"
    //);
    ////assert_eq!(http_response.body(), "start\ndata: 1\ndata: 2\ndata: 3\n");
    //let data = r#"{"id":"chatcmpl-7HUPdLSLH82dsYD8nWD0gPqFFO8jU","object":"chat.completion.chunk","created":1684402837,"model":"gpt-3.5-turbo-0301","choices":[{"delta":{"role":"assistant"},"index":0,"finish_reason":null}]}"#;
    //assert_eq!(http_response.new_event().unwrap(), data);
    //}
    //#[test]
    //fn 連続的な行からhttp_responseを構築可能() {
    //let mut http_response = HttpResponse::new();
    //http_response.add_line("HTTP/1.1 200 OK");
    //http_response.add_line("Content-Type: text/event-stream");
    //http_response.add_line("");
    //http_response.add_line("start\n");
    //http_response.add_line("data: 1\n");
    //http_response.add_line("data: 2\n");
    //assert_eq!(http_response.status_code(), 200);
    //assert_eq!(
    //http_response.get_header("Content-Type").unwrap(),
    //"text/event-stream"
    //);
    //assert_eq!(http_response.body(), "start\ndata: 1\ndata: 2\n");
    //}
}
