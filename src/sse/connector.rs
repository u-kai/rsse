use std::{
    cell::RefCell,
    fmt::{Debug, Display, Formatter},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::TcpStream,
    path::Path,
    sync::Arc,
};

use rustls::{Certificate, ClientConfig};
use rustls_pemfile::{read_one, Item};
use thiserror::Error;

use crate::{
    http::{
        body::HttpBody,
        header::HttpHeader,
        request::{Request, RequestBuilder},
        response::HttpResponse,
        status_line::HttpStatusLine,
    },
    url::Url,
};

use super::response::SseResponse;
pub type Result<T> = std::result::Result<T, SseConnectionError>;

pub(crate) struct SseTlsConnectorBuilder {
    url: Url,
    ca_paths: Vec<String>,
    proxy_url: Option<Url>,
}

impl SseTlsConnectorBuilder {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            ca_paths: Vec::new(),
            proxy_url: None,
        }
    }

    pub fn add_ca(mut self, ca_path: impl AsRef<Path>) -> Self {
        self.ca_paths
            .push(ca_path.as_ref().to_str().unwrap().to_string());
        self
    }

    pub fn proxy(mut self, proxy_url: impl Into<Url>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub fn build(self) -> Result<SseTlsConnector> {
        // set ca
        let mut ca = RootCertStore::new();
        self.ca_paths
            .iter()
            .map(|path| ca.add_ca(path))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SseConnectionError::CAFileIOError(e))?;

        // set proxy
        if let Some(proxy_url) = self.proxy_url {
            let client_connection = ClientConnection::proxy_connection(&self.url, &proxy_url, ca)?;
            return Ok(SseTlsConnector::new(client_connection));
        }

        let client_connection = ClientConnection::default(&self.url, ca)?;
        Ok(SseTlsConnector::new(client_connection))
    }
}

pub struct SseTlsConnector {
    conn: SseConnection<TlsSocket<StreamOwned>>,
}

impl SseTlsConnector {
    fn new(client_connection: ClientConnection) -> Self {
        let stream = StreamOwned::new(client_connection);
        let socket = TlsSocket::new(stream);
        Self {
            conn: SseConnection::new(socket),
        }
    }
}

struct ClientConnection {
    client: rustls::ClientConnection,
    tcp_stream: TcpStream,
}
impl ClientConnection {
    fn new(client: rustls::ClientConnection, tcp_stream: TcpStream) -> Self {
        Self { client, tcp_stream }
    }
    fn proxy_connection(url: &Url, proxy_url: &Url, certs: RootCertStore) -> Result<Self> {
        let client = Self::client(url, certs)?;

        let mut tcp_stream = TcpStream::connect(proxy_url.to_addr_str())
            .map_err(|e| SseConnectionError::ConnectError(e))?;
        let req = RequestBuilder::new(url).connect_request();

        tcp_stream
            .write_all(req.bytes())
            .map_err(|e| SseConnectionError::ConnectError(e))?;

        let mut buf = vec![0; 4096];

        while let Ok(size) = tcp_stream.read(&mut buf) {
            if size == 0 {
                break;
            }
            let proxy_response = String::from_utf8_lossy(&buf[..size]);
            if proxy_response.contains("Established") {
                return Ok(Self::new(client, tcp_stream));
            }
        }
        Err(ProxyConnectionError::new(
            proxy_url,
            url,
            ProxyConnectionErrorType::InvalidRequestError("Invalid Error".to_string()),
        ))
        .map_err(|e| SseConnectionError::ProxyConnectionError(e))
    }
    fn default(url: &Url, certs: RootCertStore) -> Result<Self> {
        let tcp_stream = TcpStream::connect(url.to_addr_str())
            .map_err(|e| SseConnectionError::ConnectError(e))?;
        let client = Self::client(url, certs)?;
        Ok(Self::new(client, tcp_stream))
    }
    fn client(url: &Url, certs: RootCertStore) -> Result<rustls::ClientConnection> {
        let ip = url
            .host()
            .try_into()
            .map_err(|_e| SseConnectionError::DnsError(InvalidDnsNameError::new(url)))?;
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(certs.root_store)
            .with_no_client_auth();
        let client = rustls::ClientConnection::new(Arc::new(config), ip).unwrap();
        Ok(client)
    }
}

