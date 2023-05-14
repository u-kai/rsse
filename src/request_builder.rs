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
    pub fn new(url: Url) -> Self {
        Self {
            url,
            method: HttpMethod::Get,
            headers: BTreeMap::new(),
            body: String::new(),
        }
    }
    pub fn post(mut self) -> Self {
        self.method = HttpMethod::Post;
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
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.body = serde_json::to_string(&json).unwrap();
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());
        println!("{}", self.body);
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }
    pub fn to_request(&self) -> String {
        let mut request = String::new();
        match self.method {
            HttpMethod::Get => request.push_str("GET"),
            HttpMethod::Post => request.push_str("POST"),
        }
        request.push_str(" ");
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
        request
    }
}

#[derive(Debug)]
enum HttpMethod {
    Get,
    Post,
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
            "POST /test HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\nContent-Type: application/json\r\n\r\n[1,2,3]"
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
}
