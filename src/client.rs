use crate::{
    http::{request::RequestBuilder, url::Url},
    sse::{
        connector::{SseConnectionError, SseConnector, SseTlsConnector, SseTlsConnectorBuilder},
        subscriber::{Result, SseHandler, SseMutHandler, SseSubscriber},
    },
};

#[derive(Debug)]
pub struct SseClient<C: SseConnector> {
    subscriber: SseSubscriber<C>,
    req_builder: RequestBuilder,
}
impl<C: SseConnector> SseClient<C> {
    pub fn send<T, E, H: SseHandler<T, E>>(&mut self, handler: &H) -> Result<T, E> {
        self.subscriber
            .subscribe(&self.req_builder.clone().build(), handler)
    }
    pub fn send_mut<T, E, H: SseMutHandler<T, E>>(&mut self, handler: &mut H) -> Result<T, E> {
        self.subscriber
            .subscribe_mut(&self.req_builder.clone().build(), handler)
    }
    pub fn post(mut self) -> Self {
        self.req_builder = self.req_builder.post();
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.req_builder = self.req_builder.bearer_auth(token);
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.req_builder = self.req_builder.header(key, value);
        self
    }
    pub fn get(mut self) -> Self {
        self.req_builder = self.req_builder.get();
        self
    }
    pub fn json<S: serde::Serialize>(mut self, json: S) -> Self {
        self.req_builder = self.req_builder.json(json);
        self
    }
}

pub struct SseClientBuilder<C: SseConnector> {
    url: Url,
    connector: C,
    req_builder: RequestBuilder,
}
impl SseClientBuilder<SseTlsConnector> {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder<SseTlsConnector> {
        let url = url.into();
        SseClientBuilder {
            url: url.clone(),
            connector: SseTlsConnectorBuilder::new(&url).build().unwrap(),
            req_builder: RequestBuilder::new(&url),
        }
    }
}

impl<C: SseConnector> SseClientBuilder<C> {
    pub fn set_connector<NewC>(self, connector: NewC) -> SseClientBuilder<NewC>
    where
        NewC: SseConnector,
    {
        SseClientBuilder {
            connector,
            url: self.url,
            req_builder: self.req_builder,
        }
    }
    pub fn proxy(
        self,
        proxy: &Url,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, SseConnectionError> {
        let connector = SseTlsConnectorBuilder::new(&self.url)
            .proxy(proxy)
            .build()?;

        Ok(SseClientBuilder {
            url: self.url,
            connector,
            req_builder: self.req_builder,
        })
    }
    pub fn add_ca(
        self,
        ca: &str,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, SseConnectionError> {
        let connector = SseTlsConnectorBuilder::new(&self.url).add_ca(ca).build()?;
        Ok(SseClientBuilder {
            url: self.url,
            connector,
            req_builder: self.req_builder,
        })
    }
    pub fn build(self) -> SseClient<C> {
        SseClient {
            subscriber: SseSubscriber::new(self.connector),
            req_builder: self.req_builder,
        }
    }
    pub fn post(mut self) -> Self {
        self.req_builder = self.req_builder.post();
        self
    }
    pub fn json<S: serde::Serialize>(mut self, json: S) -> Self {
        self.req_builder = self.req_builder.json(json);
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.req_builder = self.req_builder.header(key, value);
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.req_builder = self.req_builder.bearer_auth(token);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        http::request::RequestBuilder,
        sse::{
            self,
            connector::{
                chatgpt::{chatgpt_key, message, GptHandler, URL},
                fakes::FakeSseConnector,
            },
            response::SseResponse,
        },
    };

    #[test]
    #[ignore = "dockerによるproxyが必要のため"]
    fn proxyに対して通信可能() {
        let mut gpt_handler = GptHandler::new();
        let mut sut = SseClientBuilder::new(&URL.try_into().unwrap())
            .proxy(&"http://localhost:8080".try_into().unwrap())
            .unwrap()
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();

        let result = sut.send_mut(&mut gpt_handler).unwrap();

        println!("gpt > {:?}", result);
        assert!(result.len() > 0);
        assert!(gpt_handler.is_success());
    }
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn chatgptに通信する() {
        let mut gpt_handler = GptHandler::new();
        let req = RequestBuilder::new(&URL.try_into().unwrap()).build();
        let mut sut = SseClientBuilder::new(req.url())
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();

        let result = sut.send_mut(&mut gpt_handler).unwrap();

        println!("gpt > {:?}", result);
        assert!(result.len() > 0);
        assert!(gpt_handler.is_success());
    }
    #[test]
    fn ビルダーパターンでsseのサブスクリプションを行う() {
        let handler = sse::subscriber::fakes::MockHandler::new();
        let mut connector = FakeSseConnector::new();
        connector.set_response("HTTP/1.1 200 OK\r\n");
        connector.set_response("Content-Type: text/event-stream\r\n");
        connector.set_response("\r\n\r\n");
        connector.set_response("data: Hello\r\n");
        connector.set_response("data: World!\r\n");

        let mut sut = SseClientBuilder::new(&"http://fake.com".try_into().unwrap())
            .post()
            .json(r#"{"name":"John"}"#)
            .set_connector(connector)
            .build();

        let result = sut.send(&handler);

        assert!(result.is_ok());
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ]);

        let result = sut.post().json(r#"{"name":"John"}"#).send(&handler);
        assert!(result.is_ok());
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ]);
    }
}
