use std::fmt::Debug;

use thiserror::Error;

use crate::http::{request::Request, response::HttpResponse};

use super::{
    connector::{ConnectedSseResponse, SseConnectionError, SseConnector},
    response::SseResponse,
};
pub type Result<T, E> = std::result::Result<T, SseSubscribeError<E>>;

#[derive(Debug)]
pub enum HandleProgress<E> {
    Done,
    Progress,
    Err(E),
}

pub trait SseHandler<T, E> {
    fn handle(&self, res: SseResponse) -> HandleProgress<E>;
    fn result(&self) -> std::result::Result<T, E>;
}
pub trait SseMutHandler<T, E> {
    fn handle(&mut self, res: SseResponse) -> HandleProgress<E>;
    fn result(&self) -> std::result::Result<T, E>;
}

#[derive(Debug)]
pub struct SseSubscriber<C: SseConnector> {
    connector: C,
}
impl<C: SseConnector> SseSubscriber<C> {
    pub fn new(connector: C) -> Self {
        Self { connector }
    }

    pub fn subscribe<T, E>(
        &mut self,
        req: &Request,
        handler: &impl SseHandler<T, E>,
    ) -> Result<T, E> {
        let conn = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            let res = conn.read().map_err(SseSubscribeError::from)?;
            match res {
                ConnectedSseResponse::Progress(sse_response) => {
                    match handler.handle(sse_response) {
                        HandleProgress::Progress => {}
                        HandleProgress::Done => {
                            return handler
                                .result()
                                .map_err(|e| SseSubscribeError::HandlerError(e));
                        }
                        HandleProgress::Err(e) => {
                            return Err(SseSubscribeError::HandlerError(e));
                        }
                    };
                }
                ConnectedSseResponse::Done => {
                    return handler
                        .result()
                        .map_err(|e| SseSubscribeError::HandlerError(e));
                }
            }
        }
    }

    pub fn subscribe_mut<T, E>(
        &mut self,
        req: &Request,
        handler: &mut impl SseMutHandler<T, E>,
    ) -> Result<T, E> {
        let connection = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            let res = connection.read().map_err(SseSubscribeError::from)?;
            match res {
                ConnectedSseResponse::Progress(sse_response) => {
                    match handler.handle(sse_response) {
                        HandleProgress::Progress => {}
                        HandleProgress::Done => {
                            return handler
                                .result()
                                .map_err(|e| SseSubscribeError::HandlerError(e));
                        }
                        HandleProgress::Err(_) => {
                            todo!()
                        }
                    };
                }
                ConnectedSseResponse::Done => {
                    return handler
                        .result()
                        .map_err(|e| SseSubscribeError::HandlerError(e));
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum SseSubscribeError<E> {
    #[error("SseSubscribeError invalid url: {0}")]
    InvalidUrl(String),
    #[error("SseSubscribeError connection error: {0}")]
    ConnectionError(SseConnectionError),
    #[error("SseSubscribeError http error: {0}")]
    HttpError(HttpResponse),
    #[error("SseSubscribeError handler error: {0:?}")]
    HandlerError(E),
}
impl<E> From<SseConnectionError> for SseSubscribeError<E> {
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
        http::request::RequestBuilder,
        sse::{
            connector::fakes::FakeSseConnector,
            subscriber::fakes::{MockHandler, MockMutHandler},
        },
    };

    use super::*;
    #[test]
    fn handlerの最終結果を取得可能() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        struct StringReturnHandler {
            result: String,
        }
        impl SseMutHandler<String, ()> for StringReturnHandler {
            fn handle(&mut self, res: SseResponse) -> HandleProgress<()> {
                match res {
                    SseResponse::Data(data) => {
                        self.result.push_str(&data);
                        HandleProgress::Progress
                    }
                    _ => todo!(),
                }
            }
            fn result(&self) -> std::result::Result<String, ()> {
                Ok(self.result.clone())
            }
        }
        let mut handler = StringReturnHandler {
            result: String::new(),
        };
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new(&"https://www.fake".try_into().unwrap())
            .get()
            .build();

        let result = sut.subscribe_mut(&request, &mut handler).unwrap();

        assert_eq!(result, "HelloWorld!");
    }
    #[test]
    fn handlerは処理を中断する旨のデータを返却可能() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        let handler = MockHandler::new();
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new(&"https://www.fake".try_into().unwrap())
            .get()
            .build();

        sut.subscribe(&request, &handler).unwrap();

        assert_eq!(sut.connector.connected_times(), 1);
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ])
    }
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
        let request = RequestBuilder::new(&"https://www.fake".try_into().unwrap())
            .get()
            .build();

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
        let request = RequestBuilder::new(&"https://www.fake".try_into().unwrap())
            .get()
            .build();

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
        let request = RequestBuilder::new(&"https://www.fake".try_into().unwrap())
            .get()
            .build();

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

    use super::{HandleProgress, SseHandler, SseMutHandler};
    pub struct MockHandler {
        called: RefCell<usize>,
        events: RefCell<Vec<SseResponse>>,
        returns: RefCell<Vec<()>>,
    }
    impl MockHandler {
        pub fn new() -> Self {
            Self {
                called: RefCell::new(0),
                events: RefCell::new(vec![]),
                returns: RefCell::new(vec![]),
            }
        }
        pub fn called_time(&self) -> usize {
            self.called.borrow().clone()
        }
        pub fn assert_received(&self, expected: &[SseResponse]) {
            assert_eq!(*self.events.borrow(), expected);
        }
    }
    impl SseHandler<(), ()> for MockHandler {
        fn handle(&self, message: SseResponse) -> HandleProgress<()> {
            self.events.borrow_mut().push(message);
            *self.called.borrow_mut() += 1;
            if self.called.borrow().eq(&self.returns.borrow().len()) {
                HandleProgress::Done
            } else {
                HandleProgress::Progress
            }
        }
        fn result(&self) -> std::result::Result<(), ()> {
            Ok(())
        }
    }
    pub struct MockMutHandler {
        called: usize,
        events: Vec<SseResponse>,
        returns: Vec<()>,
    }
    impl MockMutHandler {
        pub fn new() -> Self {
            Self {
                called: 0,
                events: vec![],
                returns: vec![],
            }
        }
        pub fn called_time(&self) -> usize {
            self.called
        }
        pub fn assert_received(&self, expected: &[SseResponse]) {
            assert_eq!(self.events, expected);
        }
    }
    impl SseMutHandler<(), ()> for MockMutHandler {
        fn handle(&mut self, message: SseResponse) -> HandleProgress<()> {
            self.events.push(message);
            self.called += 1;
            if self.called.eq(&self.returns.len()) {
                HandleProgress::Done
            } else {
                HandleProgress::Progress
            }
        }
        fn result(&self) -> std::result::Result<(), ()> {
            Ok(())
        }
    }
}