struct RootCertStore {
    root_store: rustls::RootCertStore,
}
impl RootCertStore {
    fn new() -> Self {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        Self { root_store }
    }
    fn add_ca(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let Ok(Some(Item::X509Certificate(cert))) = read_one(&mut reader) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid cert",
            ));
        };
        let cert = Certificate(cert);
        self.root_store.add(&cert).unwrap();
        Ok(())
    }
}

impl SseConnector for SseTlsConnector {
    type Socket = TlsSocket<StreamOwned>;
    fn connect(&mut self, req: &Request) -> Result<&mut SseConnection<Self::Socket>> {
        self.conn
            .write(req.bytes())
            .map_err(|e| SseConnectionError::ConnectError(e))?;
        Ok(&mut self.conn)
    }
}

pub trait SseConnector {
    type Socket: Socket;
    fn connect(&mut self, req: &Request) -> Result<&mut SseConnection<Self::Socket>>;
}

pub trait Socket {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error>;
    fn write_all(&mut self, buf: &[u8]) -> std::result::Result<(), std::io::Error>;
}

pub trait Stream: std::io::Write + std::io::Read + Sized {
    fn reader(&self) -> BufReader<Self>;
    fn writer(&self) -> BufWriter<Self>;
}

#[derive(Debug)]
pub struct StreamOwned {
    client: Arc<RefCell<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>>,
}

impl StreamOwned {
    fn new(client: ClientConnection) -> Self {
        Self {
            client: Arc::new(RefCell::new(rustls::StreamOwned::new(
                client.client,
                client.tcp_stream,
            ))),
        }
    }
}
impl Stream for StreamOwned {
    fn reader(&self) -> BufReader<Self> {
        let client = Arc::clone(&self.client);
        BufReader::new(Self { client })
    }
    fn writer(&self) -> BufWriter<Self> {
        let client = Arc::clone(&self.client);
        BufWriter::new(Self { client })
    }
}
impl std::io::Read for StreamOwned {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        self.client.borrow_mut().read(buf)
    }
}
impl std::io::Write for StreamOwned {
    fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
        self.client.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        self.client.borrow_mut().flush()
    }
}

#[derive(Debug)]
pub struct TlsSocket<S: Stream> {
    reader: BufReader<S>,
    writer: BufWriter<S>,
}
impl<S: Stream + Debug> TlsSocket<S> {
    fn new(stream: S) -> Self {
        Self {
            reader: stream.reader(),
            writer: stream.writer(),
        }
    }
}
impl<S: Stream + Debug> Socket for TlsSocket<S> {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        let mut buf = String::new();
        let size = self.reader.read_line(&mut buf)?;
        if size == 0 {
            Ok(None)
        } else {
            Ok(Some(buf))
        }
    }
    fn write_all(&mut self, buf: &[u8]) -> std::result::Result<(), std::io::Error> {
        self.writer.write_all(buf)?;
        self.writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SseConnection<S: Socket> {
    conn: S,
}
impl<S: Socket> SseConnection<S> {
    fn new(conn: S) -> Self {
        Self { conn }
    }
    pub fn write(&mut self, buf: &[u8]) -> std::result::Result<(), std::io::Error> {
        self.conn.write_all(buf)
    }
    pub fn read(&mut self) -> Result<ConnectedSseResponse> {
        while let Some(line) = self
            .conn
            .read_line()
            .map_err(|e| SseConnectionError::ConnectionError(e))?
        {
            if let Ok(http_status) = HttpStatusLine::from_str(&line) {
                if !http_status.is_error() {
                    continue;
                };
                return Err(self.http_error(http_status));
            };
            // sse_response is look like header, so check sse_response first
            if let Ok(sse_response) = SseResponse::from_line(line.as_str()) {
                return Ok(ConnectedSseResponse::Progress(sse_response));
            };
            if let Ok(_header) = HttpHeader::from_line(line.as_str()) {
                continue;
            };
        }
        Ok(ConnectedSseResponse::Done)
    }
    fn http_error(&mut self, http_status: HttpStatusLine) -> SseConnectionError {
        let mut header = HttpHeader::new();
        let mut body = HttpBody::new();
        while let Some(line) = self.conn.read_line().map_or(None, |r| r) {
            if let Ok(add_header) = HttpHeader::from_line(line.as_str()) {
                header.concat(add_header);
                continue;
            };
            let add_body = HttpBody::from_line(line.as_str());
            body.concat(add_body)
        }
        SseConnectionError::HttpError(HttpResponse::new(http_status, header, body))
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum ConnectedSseResponse {
    Progress(SseResponse),
    Done,
}

#[derive(Debug, Error)]
pub struct InvalidDnsNameError {
    name: Url,
}
impl InvalidDnsNameError {
    pub fn new(name: impl Into<Url>) -> Self {
        Self { name: name.into() }
    }
}
impl Display for InvalidDnsNameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid dns name {}", self.name)
    }
}

#[derive(Debug, Error)]
pub struct ProxyConnectionError {
    proxy_url: Url,
    url: Url,
    error_type: ProxyConnectionErrorType,
}
impl Display for ProxyConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "proxy connection error proxy_url: {}, url: {}, error_type: {}",
            self.proxy_url, self.url, self.error_type
        )
    }
}
impl ProxyConnectionError {
    pub fn new(
        proxy_url: impl Into<Url>,
        url: impl Into<Url>,
        error_type: ProxyConnectionErrorType,
    ) -> Self {
        Self {
            proxy_url: proxy_url.into(),
            url: url.into(),
            error_type,
        }
    }
}
#[derive(Debug, Error)]
pub enum ProxyConnectionErrorType {
    #[error("connect error {0:?}")]
    ConnectError(std::io::Error),
    #[error("invalid request error {0:?}")]
    InvalidRequestError(String),
}

