use crate::{
    request::{Request, RequestBuilder},
    sse::{
        connector::{SseConnectionError, SseConnector, SseTlsConnector, SseTlsConnectorBuilder},
        subscriber::{Result, SseHandler, SseMutHandler, SseSubscribeError, SseSubscriber},
    },
    url::Url,
};

#[derive(Debug)]
pub struct SseClient<C: SseConnector> {
    req: Request,
    subscriber: SseSubscriber<C>,
}
pub struct SseClientBuilder<C: SseConnector> {
    url: Url,
    req: RequestBuilder,
    connector: C,
}
impl SseClientBuilder<SseTlsConnector> {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder<SseTlsConnector> {
        let url = url.into();
        SseClientBuilder {
            url: url.clone(),
            req: RequestBuilder::new(url.clone()),
            connector: SseTlsConnectorBuilder::new(url).build().unwrap(),
        }
    }
}

impl<C: SseConnector> SseClientBuilder<C> {
    pub fn set_connector<NewC>(self, connector: NewC) -> SseClientBuilder<NewC>
    where
        NewC: SseConnector,
    {
        SseClientBuilder {
            url: self.url,
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
    pub fn proxy(
        self,
        proxy: &str,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, Box<dyn std::error::Error + 'static>>
    {
        let connector = SseTlsConnectorBuilder::new(&self.url)
            .proxy(proxy)
            .build()?;

        Ok(SseClientBuilder {
            url: self.url,
            req: self.req,
            connector,
        })
    }
    pub fn add_ca(
        self,
        ca: &str,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, Box<dyn std::error::Error + 'static>>
    {
        let connector = SseTlsConnectorBuilder::new(&self.url).add_ca(ca).build()?;
        Ok(SseClientBuilder {
            url: self.url,
            req: self.req,
            connector,
        })
    }
    pub fn build(self) -> SseClient<C> {
        SseClient {
            req: self.req.build(),
            subscriber: SseSubscriber::new(self.connector),
        }
    }
}
impl<C: SseConnector> SseClient<C> {
    pub fn send<T, E, H: SseHandler<T, E>>(&mut self, handler: &H) -> Result<T, E> {
        self.subscriber.subscribe(&self.req, handler)
    }
    pub fn send_mut<T, E, H: SseMutHandler<T, E>>(&mut self, handler: &mut H) -> Result<T, E> {
        self.subscriber.subscribe_mut(&self.req, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::{
        self,
        connector::{
            chatgpt::{chatgpt_key, message, ChatGptRes, GptHandler, URL},
            fakes::FakeSseConnector,
            ConnectedSseResponse,
        },
        response::SseResponse,
    };

    #[test]
    #[ignore = "dockerによるproxyが必要のため"]
    fn proxyに対して通信可能() {
        let mut gpt_handler = GptHandler::new();
        let mut sut = SseClientBuilder::new(URL)
            .post()
            .json(message("Hello"))
            .proxy("http://localhost:8080")
            .unwrap()
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
        let mut sut = SseClientBuilder::new(URL)
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
