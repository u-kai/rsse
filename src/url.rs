#[derive(Debug, Clone)]
pub struct Url {
    scheme: Schema,
    host: String,
    port: u16,
    path: String,
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
        let mut path = host_and_maybe_path.fold(String::new(), |mut acc, s| {
            acc.push_str("/");
            acc.push_str(s);
            acc
        });
        if path.len() == 0 {
            path.push_str("/");
        };
        Ok(Self {
            scheme: schema,
            host: host.to_string(),
            port,
            path,
        })
    }
    pub fn to_addr_str(&self) -> String {
        format!("{}:{}", self.host(), self.port())
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
        s.push_str(self.path());
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
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}
impl Into<Url> for &str {
    fn into(self) -> Url {
        Url::from_str(self).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
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
impl UrlError {
    pub fn to_string(&self) -> String {
        match self {
            UrlError::InvalidSchema(s) => format!("Invalid schema: {}", s),
            UrlError::InvalidString(s) => format!("Invalid string: {}", s),
        }
    }
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
        let url = Url::from_str("https://api.openai.com/v1/chat/completions").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "api.openai.com");
        assert_eq!(url.path(), "/v1/chat/completions");
        assert_eq!(url.port(), 443);
        assert_eq!(
            url.to_string(),
            "https://api.openai.com/v1/chat/completions"
        );
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
    fn url構造体はaddr_strを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.to_addr_str(), "localhost:443");
    }
    #[test]
    fn url構造体はportを返すことができる() {
        let url = Url::from_str("https://localhost/test").unwrap();
        assert_eq!(url.port(), 443);
        let url = Url::from_str("http://localhost/test:10000").unwrap();
        assert_eq!(url.port(), 10000);
    }
}
