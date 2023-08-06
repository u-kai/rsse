use crate::{http::response::HttpResponse, request::Request};

use super::{
    connector::{ConnectedSseResponse, Socket, SseConnection, SseConnectionError, SseConnector},
    response::SseResponse,
};

pub type Result<T> = std::result::Result<T, SseSubscribeError>;
pub trait SseMutHandler {
    fn handle(&mut self, res: SseResponse);
}
pub struct SseSubscriber<S: Socket, T: SseConnector<S>> {
    connector: T,
    _phantom: std::marker::PhantomData<S>,
}
impl<S: Socket, T: SseConnector<S>> SseSubscriber<S, T> {
    pub fn new(connector: T) -> Self {
        Self {
            connector,
            _phantom: std::marker::PhantomData,
        }
    }
    pub fn subscribe_mut(&mut self, req: &Request, handler: &mut impl SseMutHandler) -> Result<()> {
        let connection: &mut SseConnection<S> = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            match connection.consume() {
                Ok(res) => match res {
                    ConnectedSseResponse::Progress(sse_response) => {
                        handler.handle(sse_response);
                    }
                    ConnectedSseResponse::Done => {
                        return Ok(());
                    }
                },
                Err(e) => {
                    return Err(SseSubscribeError::from(e));
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
            //SseConnectionError::HttpError(err) => Self::HttpError(err),
            _ => Self::ConnectionError(err),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{request::RequestBuilder, sse::connector::fakes::FakeSseConnector};

    use super::*;
    #[test]
    fn sseのデータを捕捉する() {
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        let mut handler = MockHandler::new();
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
    //#[test]
    //fn sseのhttp接続に失敗する() {
    //let mut connector = FakeSseConnector::new();
    //let status_line = HttpStatusLine::from_str("HTTP/1.1 400 Bad Request").unwrap();
    //let header = HttpHeader::from_line("Retry-After: 3600").unwrap();
    //let body_str = "your request is bad request";
    //let body = HttpBody::from_line(body_str);
    //connector.set_http_response(HttpResponse::new(status_line, header, body));
    //let mut handler = MockHandler::new();
    //let mut sut = SseSubscriber::new(connector);
    //let request = RequestBuilder::new("https://www.fake").get().build();

    //let result = sut.subscribe_mut(&request, &mut handler);
    //let Err(SseSubscribeError::HttpError(err)) = result else {
    //panic!("expected Err, but got Ok");
    //};

    //assert_eq!(err.status_code(), 400);
    //assert_eq!(err.get_header("Retry-After"), Some("3600"));
    //assert_eq!(err.body_str(), body_str);
    //assert_eq!(sut.connector.connected_time(), 1);
    //assert_eq!(handler.called_time(), 0);
    //}
    struct MockHandler {
        called: usize,
        events: Vec<SseResponse>,
    }
    impl MockHandler {
        fn new() -> Self {
            Self {
                called: 0,
                events: vec![],
            }
        }
        fn called_time(&self) -> usize {
            self.called
        }
        fn assert_received(&self, expected: &[SseResponse]) {
            assert_eq!(self.events, expected);
        }
    }
    impl SseMutHandler for MockHandler {
        fn handle(&mut self, message: SseResponse) {
            self.events.push(message);
            self.called += 1;
        }
    }
}
