use crate::request::Request;

use super::connector::{SseConnection, SseConnectionError, SseConnector, SseResponse};

pub type Result<T> = std::result::Result<T, SseSubscribeError>;
trait SseMutHandler {
    fn handle(&mut self, buf: &str);
}
struct SseSubscriber<C: SseConnection, T: SseConnector<Connection = C>> {
    connector: T,
}
impl<C: SseConnection, T: SseConnector<Connection = C>> SseSubscriber<C, T> {
    fn new(connector: T) -> Self {
        Self { connector }
    }
    fn subscribe_mut(&mut self, req: &Request, handler: &mut impl SseMutHandler) -> Result<()> {
        let connection = self
            .connector
            .connect(req)
            .map_err(SseSubscribeError::from)?;
        loop {
            match connection.consume() {
                Ok(res) => match res {
                    SseResponse::Data(data) => {
                        handler.handle(data.as_str());
                    }
                    SseResponse::Done => {
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
pub struct SseSubscribeError {
    message: String,
}
impl SseSubscribeError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for SseSubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SseSubscribeError: {}", self.message)
    }
}
impl std::error::Error for SseSubscribeError {}
impl From<SseConnectionError> for SseSubscribeError {
    fn from(err: SseConnectionError) -> Self {
        Self::new(&err.to_string())
    }
}
#[cfg(test)]
mod tests {

    use crate::{request::RequestBuilder, sse::connector::fakes::FakeSseConnector};

    use super::*;
    //#[test]
    //fn sse_のhttp接続に失敗する() {
    //let mut connector = FakeSseConnector::new();
    //connector.set_http_failure_sse(400);
    //let mut handler = MockHandler::new();
    //let mut sut = SseSubscriber::new(connector);
    //let request = RequestBuilder::new("https://www.fake").get().build();

    //let result = sut.subscribe_mut(&request, &mut handler);

    //let Err(result) = result else {
    //panic!("expected Err, but got Ok");
    //};
    //assert!(result);
    //assert_eq!(sut.connector.connected_time(), 1);
    //assert_eq!(handler.called_time(), 0);
    //}
    #[test]
    fn sseのデータを捕捉する() {
        let mut connector = FakeSseConnector::new();
        let response = "hello world";
        connector.set_success_sse(response);
        let mut handler = MockHandler::new();
        let mut sut = SseSubscriber::new(connector);
        let request = RequestBuilder::new("https://www.fake").get().build();

        sut.subscribe_mut(&request, &mut handler).unwrap();
        assert_eq!(sut.connector.connected_time(), 1);
        assert_eq!(handler.called_time(), response.len());
    }
    struct MockHandler {
        called: usize,
    }
    impl MockHandler {
        fn new() -> Self {
            Self { called: 0 }
        }
        fn called_time(&self) -> usize {
            self.called
        }
    }
    impl SseMutHandler for MockHandler {
        fn handle(&mut self, _message: &str) {
            self.called += 1;
        }
    }
}
