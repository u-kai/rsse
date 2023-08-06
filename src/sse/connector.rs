use crate::{
    http::{
        header::HttpHeader,
        status_line::{HttpStatusLine, HttpVersion},
    },
    request::Request,
};

use super::response::SseResponse;

pub type Result<T> = std::result::Result<T, SseConnectionError>;

#[derive(Debug, PartialEq, Clone)]
pub enum ConnectedSseResponse {
    Progress(SseResponse),
    Done,
}

pub trait Socket {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error>;
    fn write(&mut self, data: &[u8]) -> std::result::Result<(), std::io::Error>;
}
pub struct SseConnection<S: Socket> {
    conn: S,
}
impl<S: Socket> SseConnection<S> {
    pub fn new(conn: S) -> Self {
        Self { conn }
    }
    pub fn consume(&mut self) -> Result<ConnectedSseResponse> {
        while let Some(line) = self
            .conn
            .read_line()
            .map_err(|e| SseConnectionError::IOError(e))?
        {
            if let Ok(_http_version) = HttpStatusLine::from_str(&line) {
                continue;
            };
            // sse_response is look like header, so check sse_response first
            if let Ok(sse_response) = SseResponse::from_line(line.as_str()) {
                return Ok(ConnectedSseResponse::Progress(sse_response));
            };
            if let Ok(_header) = HttpHeader::from_line(line.as_str()) {
                continue;
            };
        }
        Ok(ConnectedSseResponse::Done)
    }
}
pub trait SseConnector {
    fn connect<S: Socket>(&mut self, req: &Request) -> Result<&mut SseConnection<S>>;
}

#[derive(Debug)]
pub enum SseConnectionError {
    IOError(std::io::Error),
}

impl std::fmt::Display for SseConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SseConnectionError::IOError(err) => {
                write!(f, "SseConnectionError: {}", err.to_string())
            }
        }
    }
}
impl std::error::Error for SseConnectionError {}

#[cfg(test)]
mod tests {
    use crate::sse::connector::fakes::FakeTcpConnection;

    use super::*;
    #[test]
    fn sse_connectionはデータを接続相手から受け取りsseのレスポンスを返す() {
        let mut fake = FakeTcpConnection::new();
        fake.set_response("HTTP/1.1 200 OK\n\n");
        fake.set_response("Content-Type: text/event-stream\n\n");
        fake.set_response("data: Hello, World!\n\n");
        fake.set_response("data: Good Bye World\n\n");
        let mut sut = SseConnection::new(fake);
        let result = sut.consume().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Hello, World!".to_string()))
        );

        let result = sut.consume().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Good Bye World".to_string()))
        );

        let done = sut.consume().unwrap();
        assert_eq!(done, ConnectedSseResponse::Done);
    }
}
#[cfg(test)]
pub(crate) mod fakes {
    pub struct FakeTcpConnection {
        responses: Vec<String>,
    }
    impl FakeTcpConnection {
        pub fn new() -> Self {
            Self {
                responses: Vec::new(),
            }
        }
        pub fn set_response(&mut self, response: &str) {
            self.responses.push(response.to_string());
        }
    }
    impl super::Socket for FakeTcpConnection {
        fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
            if self.responses.is_empty() {
                return Ok(None);
            }
            Ok(Some(self.responses.remove(0)))
        }
        fn write(&mut self, _data: &[u8]) -> std::result::Result<(), std::io::Error> {
            Ok(())
        }
    }
}

//#[cfg(test)]
//pub(crate) mod fakes {
//use crate::http::{
//body::HttpBody, header::HttpHeader, response::HttpResponse, status_line::HttpStatusLine,
//};

//use super::*;
//pub struct FakeSseConnector {
//called_time: usize,
//connection: FakeSseConnection,
//}
//impl FakeSseConnector {
//pub fn new() -> Self {
//Self {
//called_time: 0,
//connection: FakeSseConnection {
//index: 0,
//response: String::new(),
//},
//}
//}
//pub fn set_success_sse(&mut self, response: &str) {
//self.connection.response = response.to_string();
//}
//pub fn set_http_response(&mut self, response: HttpResponse) {
//self.connection.response = response.to_string();
//}
//pub fn connected_time(&self) -> usize {
//self.called_time
//}
//}
//pub struct FakeSseConnection {
//index: usize,
//response: String,
//}
//impl SseConnection for FakeSseConnection {
//fn consume(&mut self) -> Result<ConnectedSseResponse> {
//let c = self
//.response
//.get(self.index..self.index + 1)
//.map(String::from);
//self.index += 1;
//match c {
//Some(c) => Ok(ConnectedSseResponse::Progress(SseResponse::Data(c))),
//None => Ok(ConnectedSseResponse::Done),
//}
//}
//}
//impl SseConnector for FakeSseConnector {
//type Connection = FakeSseConnection;
//fn connect(&mut self, _req: &Request) -> Result<&mut FakeSseConnection> {
//self.called_time += 1;
//Ok(&mut self.connection)
//}
//}
//}