#[derive(Debug, Error)]
pub enum SseConnectionError {
    #[error("invalid url {0:?}")]
    InvalidUrl(String),
    #[error("connect to proxy error {0:?}")]
    ProxyConnectionError(ProxyConnectionError),
    #[error("ca file io error {0:?}")]
    CAFileIOError(std::io::Error),
    #[error("http error {0:?}")]
    HttpError(HttpResponse),
    #[error("connect io error {0:?}")]
    ConnectError(std::io::Error),
    #[error("connection io error {0:?}")]
    ConnectionError(std::io::Error),
    #[error("dns error {0:?}")]
    DnsError(InvalidDnsNameError),
}

#[cfg(test)]
mod tests {

    use crate::{
        http::{body::HttpBody, request::RequestBuilder, response::HttpResponse},
        sse::connector::{
            chatgpt::{chatgpt_key, evaluate_chatgpt_response, message, ChatGptRes, URL},
            fakes::FakeTcpConnection,
        },
    };

    use super::*;
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn 同じconnectionで通信を行うことが可能() {
        fn one_request(connector: &mut SseTlsConnector, message_: &str) {
            let req = RequestBuilder::new(&URL.try_into().unwrap())
                .post()
                .bearer_auth(&chatgpt_key())
                .json(message(message_))
                .build();
            let conn = connector.connect(&req).unwrap();
            let mut result = conn.read();
            let mut flag = false;
            while let Ok(res) = &result {
                let chatgpt_res = evaluate_chatgpt_response(res);
                match chatgpt_res {
                    ChatGptRes::Done => {
                        println!("done");
                        flag = true;
                        break;
                    }
                    ChatGptRes::Data(data) => {
                        println!("progress: {}", data);
                        result = conn.read();
                        continue;
                    }
                    ChatGptRes::Err => {
                        flag = false;
                        println!("err");
                        break;
                    }
                }
            }
            assert!(flag);
        }
        let mut tls_connector = SseTlsConnectorBuilder::new(&URL.try_into().unwrap())
            .build()
            .unwrap();
        one_request(&mut tls_connector, "hello");
        one_request(&mut tls_connector, "Ary you OK?");
        one_request(&mut tls_connector, "thanks");
    }
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn chatgptにtlsで通信する() {
        let req = RequestBuilder::new(&URL.try_into().unwrap())
            .post()
            .bearer_auth(&chatgpt_key())
            .json(message("hello"))
            .build();
        let mut tls_connector = SseTlsConnectorBuilder::new(req.url()).build().unwrap();
        let conn = tls_connector.connect(&req).unwrap();
        let mut result = conn.read();
        let mut flag = false;
        while let Ok(res) = &result {
            let chatgpt_res = evaluate_chatgpt_response(res);
            match chatgpt_res {
                ChatGptRes::Done => {
                    println!("done");
                    flag = true;
                    break;
                }
                ChatGptRes::Data(data) => {
                    println!("progress: {}", data);
                    result = conn.read();
                    continue;
                }
                ChatGptRes::Err => {
                    flag = false;
                    println!("err");
                    break;
                }
            }
        }
        assert!(flag);
    }
    #[test]
    fn tls_socketは書き込みもできる() {
        let url: Url = "https://www.google.com".try_into().unwrap();
        let client = ClientConnection::default(&url, RootCertStore::new()).unwrap();
        let stream = StreamOwned::new(client);
        let mut socket = TlsSocket::new(stream);
        socket
            .write_all("GET / HTTP/1.1\r\nHost: www.google.com:443\r\n\r\n".as_bytes())
            .unwrap();
        let res = socket.read_line().unwrap();
        println!("{:#?}", res);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "HTTP/1.1 200 OK\r\n");
    }
    #[test]
    fn sse_connectionはデータを接続相手から受け取りsseのレスポンスを返す() {
        let mut fake = FakeTcpConnection::new();
        fake.set_response("HTTP/1.1 200 OK\n\n");
        fake.set_response("Content-Type: text/event-stream\n\n");
        fake.set_response("\n\n");
        fake.set_response("data: Hello, World!\n\n");
        fake.set_response("data: Good Bye World\n\n");

        let mut sut = SseConnection::new(fake);

        let result = sut.read().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Hello, World!".to_string()))
        );

        let result = sut.read().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Good Bye World".to_string()))
        );

        let done = sut.read().unwrap();
        assert_eq!(done, ConnectedSseResponse::Done);
    }
    #[test]
    fn http_errorの場合はhttp_responseをそのままerrorに包んで返す() {
        let mut fake = FakeTcpConnection::new();
        fake.set_response("HTTP/1.1 404 Not Found\n\n");
        fake.set_response("Content-Type: text/event-stream\n\n");

        let mut sut = SseConnection::new(fake);
        let Err(SseConnectionError::HttpError(result)) = sut.read() else {
            panic!("expected Err, but got Ok");
        };

        assert_eq!(
            result,
            HttpResponse::new(
                HttpStatusLine::from_str("HTTP/1.1 404 Not Found").unwrap(),
                HttpHeader::from_line("Content-Type: text/event-stream").unwrap(),
                HttpBody::from_line("")
            )
        );
    }
}
#[cfg(test)]
pub(crate) mod fakes {
    use super::{Socket, SseConnection, SseConnectionError};

