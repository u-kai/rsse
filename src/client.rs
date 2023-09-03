use crate::{
    request::{Request, RequestBuilder},
    sse::{
        connector::{SseConnector, SseTlsConnector},
        subscriber::{SseHandler, SseMutHandler, SseSubscriber},
    },
    url::Url,
};

#[derive(Debug)]
pub struct SseClient<C: SseConnector> {
    req: Request,
    subscriber: SseSubscriber<C>,
}
pub struct SseClientBuilder<C: SseConnector> {
    req: RequestBuilder,
    connector: C,
}
impl SseClientBuilder<SseTlsConnector> {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder<SseTlsConnector> {
        SseClientBuilder {
            req: RequestBuilder::new(url),
            connector: SseTlsConnector::new(),
        }
    }
}

impl<C: SseConnector> SseClientBuilder<C> {
    pub fn set_connector<NewC>(mut self, connector: NewC) -> SseClientBuilder<NewC>
    where
        NewC: SseConnector,
    {
        SseClientBuilder {
            req: self.req,
            connector,
        }
    }
    pub fn post(mut self) -> Self {
        self.req = self.req.post();
        self
    }
    pub fn get(mut self) -> Self {
        self.req = self.req.get();
        self
    }
    pub fn json<S: serde::Serialize>(mut self, json: S) -> Self {
        self.req = self.req.json(json);
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.req = self.req.header(key, value);
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.req = self.req.bearer_auth(token);
        self
    }
    pub fn build(self) -> SseClient<C> {
        SseClient {
            req: self.req.build(),
            subscriber: SseSubscriber::new(self.connector),
        }
    }
}
impl<C: SseConnector> SseClient<C> {
    pub fn send<T: SseHandler>(
        &mut self,
        handler: &T,
    ) -> Result<(), crate::sse::subscriber::SseSubscribeError> {
        self.subscriber.subscribe(&self.req, handler)
    }
    pub fn send_mut<T: SseMutHandler>(
        &mut self,
        handler: &mut T,
    ) -> Result<(), crate::sse::subscriber::SseSubscribeError> {
        self.subscriber.subscribe_mut(&self.req, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::{self, connector::fakes::FakeSseConnector, response::SseResponse};

    #[test]
    fn ビルダーパターンでsseのサブスクリプションを行う() {
        let handler = sse::subscriber::fakes::MockHandler::new();
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");
        let mut sut = SseClientBuilder::new("http://fake.com")
            .post()
            .set_connector(connector)
            .json(r#"{"name":"John"}"#)
            .build();

        let result = sut.send(&handler);

        assert!(result.is_ok());
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ])
    }
}
