use crate::request::Request;

pub type Result<T> = std::result::Result<T, SseConnectionError>;

#[derive(Debug, PartialEq, Clone)]
pub enum SseResponse {
    Data(String),
    Done,
}

pub trait SseConnection {
    fn consume(&mut self) -> Result<SseResponse>;
}
pub trait SseConnector {
    type Connection: SseConnection;
    fn connect(&mut self, req: &Request) -> Result<&mut Self::Connection>;
}

#[derive(Debug)]
pub struct SseConnectionError {
    message: String,
}

impl SseConnectionError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for SseConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SseConnectionError: {}", self.message)
    }
}
impl std::error::Error for SseConnectionError {}

#[cfg(test)]
pub(crate) mod fakes {
    use super::*;
    pub struct FakeSseConnector {
        called_time: usize,
        connection: FakeSseConnection,
    }
    impl FakeSseConnector {
        pub fn new() -> Self {
            Self {
                called_time: 0,
                connection: FakeSseConnection {
                    index: 0,
                    response: String::new(),
                },
            }
        }
        pub fn set_success_sse(&mut self, response: &str) {
            self.connection.response = response.to_string();
        }
        pub fn connected_time(&self) -> usize {
            self.called_time
        }
    }
    pub struct FakeSseConnection {
        index: usize,
        response: String,
    }
    impl SseConnection for FakeSseConnection {
        fn consume(&mut self) -> Result<SseResponse> {
            let c = self
                .response
                .get(self.index..self.index + 1)
                .map(String::from);
            self.index += 1;
            match c {
                Some(c) => Ok(SseResponse::Data(c)),
                None => Ok(SseResponse::Done),
            }
        }
    }
    impl SseConnector for FakeSseConnector {
        type Connection = FakeSseConnection;
        fn connect(&mut self, _req: &Request) -> Result<&mut FakeSseConnection> {
            self.called_time += 1;
            Ok(&mut self.connection)
        }
    }
}