    pub struct FakeSseConnector {
        connected_times: usize,
        pub connection: SseConnection<FakeTcpConnection>,
    }
    impl FakeSseConnector {
        pub fn new() -> Self {
            Self {
                connection: SseConnection::new(FakeTcpConnection::new()),
                connected_times: 0,
            }
        }
        pub fn set_response(&mut self, response: &str) {
            self.connection.conn.set_response(response);
        }
        pub fn connected_times(&self) -> usize {
            self.connected_times
        }
    }
    impl super::SseConnector for FakeSseConnector {
        type Socket = FakeTcpConnection;
        fn connect(
            &mut self,
            _req: &super::Request,
        ) -> std::result::Result<&mut SseConnection<FakeTcpConnection>, SseConnectionError>
        {
            self.connected_times += 1;
            Ok(&mut self.connection)
        }
    }
    #[derive(Debug, Clone)]
    pub struct FakeTcpConnection {
        responses: Vec<String>,
    }
    impl FakeTcpConnection {
        pub fn new() -> Self {
            Self {
                responses: Vec::new(),
            }
        }
        pub fn set_response(&mut self, response: &str) {
            self.responses.push(response.to_string());
        }
    }
    impl Socket for FakeTcpConnection {
        fn write_all(&mut self, _buf: &[u8]) -> std::result::Result<(), std::io::Error> {
            Ok(())
        }
        fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
            if self.responses.is_empty() {
                return Ok(None);
            }
            Ok(Some(self.responses.remove(0)))
        }
    }
}
#[cfg(test)]
pub mod chatgpt {
    use crate::sse::{response::SseResponse, subscriber::HandleProgress};

