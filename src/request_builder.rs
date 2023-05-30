use std::collections::BTreeMap;

use crate::url::Url;

#[derive(Debug)]
pub struct RequestBuilder {
    url: Url,
    method: HttpMethod,
    headers: BTreeMap<String, String>,
    body: String,
}
impl RequestBuilder {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Get,
            headers: BTreeMap::new(),
            body: String::new(),
        }
    }
    pub fn post(mut self) -> Self {
        self.method = HttpMethod::Post;
        self
    }
    pub fn connect(mut self) -> Self {
        self.method = HttpMethod::Connect;
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
    fn header_string(&self) -> String {
        self.headers
            .iter()
            .fold(String::new(), |mut acc, (key, value)| {
                acc.push_str(key);
                acc.push_str(": ");
                acc.push_str(value);
                acc.push_str("\r\n");
                acc
            })
    }
    pub fn json<T: serde::Serialize>(mut self, json: T) -> Self {
        self = self.header("Content-Type", "application/json");
        self.body = serde_json::to_string(&json).unwrap();
        let len = self.body.len();
        self = self.header("Content-Length", len.to_string().as_str());
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }
    pub fn build(self) -> Request {
        Request(self.to_request())
    }
    fn to_request(&self) -> String {
        let mut request = String::new();
        request.push_str(self.method.to_str());
        request.push_str(" ");
        match self.method {
            HttpMethod::Get => {}
            HttpMethod::Post => {
                request.push_str(self.url.path());
                request.push_str(" HTTP/1.1\r\n");
                request.push_str("Host: ");
                request.push_str(self.url.host());
                request.push_str("\r\n");
                request.push_str("Accept: text/event-stream\r\n");
                request.push_str("Connection: keep-alive\r\n");
                request.push_str(self.header_string().as_str());
                request.push_str("\r\n");
                request.push_str(self.body.as_str());
            }
            HttpMethod::Connect => {
                request.push_str(self.url.host());
                request.push_str(&format!(":{}", self.url.port()));
                request.push_str(" HTTP/1.1\r\n");
                request.push_str("Host: ");
                request.push_str(self.url.host());
                request.push_str(&format!(":{}", self.url.port()));
                request.push_str("\r\n");
            }
        }
        request
    }
}
#[derive(Debug, Clone)]
pub struct Request(String);
impl Request {
    pub fn bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Debug)]
enum HttpMethod {
    Get,
    Post,
    Connect,
}
impl HttpMethod {
    fn to_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Connect => "CONNECT",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::url::Url;

    use super::*;
    #[test]
    fn bodyにjsonを追加できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        let request = RequestBuilder::new(url)
            .post()
            .json(vec![1, 2, 3])
            .to_request();
        assert_eq!(
            request,
            "POST /test HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\nContent-Length: 7\r\nContent-Type: application/json\r\n\r\n[1,2,3]"
        )
    }
    #[test]
    fn bearer_authを追加できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        let request = RequestBuilder::new(url)
            .post()
            .bearer_auth("token")
            .to_request();
        assert_eq!(
            request,
            "POST /test HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\nAuthorization: Bearer token\r\n\r\n"
        )
    }
    #[test]
    fn headerを追加できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        let request = RequestBuilder::new(url)
            .post()
            .header("Content-Type", "application/json")
            .to_request();
        assert_eq!(
            request,
            "POST /test HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\nContent-Type: application/json\r\n\r\n"
        )
    }
    #[test]
    fn urlからpostのrequestを作成できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        let request = RequestBuilder::new(url).post().to_request();
        assert_eq!(
            request,
            "POST /test HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\n\r\n"
        )
    }
    #[test]
    fn urlからconnectのrequestを作成できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        let request = RequestBuilder::new(url).connect().to_request();
        assert_eq!(
            request,
            "CONNECT localhost:443 HTTP/1.1\r\nHost: localhost:443\r\n"
        )
    }
}
