use std::collections::HashMap;

pub struct HttpResponseBuilder {
    status_code: u32,
    headers: HashMap<String, String>,
    body: String,
    is_next_body: bool,
    is_next_header: bool,
}

impl HttpResponseBuilder {
    pub fn new() -> Self {
        Self {
            status_code: 0,
            headers: HashMap::new(),
            body: String::new(),
            is_next_body: false,
            is_next_header: false,
        }
    }
    pub fn add_line(&mut self, line: &str) {
        if self.is_start_line() && line.starts_with("HTTP/") {
            self.status_code = line.split(" ").nth(1).unwrap().parse().unwrap();
            self.is_next_header = true;
            return;
        }
        if self.is_next_header && line.contains(":") {
            let mut iter = line.split(":");
            let key = iter.next().unwrap().trim().to_string();
            let value = iter.next().unwrap().trim().to_string();
            self.headers.insert(key, value);
            return;
        }
        if line.is_empty() || line == "\r" {
            self.is_next_body = true;
            self.is_next_header = false;
            return;
        }
        if self.is_next_body {
            self.body.push_str(line);
        }
    }
    pub fn build(self) -> HttpResponse {
        HttpResponse {
            status_code: self.status_code,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
    fn is_start_line(&self) -> bool {
        self.status_code == 0 && self.headers.is_empty()
    }
}
pub struct HttpResponse {
    status_code: u32,
    headers: HashMap<String, String>,
    body: String,
}
impl HttpResponse {
    pub fn status_code(&self) -> u32 {
        self.status_code
    }
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|v| v.as_str())
    }
    pub fn body(&self) -> &str {
        self.body.as_str()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 連続的な行からhttp_responseを構築可能() {
        let mut http_response = HttpResponseBuilder::new();
        http_response.add_line("HTTP/1.1 200 OK");
        http_response.add_line("Content-Type: text/event-stream");
        http_response.add_line("");
        http_response.add_line("start\n");
        http_response.add_line("data: 1\n");
        http_response.add_line("data: 2\n");
        let http_response = http_response.build();
        assert_eq!(http_response.status_code(), 200);
        assert_eq!(
            http_response.get_header("Content-Type").unwrap(),
            "text/event-stream"
        );
        assert_eq!(http_response.body(), "start\ndata: 1\ndata: 2\n");
    }
}