    use super::ConnectedSseResponse;

    pub struct GptHandler {
        flag: bool,
        res: String,
    }
    impl GptHandler {
        pub fn new() -> Self {
            GptHandler {
                flag: false,
                res: "".to_string(),
            }
        }
        pub fn is_success(&self) -> bool {
            self.flag
        }
    }
    impl crate::sse::subscriber::SseMutHandler<String, ()> for GptHandler {
        fn handle(&mut self, res: SseResponse) -> HandleProgress<()> {
            let res = evaluate_chatgpt_sse_response(&res);
            match res {
                ChatGptRes::Done => {
                    println!("done");
                    self.flag = true;
                    HandleProgress::Done
                }
                ChatGptRes::Data(data) => {
                    println!("progress: {}", data);
                    self.res.push_str(&data);
                    HandleProgress::Progress
                }
                ChatGptRes::Err => {
                    self.flag = false;
                    println!("error");
                    HandleProgress::Err(())
                }
            }
        }
        fn result(&self) -> std::result::Result<String, ()> {
            Ok(self.res.clone())
        }
    }
    pub enum ChatGptRes {
        Done,
        Data(String),
        Err,
    }
    pub fn evaluate_chatgpt_sse_response(res: &SseResponse) -> ChatGptRes {
        match res {
            SseResponse::Retry(_) => ChatGptRes::Err,
            SseResponse::Data(data) => {
                if "[DONE]".contains(data) {
                    ChatGptRes::Done
                } else {
                    ChatGptRes::Data(data.to_string())
                }
            }
            SseResponse::Id(id) => ChatGptRes::Data(id.to_string()),
            SseResponse::Event(event) => ChatGptRes::Data(event.to_string()),
        }
    }
    pub fn evaluate_chatgpt_response(res: &ConnectedSseResponse) -> ChatGptRes {
        match res {
            ConnectedSseResponse::Done => ChatGptRes::Done,
            ConnectedSseResponse::Progress(SseResponse::Data(data)) => {
                if "[DONE]".contains(data) {
                    ChatGptRes::Done
                } else {
                    ChatGptRes::Data(data.to_string())
                }
            }
            _ => ChatGptRes::Err,
        }
    }

    pub const URL: &'static str = "https://api.openai.com/v1/chat/completions";
    pub fn message(mes: &str) -> ChatRequest {
        ChatRequest {
            model: OpenAIModel::Gpt3Dot5Turbo,
            messages: vec![Message {
                role: Role::User,
                content: mes.to_string(),
            }],
            stream: true,
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ChatRequest {
        model: OpenAIModel,
        messages: Vec<Message>,
        stream: bool,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Message {
        role: Role,
        content: String,
    }
    #[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
    pub enum Role {
        User,
        Assistant,
    }
    impl Role {
        fn into_str(&self) -> &'static str {
            match self {
                Self::User => "user",
                Self::Assistant => "assistant",
            }
        }
    }
    impl serde::Serialize for Role {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let role: &str = self.into_str();
            serializer.serialize_str(role)
        }
    }
    #[derive(Debug, Clone, serde::Deserialize)]
    pub enum OpenAIModel {
        Gpt3Dot5Turbo,
    }
    impl serde::Serialize for OpenAIModel {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
        {
            serializer.serialize_str(self.into_str())
        }
    }

    impl OpenAIModel {
        pub fn into_str(&self) -> &'static str {
            match self {
                Self::Gpt3Dot5Turbo => "gpt-3.5-turbo",
            }
        }
    }
    impl Into<&'static str> for OpenAIModel {
        fn into(self) -> &'static str {
            self.into_str()
        }
    }
    pub fn chatgpt_key() -> String {
        std::env::var("OPENAI_API_KEY").unwrap()
    }
}
