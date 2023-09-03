use crate::{http::response::HttpResponse, request::Request};

use super::{
    connector::{ConnectedSseResponse, Socket, SseConnection, SseConnectionError, SseConnector},
    response::SseResponse,
};

pub type Result<T> = std::result::Result<T, SseSubscribeError>;
pub trait SseHandler {
    fn handle(&self, res: SseResponse);
}
pub trait SseMutHandler {
    fn handle(&mut self, res: SseResponse);
}
pub struct SseSubscriber<T: SseConnector> {
    connector: T,
}
impl<T: SseConnector> SseSubscriber<T> {
    pub fn new(connector: T) -> Self {
        Self { connector }
    }
    pub fn subscribe(&mut self, req: &Request, handler: &impl SseHandler) -> Result<()> {
        let mut conn = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            let res = conn.read().map_err(SseSubscribeError::from)?;
            match res {
                ConnectedSseResponse::Progress(sse_response) => {
                    handler.handle(sse_response);
                }
                ConnectedSseResponse::Done => {
                    return Ok(());
                }
            }
        }
    }
    pub fn subscribe_mut(&mut self, req: &Request, handler: &mut impl SseMutHandler) -> Result<()> {
        let mut connection = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            let res = connection.read().map_err(SseSubscribeError::from)?;
            match res {
                ConnectedSseResponse::Progress(sse_response) => {
                    handler.handle(sse_response);
                }
                ConnectedSseResponse::Done => {
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum SseSubscribeError {
    ConnectionError(SseConnectionError),
    HttpError(HttpResponse),
}
impl std::fmt::Display for SseSubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SseSubscribeError::HttpError(err) => {
                write!(f, "SseSubscribeError: {}", err.to_string())
            }
            Self::ConnectionError(err) => write!(f, "SseSubscribeError: {}", err.to_string()),
        }
    }
}
impl std::error::Error for SseSubscribeError {}
impl From<SseConnectionError> for SseSubscribeError {
    fn from(err: SseConnectionError) -> Self {
        match err {
            SseConnectionError::HttpError(err) => Self::HttpError(err),
            _ => Self::ConnectionError(err),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        request::RequestBuilder,
        sse::{
            connector::fakes::FakeSseConnector,
            subscriber::fakes::{MockHandler, MockMutHandler},
        },
    };

    use super::*;
    #[test]
    fn sseのデータを不変なhandlerが捕捉する() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        let handler = MockHandler::new();
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new("https://www.fake").get().build();

        sut.subscribe(&request, &handler).unwrap();

        assert_eq!(sut.connector.connected_times(), 1);
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ])
    }
    #[test]
    fn sseのデータを可変なhandlerが捕捉する() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        let mut handler = MockMutHandler::new();
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new("https://www.fake").get().build();

        sut.subscribe_mut(&request, &mut handler).unwrap();

        assert_eq!(sut.connector.connected_times(), 1);
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ])
    }
    #[test]
    fn sseのhttp接続エラーの場合はhttpのレスポンスをエラーに包んで返す() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 400 Bad Request\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");

        let mut handler = MockMutHandler::new();
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new("https://www.fake").get().build();

        let result = sut.subscribe_mut(&request, &mut handler);
        let Err(SseSubscribeError::HttpError(err)) = result else {
            panic!("expected Err, but got Ok");
        };

        assert_eq!(err.status_code(), 400);
        assert_eq!(err.get_header("Content-Type"), Some("text/event-stream"));
    }
}
#[cfg(test)]
pub(crate) mod fakes {
    use std::cell::RefCell;

    use crate::sse::response::SseResponse;

    use super::{SseHandler, SseMutHandler};
    pub struct MockHandler {
        called: RefCell<usize>,
        events: RefCell<Vec<SseResponse>>,
    }
    impl MockHandler {
        pub fn new() -> Self {
            Self {
                called: RefCell::new(0),
                events: RefCell::new(vec![]),
            }
        }
        pub fn called_time(&self) -> usize {
            self.called.borrow().clone()
        }
        pub fn assert_received(&self, expected: &[SseResponse]) {
            assert_eq!(*self.events.borrow(), expected);
        }
    }
    impl SseHandler for MockHandler {
        fn handle(&self, message: SseResponse) {
            self.events.borrow_mut().push(message);
            *self.called.borrow_mut() += 1;
        }
    }
    pub struct MockMutHandler {
        called: usize,
        events: Vec<SseResponse>,
    }
    impl MockMutHandler {
        pub fn new() -> Self {
            Self {
                called: 0,
                events: vec![],
            }
        }
        pub fn called_time(&self) -> usize {
            self.called
        }
        pub fn assert_received(&self, expected: &[SseResponse]) {
            assert_eq!(self.events, expected);
        }
    }
    impl SseMutHandler for MockMutHandler {
        fn handle(&mut self, message: SseResponse) {
            self.events.push(message);
            self.called += 1;
        }
    }
}
