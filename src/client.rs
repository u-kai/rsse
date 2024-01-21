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
    // always Some
    // Reason of Option, we need to take ownership of the RequestBuilder
    req_builder: Option<RequestBuilder>,
}
impl<C: SseConnector> SseClient<C> {
    pub fn send<T, E, H: SseHandler<T, E>>(&mut self, handler: &H) -> Result<T, E> {
        let req = self.req_builder.take().unwrap().build();
        self.req_builder = Some(RequestBuilder::new(req.url()));
        self.subscriber.subscribe(&req, handler)
    }
    pub fn send_mut_fn<E, F: FnMut(SseResponse) -> HandleProgress<E>>(
        &mut self,
        f: F,
    ) -> Result<(), E> {
        let req = self.req_builder.take().unwrap().build();
        self.req_builder = Some(RequestBuilder::new(req.url()));
        self.subscriber.subscribe_mut_fn(&req, f)
    }
    pub fn send_fn<E, F: Fn(SseResponse) -> HandleProgress<E>>(&mut self, f: F) -> Result<(), E> {
        let req = self.req_builder.take().unwrap().build();
        self.req_builder = Some(RequestBuilder::new(req.url()));
        self.subscriber.subscribe_fn(&req, f)
    }
    pub fn send_mut<T, E, H: SseMutHandler<T, E>>(&mut self, handler: &mut H) -> Result<T, E> {
        let req = self.req_builder.take().unwrap().build();
        self.req_builder = Some(RequestBuilder::new(req.url()));
        self.subscriber.subscribe_mut(&req, handler)
    }
    pub fn post(&mut self) -> &mut Self {
        self.req_builder = Some(self.req_builder.take().unwrap().post());
        self
    }
    pub fn bearer_auth(&mut self, token: &str) -> &mut Self {
        self.req_builder = Some(self.req_builder.take().unwrap().bearer_auth(token));
        self
    }
    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.req_builder = Some(self.req_builder.take().unwrap().header(key, value));
        self
    }
    pub fn get(&mut self) -> &mut Self {
        self.req_builder = Some(self.req_builder.take().unwrap().get());
        self
    }
    pub fn json<S: serde::Serialize>(&mut self, json: S) -> &mut Self {
        self.req_builder = Some(self.req_builder.take().unwrap().json(json));
        self
    }
}

pub struct SseClientBuilder {
    url: Url,
    connector_builder: SseTlsConnectorBuilder,
    req_builder: RequestBuilder,
}
impl SseClientBuilder {
    pub fn new(url: impl Into<Url>) -> SseClientBuilder {
        let url = url.into();
        SseClientBuilder {
            url: url.clone(),
            connector_builder: SseTlsConnectorBuilder::new(&url),
            req_builder: RequestBuilder::new(&url),
        }
    }
}

impl SseClientBuilder {
    pub fn proxy(self, proxy: &Url) -> std::result::Result<SseClientBuilder, SseConnectionError> {
        let connector_builder = self.connector_builder.proxy(proxy);

        Ok(SseClientBuilder {
            url: self.url.clone(),
            connector_builder,
            req_builder: self.req_builder,
        })
    }
    pub fn add_ca(self, ca: &str) -> std::result::Result<SseClientBuilder, SseConnectionError> {
        let connector_builder = self.connector_builder.add_ca(ca);
        Ok(SseClientBuilder {
            url: self.url.clone(),
            connector_builder,
            req_builder: self.req_builder,
        })
    }
    pub fn build(self) -> SseClient<SseTlsConnector> {
        SseClient {
            subscriber: SseSubscriber::new(self.connector_builder.build().unwrap()),
            req_builder: Some(self.req_builder),
        }
    }
    pub fn post(mut self) -> Self {
        let new_req_builder = self.req_builder.post();
        self.req_builder = new_req_builder;
        self
    }
    pub fn json<S: serde::Serialize>(mut self, json: S) -> Self {
        let new_req_builder = self.req_builder.json(json);
        self.req_builder = new_req_builder;
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        let new_req_builder = self.req_builder.header(key, value);
        self.req_builder = new_req_builder;
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        let new_req_builder = self.req_builder.bearer_auth(token);
        self.req_builder = new_req_builder;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sse::{
        connector::chatgpt::{chatgpt_key, message, GptHandler, URL},
        response::SseResponse,
        subscriber::HandleProgress,
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
    fn clientは何度でも利用可能() {
        let mut store = Vec::new();
        let mut sut = SseClientBuilder::new(&URL.try_into().unwrap())
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
        sut.post()
            .json(message("なんで地球は丸いの?"))
            .bearer_auth(&chatgpt_key())
            .send_mut_fn(|res| match res {
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
    fn chatgptにfnを登録して通信する() {
        let mut store = Vec::new();
        let mut sut = SseClientBuilder::new(&URL.try_into().unwrap())
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
        let mut sut = SseClientBuilder::new(&URL.try_into().unwrap())
            .post()
            .json(message("Hello"))
            .bearer_auth(&chatgpt_key())
            .build();

        let result = sut.send_mut(&mut gpt_handler).unwrap();

        println!("gpt > {:?}", result);
        assert!(result.len() > 0);
        assert!(gpt_handler.is_success());
    }
}
