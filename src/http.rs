use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpStatusLineError {
    InvalidFormat(String),
}
impl Display for HttpStatusLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpStatusLineError::InvalidFormat(line) => {
                write!(f, "Invalid format: {}", line)
            }
        }
    }
}
impl std::error::Error for HttpStatusLineError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpStatusLine {
    version: HttpVersion,
    status_code: HttpStatusCode,
}
impl HttpStatusLine {
    pub fn new(version: HttpVersion, status_code: HttpStatusCode) -> Self {
        Self {
            version,
            status_code,
        }
    }
    pub fn from_str(line: &str) -> Result<Self, HttpStatusLineError> {
        let mut split_line = line.split(" ");
        let (Some(version),Some(status_num),Some(_status_message)) = (split_line.next(),split_line.next(),split_line.next()) else {
            return Err(HttpStatusLineError::InvalidFormat(line.to_string()));
        };
        let Some(version) = HttpVersion::from_str(version) else {
            return Err(HttpStatusLineError::InvalidFormat(line.to_string()));
        };
        let Some(status_code) = HttpStatusCode::from_num_str(status_num) else {
            return Err(HttpStatusLineError::InvalidFormat(line.to_string()));
        };
        Ok(Self::new(version, status_code))
    }
    pub fn to_string(&self) -> String {
        format!(
            "{} {} {}",
            self.version.to_str(),
            self.status_code.num(),
            self.status_code.to_str()
        )
    }
    pub fn version(&self) -> HttpVersion {
        self.version
    }
    pub fn status_code(&self) -> HttpStatusCode {
        self.status_code
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    V1_1,
}
impl HttpVersion {
    pub fn to_str(&self) -> &'static str {
        match self {
            HttpVersion::V1_1 => "HTTP/1.1",
        }
    }
    pub fn from_str(version: &str) -> Option<Self> {
        match version {
            "HTTP/1.1" => Some(HttpVersion::V1_1),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusCode {
    Unknown,
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
impl Into<u32> for HttpStatusCode {
    fn into(self) -> u32 {
        match self {
            HttpStatusCode::Continue => 100,
            HttpStatusCode::SwitchingProtocols => 101,
            HttpStatusCode::Processing => 102,
            HttpStatusCode::EarlyHints => 103,
            HttpStatusCode::OK => 200,
            HttpStatusCode::Created => 201,
            HttpStatusCode::Accepted => 202,
            HttpStatusCode::NonAuthoritativeInformation => 203,
            HttpStatusCode::NoContent => 204,
            HttpStatusCode::ResetContent => 205,
            HttpStatusCode::PartialContent => 206,
            HttpStatusCode::MultiStatus => 207,
            HttpStatusCode::AlreadyReported => 208,
            HttpStatusCode::IMUsed => 226,
            HttpStatusCode::MultipleChoices => 300,
            HttpStatusCode::MovedPermanently => 301,
            HttpStatusCode::Found => 302,
            HttpStatusCode::SeeOther => 303,
            HttpStatusCode::NotModified => 304,
            HttpStatusCode::UseProxy => 305,
            HttpStatusCode::SwitchProxy => 306,
            HttpStatusCode::TemporaryRedirect => 307,
            HttpStatusCode::PermanentRedirect => 308,
            HttpStatusCode::BadRequest => 400,
            HttpStatusCode::Unauthorized => 401,
            HttpStatusCode::PaymentRequired => 402,
            HttpStatusCode::Forbidden => 403,
            HttpStatusCode::NotFound => 404,
            HttpStatusCode::MethodNotAllowed => 405,
            HttpStatusCode::NotAcceptable => 406,
            HttpStatusCode::ProxyAuthenticationRequired => 407,
            HttpStatusCode::RequestTimeout => 408,
            HttpStatusCode::Conflict => 409,
            HttpStatusCode::Gone => 410,
            HttpStatusCode::LengthRequired => 411,
            HttpStatusCode::PreconditionFailed => 412,
            HttpStatusCode::PayloadTooLarge => 413,
            HttpStatusCode::URITooLong => 414,
            HttpStatusCode::UnsupportedMediaType => 415,
            HttpStatusCode::RangeNotSatisfiable => 416,
            HttpStatusCode::ExpectationFailed => 417,
            HttpStatusCode::ImAteapot => 418,
            HttpStatusCode::MisdirectedRequest => 421,
            HttpStatusCode::UnprocessableEntity => 422,
            HttpStatusCode::Locked => 423,
            HttpStatusCode::FailedDependency => 424,
            HttpStatusCode::TooEarly => 425,
            HttpStatusCode::UpgradeRequired => 426,
            HttpStatusCode::PreconditionRequired => 428,
            HttpStatusCode::TooManyRequests => 429,
            HttpStatusCode::RequestHeaderFieldsTooLarge => 431,
            HttpStatusCode::UnavailableForLegalReasons => 451,
            HttpStatusCode::InternalServerError => 500,
            HttpStatusCode::NotImplemented => 501,
            HttpStatusCode::BadGateway => 502,
            HttpStatusCode::ServiceUnavailable => 503,
            HttpStatusCode::GatewayTimeout => 504,
            HttpStatusCode::HTTPVersionNotSupported => 505,
            HttpStatusCode::VariantAlsoNegotiates => 506,
            HttpStatusCode::InsufficientStorage => 507,
            HttpStatusCode::LoopDetected => 508,
            HttpStatusCode::NotExtended => 510,
            HttpStatusCode::NetworkAuthenticationRequired => 511,
            HttpStatusCode::Unknown => 0,
        }
    }
}
impl HttpStatusCode {
    pub fn num(&self) -> u32 {
        self.clone().into()
    }
    pub fn from_num_str(s: &str) -> Option<Self> {
        let parse = s.parse::<u32>();
        let Ok(num) = parse else {
            return None;
        };
        Some(Self::from_num(num))
    }
    pub fn is_ok(&self) -> bool {
        self.num() < 400
    }
    pub fn is_error(&self) -> bool {
        self.num() >= 400
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            HttpStatusCode::Continue => "Continue",
            HttpStatusCode::SwitchingProtocols => "Switching Protocols",
            HttpStatusCode::Processing => "Processing",
            HttpStatusCode::EarlyHints => "Early Hints",
            HttpStatusCode::OK => "OK",
            HttpStatusCode::Created => "Created",
            HttpStatusCode::Accepted => "Accepted",
            HttpStatusCode::NonAuthoritativeInformation => "Non-Authoritative Information",
            HttpStatusCode::NoContent => "No Content",
            HttpStatusCode::ResetContent => "Reset Content",
            HttpStatusCode::PartialContent => "Partial Content",
            HttpStatusCode::MultiStatus => "Multi-Status",
            HttpStatusCode::AlreadyReported => "Already Reported",
            HttpStatusCode::IMUsed => "IM Used",
            HttpStatusCode::MultipleChoices => "Multiple Choices",
            HttpStatusCode::MovedPermanently => "Moved Permanently",
            HttpStatusCode::Found => "Found",
            HttpStatusCode::SeeOther => "See Other",
            HttpStatusCode::NotModified => "Not Modified",
            HttpStatusCode::UseProxy => "Use Proxy",
            HttpStatusCode::SwitchProxy => "Switch Proxy",
            HttpStatusCode::TemporaryRedirect => "Temporary Redirect",
            HttpStatusCode::PermanentRedirect => "Permanent Redirect",
            HttpStatusCode::BadRequest => "Bad Request",
            HttpStatusCode::Unauthorized => "Unauthorized",
            HttpStatusCode::PaymentRequired => "Payment Required",
            HttpStatusCode::Forbidden => "Forbidden",
            HttpStatusCode::NotFound => "Not Found",
            HttpStatusCode::MethodNotAllowed => "Method Not Allowed",
            HttpStatusCode::NotAcceptable => "Not Acceptable",
            HttpStatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
            HttpStatusCode::RequestTimeout => "Request Timeout",
            HttpStatusCode::Conflict => "Conflict",
            HttpStatusCode::Gone => "Gone",
            HttpStatusCode::LengthRequired => "Length Required",
            HttpStatusCode::PreconditionFailed => "Precondition Failed",
            HttpStatusCode::PayloadTooLarge => "Payload Too Large",
            HttpStatusCode::URITooLong => "URI Too Long",
            HttpStatusCode::UnsupportedMediaType => "Unsupported Media Type",
            HttpStatusCode::RangeNotSatisfiable => "Range Not Satisfiable",
            HttpStatusCode::ExpectationFailed => "Expectation Failed",
            HttpStatusCode::ImAteapot => "I'm a teapot",
            HttpStatusCode::MisdirectedRequest => "Misdirected Request",
            HttpStatusCode::UnprocessableEntity => "Unprocessable Entity",
            HttpStatusCode::Locked => "Locked",
            HttpStatusCode::FailedDependency => "Failed Dependency",
            HttpStatusCode::TooEarly => "Too Early",
            HttpStatusCode::UpgradeRequired => "Upgrade Required",
            HttpStatusCode::PreconditionRequired => "Precondition Required",
            HttpStatusCode::TooManyRequests => "Too Many Requests",
            HttpStatusCode::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            HttpStatusCode::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            HttpStatusCode::InternalServerError => "Internal Server Error",
            HttpStatusCode::NotImplemented => "Not Implemented",
            HttpStatusCode::BadGateway => "Bad Gateway",
            HttpStatusCode::ServiceUnavailable => "Service Unavailable",
            HttpStatusCode::GatewayTimeout => "Gateway Timeout",
            HttpStatusCode::HTTPVersionNotSupported => "HTTP Version Not Supported",
            HttpStatusCode::VariantAlsoNegotiates => "Variant Also Negotiates",
            HttpStatusCode::InsufficientStorage => "Insufficient Storage",
            HttpStatusCode::LoopDetected => "Loop Detected",
            HttpStatusCode::NotExtended => "Not Extended",
            HttpStatusCode::NetworkAuthenticationRequired => "Network Authentication Required",
            HttpStatusCode::Unknown => "Unknown",
        }
    }
    fn from_num(num: u32) -> Self {
        match num {
            100 => HttpStatusCode::Continue,
            101 => HttpStatusCode::SwitchingProtocols,
            102 => HttpStatusCode::Processing,
            103 => HttpStatusCode::EarlyHints,
            200 => HttpStatusCode::OK,
            201 => HttpStatusCode::Created,
            202 => HttpStatusCode::Accepted,
            203 => HttpStatusCode::NonAuthoritativeInformation,
            204 => HttpStatusCode::NoContent,
            205 => HttpStatusCode::ResetContent,
            206 => HttpStatusCode::PartialContent,
            207 => HttpStatusCode::MultiStatus,
            208 => HttpStatusCode::AlreadyReported,
            226 => HttpStatusCode::IMUsed,
            300 => HttpStatusCode::MultipleChoices,
            301 => HttpStatusCode::MovedPermanently,
            302 => HttpStatusCode::Found,
            303 => HttpStatusCode::SeeOther,
            304 => HttpStatusCode::NotModified,
            305 => HttpStatusCode::UseProxy,
            306 => HttpStatusCode::SwitchProxy,
            307 => HttpStatusCode::TemporaryRedirect,
            308 => HttpStatusCode::PermanentRedirect,
            400 => HttpStatusCode::BadRequest,
            401 => HttpStatusCode::Unauthorized,
            402 => HttpStatusCode::PaymentRequired,
            403 => HttpStatusCode::Forbidden,
            404 => HttpStatusCode::NotFound,
            405 => HttpStatusCode::MethodNotAllowed,
            406 => HttpStatusCode::NotAcceptable,
            407 => HttpStatusCode::ProxyAuthenticationRequired,
            408 => HttpStatusCode::RequestTimeout,
            409 => HttpStatusCode::Conflict,
            410 => HttpStatusCode::Gone,
            411 => HttpStatusCode::LengthRequired,
            412 => HttpStatusCode::PreconditionFailed,
            413 => HttpStatusCode::PayloadTooLarge,
            414 => HttpStatusCode::URITooLong,
            415 => HttpStatusCode::UnsupportedMediaType,
            416 => HttpStatusCode::RangeNotSatisfiable,
            417 => HttpStatusCode::ExpectationFailed,
            418 => HttpStatusCode::ImAteapot,
            421 => HttpStatusCode::MisdirectedRequest,
            422 => HttpStatusCode::UnprocessableEntity,
            423 => HttpStatusCode::Locked,
            424 => HttpStatusCode::FailedDependency,
            425 => HttpStatusCode::TooEarly,
            426 => HttpStatusCode::UpgradeRequired,
            428 => HttpStatusCode::PreconditionRequired,
            429 => HttpStatusCode::TooManyRequests,
            431 => HttpStatusCode::RequestHeaderFieldsTooLarge,
            451 => HttpStatusCode::UnavailableForLegalReasons,
            500 => HttpStatusCode::InternalServerError,
            501 => HttpStatusCode::NotImplemented,
            502 => HttpStatusCode::BadGateway,
            503 => HttpStatusCode::ServiceUnavailable,
            504 => HttpStatusCode::GatewayTimeout,
            505 => HttpStatusCode::HTTPVersionNotSupported,
            506 => HttpStatusCode::VariantAlsoNegotiates,
            507 => HttpStatusCode::InsufficientStorage,
            508 => HttpStatusCode::LoopDetected,
            510 => HttpStatusCode::NotExtended,
            511 => HttpStatusCode::NetworkAuthenticationRequired,
            _ => HttpStatusCode::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 改行文字があるstatus_lineの文字列から構造体を生成可能() {
        let status_line = "HTTP/1.1 200 OK\n\r";
        let sut = HttpStatusLine::from_str(status_line).unwrap();

        assert_eq!(sut.version(), HttpVersion::V1_1);
        assert_eq!(sut.status_code(), HttpStatusCode::OK);
        assert_eq!(sut.to_string(), "HTTP/1.1 200 OK");
    }
    #[test]
    fn status_lineの文字列から構造体を生成可能() {
        let status_line = "HTTP/1.1 200 OK";
        let sut = HttpStatusLine::from_str(status_line).unwrap();

        assert_eq!(sut.version(), HttpVersion::V1_1);
        assert_eq!(sut.status_code(), HttpStatusCode::OK);
        assert_eq!(sut.to_string(), status_line);
    }
}
