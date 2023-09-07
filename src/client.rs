use crate::{
    http::request::Request,
    sse::{
        connector::{SseConnectionError, SseConnector, SseTlsConnector, SseTlsConnectorBuilder},
        subscriber::{Result, SseHandler, SseMutHandler, SseSubscriber},
    },
    url::Url,
};

#[derive(Debug)]
pub struct SseClient<C: SseConnector> {
    subscriber: SseSubscriber<C>,
}
pub struct SseClientBuilder<C: SseConnector> {
    url: Url,
    connector: C,
}
impl SseClientBuilder<SseTlsConnector> {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder<SseTlsConnector> {
        let url = url.into();
        SseClientBuilder {
            url: url.clone(),
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
            connector,
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
        })
    }
    pub fn build(self) -> SseClient<C> {
        SseClient {
            subscriber: SseSubscriber::new(self.connector),
        }
    }
}

impl<C: SseConnector> SseClient<C> {
    pub fn send<T, E, H: SseHandler<T, E>>(&mut self, req: &Request, handler: &H) -> Result<T, E> {
        self.subscriber.subscribe(req, handler)
    }
    pub fn send_mut<T, E, H: SseMutHandler<T, E>>(
        &mut self,
        req: &Request,
        handler: &mut H,
    ) -> Result<T, E> {
        self.subscriber.subscribe_mut(req, handler)
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
        let req = RequestBuilder::new(&URL.try_into().unwrap())
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();
        let mut sut = SseClientBuilder::new(req.url())
            .proxy(&"http://localhost:8080".try_into().unwrap())
            .unwrap()
            .build();

        let result = sut.send_mut(&req, &mut gpt_handler).unwrap();

        println!("gpt > {:?}", result);
        assert!(result.len() > 0);
        assert!(gpt_handler.is_success());
    }
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn chatgptに通信する() {
        let mut gpt_handler = GptHandler::new();
        let req = RequestBuilder::new(&URL.try_into().unwrap())
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();
        let mut sut = SseClientBuilder::new(req.url()).build();

        let result = sut.send_mut(&req, &mut gpt_handler).unwrap();

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

        let req = RequestBuilder::new(&"http://fake.com".try_into().unwrap())
            .post()
            .json(r#"{"name":"John"}"#)
            .build();
        let mut sut = SseClientBuilder::new(&"http://fake.com".try_into().unwrap())
            .set_connector(connector)
            .build();

        let result = sut.send(&req, &handler);

        assert!(result.is_ok());
        assert_eq!(handler.called_time(), 2);
        handler.assert_received(&[
            SseResponse::Data("Hello".to_string()),
            SseResponse::Data("World!".to_string()),
        ])
    }
}
