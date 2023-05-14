#[derive(Debug)]
pub struct Url {
    scheme: Schema,
    host: String,
    port: u16,
    path: Option<String>,
}

impl Url {
    pub fn from_str(s: &str) -> Result<Self> {
        let mut split = s.split("://");
        let Some(schema) = split.next() else {
            return  Err(UrlError::InvalidString(s.to_string()));
        };
        let schema = Schema::from_str(schema)?;
        let Some(host_and_maybe_path_and_maybe_port) = split.next() else {
            return Err(UrlError::InvalidString(s.to_string()));
        };
        let mut host_and_maybe_path_and_maybe_port = host_and_maybe_path_and_maybe_port.split(":");
        let Some(host_and_maybe_path) = host_and_maybe_path_and_maybe_port.next() else {
            return Err(UrlError::InvalidString(s.to_string()));
        };
        let port = host_and_maybe_path_and_maybe_port
            .next()
            .map(|s| s.parse::<u16>().unwrap_or(schema.port()))
            .unwrap_or(schema.port());
        let mut host_and_maybe_path = host_and_maybe_path.split("/");
        let Some(host) = host_and_maybe_path.next() else {
            return Err(UrlError::InvalidString(s.to_string()));
        };
        Ok(Self {
            scheme: schema,
            host: host.to_string(),
            port,
            path: host_and_maybe_path.next().map(|s| s.to_string()),
        })
    }
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(self.scheme());
        s.push_str("://");
        s.push_str(self.host());
        if self.port() != self.scheme.port() {
            s.push_str(":");
            s.push_str(&self.port().to_string());
        }
        s.push_str(self.path().as_str());
        s
    }
    pub fn scheme(&self) -> &str {
        self.scheme.to_str()
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn path(&self) -> String {
        format!(
            "/{}",
            self.path.as_ref().map(|s| s.as_str()).unwrap_or_default()
        )
    }
}

#[derive(Debug)]
enum Schema {
    Http,
    Https,
}

impl Schema {
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "http" => Ok(Schema::Http),
            "https" => Ok(Schema::Https),
            _ => Err(UrlError::InvalidSchema(s.to_string())),
        }
    }
    fn port(&self) -> u16 {
        match self {
            Schema::Http => 80,
            Schema::Https => 443,
        }
    }
    fn to_str(&self) -> &str {
        match self {
            Schema::Http => "http",
            Schema::Https => "https",
        }
    }
}

pub type Result<T> = std::result::Result<T, UrlError>;
#[derive(Debug)]
pub enum UrlError {
    InvalidSchema(String),
    InvalidString(String),
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url構造体はurlの文字列から作成できる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "localhost");
        assert_eq!(url.path(), "/test");
        assert_eq!(url.port(), 443);
        assert_eq!(url.to_string(), "https://localhost/test");
    }
    #[test]
    fn url構造体はpathを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.path(), "/test");
        let url = Url::from_str("https://localhost").unwrap();
        assert_eq!(url.path(), "/");
    }
    #[test]
    fn url構造体はhostを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.host(), "localhost");
    }
    #[test]
    fn url構造体はスキーマを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.scheme(), "https");
        let url = Url::from_str("http://localhost/test").unwrap();
        assert_eq!(url.scheme(), "http");
    }
    #[test]
    fn url構造体はportを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.port(), 443);
        let url = Url::from_str("http://localhost/test:10000").unwrap();
        assert_eq!(url.port(), 10000);
    }
}
