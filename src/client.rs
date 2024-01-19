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
    pub fn post(mut self) -> Self {
        let new_req_builder = self.req_builder.post();
        self.req_builder = new_req_builder;
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        let new_req_builder = self.req_builder.bearer_auth(token);
        self.req_builder = new_req_builder;
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        let new_req_builder = self.req_builder.header(key, value);
        self.req_builder = new_req_builder;
        self
    }
    pub fn get(mut self) -> Self {
        let new_req_builder = self.req_builder.get();
        self.req_builder = new_req_builder;
        self
    }
    pub fn json<S: serde::Serialize>(mut self, json: S) -> Self {
        let new_req_builder = self.req_builder.json(json);
        self.req_builder = new_req_builder;
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
            req_builder: self.req_builder,
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
    use crate::{
        http::request::RequestBuilder,
        sse::{
            connector::chatgpt::{chatgpt_key, message, GptHandler, URL},
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
}
