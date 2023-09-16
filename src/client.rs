use crate::{
    http::{request::RequestBuilder, url::Url},
    sse::{
        connector::{SseConnectionError, SseConnector, SseTlsConnector, SseTlsConnectorBuilder},
        response::SseResponse,
        subscriber::{HandleProgress, Result, SseHandler, SseMutHandler, SseSubscriber},
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
    pub fn send_mut_fn<E, F: FnMut(SseResponse) -> HandleProgress<E>>(
        &mut self,
        f: F,
    ) -> Result<(), E> {
        self.subscriber
            .subscribe_mut_fn(&self.req_builder.clone().build(), f)
    }
    pub fn send_fn<E, F: Fn(SseResponse) -> HandleProgress<E>>(&mut self, f: F) -> Result<(), E> {
        self.subscriber
            .subscribe_fn(&self.req_builder.clone().build(), f)
    }
    pub fn send_mut<T, E, H: SseMutHandler<T, E>>(&mut self, handler: &mut H) -> Result<T, E> {
        self.subscriber
            .subscribe_mut(&self.req_builder.clone().build(), handler)
    }
    pub fn post(&mut self) -> &mut Self {
        self.req_builder.post();
        self
    }
    pub fn bearer_auth(&mut self, token: &str) -> &mut Self {
        self.req_builder.bearer_auth(token);
        self
    }
    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.req_builder.header(key, value);
        self
    }
    pub fn get(&mut self) -> &mut Self {
        self.req_builder.get();
        self
    }
    pub fn json<S: serde::Serialize>(&mut self, json: S) -> &mut Self {
        self.req_builder.json(json);
        self
    }
}

pub struct SseClientBuilder<C: SseConnector> {
    url: Url,
    connector: Option<C>,
    req_builder: Option<RequestBuilder>,
}
impl SseClientBuilder<SseTlsConnector> {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder<SseTlsConnector> {
        let url = url.into();
        SseClientBuilder {
            url: url.clone(),
            connector: Some(SseTlsConnectorBuilder::new(&url).build().unwrap()),
            req_builder: Some(RequestBuilder::new(&url)),
        }
    }
}

impl<C: SseConnector> SseClientBuilder<C> {
    pub fn set_connector<NewC>(self, connector: NewC) -> SseClientBuilder<NewC>
    where
        NewC: SseConnector,
    {
        SseClientBuilder {
            connector: Some(connector),
            url: self.url,
            req_builder: self.req_builder,
        }
    }
    pub fn proxy(
        &mut self,
        proxy: &Url,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, SseConnectionError> {
        let mut builder = SseTlsConnectorBuilder::new(&self.url);
        let connector = builder.proxy(proxy).build()?;

        Ok(SseClientBuilder {
            url: self.url.clone(),
            connector: Some(connector),
            req_builder: self.req_builder.take(),
        })
    }
    pub fn add_ca(
        &mut self,
        ca: &str,
    ) -> std::result::Result<SseClientBuilder<SseTlsConnector>, SseConnectionError> {
        let connector = SseTlsConnectorBuilder::new(&self.url).add_ca(ca).build()?;
        Ok(SseClientBuilder {
            url: self.url.clone(),
            connector: Some(connector),
            req_builder: self.req_builder.take(),
        })
    }
    pub fn build(&mut self) -> SseClient<C> {
        SseClient {
            subscriber: SseSubscriber::new(self.connector.take().unwrap()),
            req_builder: self.req_builder.take().unwrap(),
        }
    }
    pub fn post(&mut self) -> &mut Self {
        self.req_builder.as_mut().map(|r| r.post());
        self
    }
    pub fn json<S: serde::Serialize>(&mut self, json: S) -> &mut Self {
        self.req_builder.as_mut().map(|r| r.json(json));
        self
    }
    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.req_builder.as_mut().map(|r| r.header(key, value));
        self
    }
    pub fn bearer_auth(&mut self, token: &str) -> &mut Self {
        self.req_builder.as_mut().map(|r| r.bearer_auth(token));
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
            subscriber::HandleProgress,
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
    fn chatgptにfnを登録して通信する() {
        let mut store = Vec::new();
        let req = RequestBuilder::new(&URL.try_into().unwrap()).build();
        let mut sut = SseClientBuilder::new(req.url())
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();

        sut.send_mut_fn(|res| match res {
            SseResponse::Data(data) => {
                if data.contains("[DONE]") {
                    return HandleProgress::Done;
                }
                store.push(data);
                HandleProgress::<String>::Progress
            }
            _ => HandleProgress::Progress,
        })
        .unwrap();
        println!("gpt > {:#?}", store);
        assert!(store.len() > 0);
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

        let mut builder =
            SseClientBuilder::new(&"http://fake.com".try_into().unwrap()).set_connector(connector);
        let mut sut = builder.post().json(r#"{"name":"John"}"#).build();

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
