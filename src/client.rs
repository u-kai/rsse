use std::{
    borrow::BorrowMut,
    cell::RefCell,
    fmt::Display,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
};

use rustls::{ClientConfig, ClientConnection, Stream};

use crate::{request_builder::Request, url::Url};

#[derive(Debug)]
pub struct SseClient {
    client: ClientConnection,
    tcp_stream: TcpStream,
}
#[derive(Debug)]
pub enum SseClientError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    TcpStreamConnectionError(String),
    ReadLineError(String),
    WriteAllError(String),
}
impl Display for SseClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrlError(e) => write!(f, "InvalidUrlError: {}", e),
            Self::ClientConnectionError(e) => write!(f, "ClientConnectionError: {}", e),
            Self::ReadLineError(e) => write!(f, "ReadLineError: {}", e),
            Self::TcpStreamConnectionError(e) => write!(f, "TcpStreamConnectionError: {}", e),
            Self::WriteAllError(e) => write!(f, "WriteAllError: {}", e),
        }
    }
}
impl std::error::Error for SseClientError {}

type Result<T> = std::result::Result<T, SseClientError>;

impl SseClient {
    pub fn default(url: &str) -> Result<Self> {
        let url = Url::from_str(url).map_err(|e| SseClientError::InvalidUrlError(e.to_string()))?;
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let client = ClientConnection::new(Arc::new(config), url.host().try_into().unwrap())
            .map_err(|e| SseClientError::ClientConnectionError(e.to_string()))?;
        let socket = TcpStream::connect(url.to_addr_str())
            .map_err(|e| SseClientError::TcpStreamConnectionError(e.to_string()))?;
        Ok(Self {
            client,
            tcp_stream: socket,
        })
    }
    pub fn stream_reader<'a>(
        &'a mut self,
        request: Request,
    ) -> Result<BufReader<Stream<'a, ClientConnection, TcpStream>>> {
        let req = request.bytes();
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream.write_all(req).map_err(|e| {
            SseClientError::WriteAllError(format!("error : {:#?}\nrequest : {:#?}", e, request))
        })?;
        let reader = BufReader::new(tls_stream);
        Ok(reader)
    }

    //pub fn stream_event(
    //&'static mut self,
    //request: Request,
    //) -> Result<(Receiver<String>, JoinHandle<Result<()>>)> {
    //let mut reader = self.stream_reader(request)?;
    //let mut line = String::new();
    //let mut len = 1;
    //let mut response = HttpResponse::new();
    //let (sender, receiver) = mpsc::channel();
    //let handle = thread::spawn(move || {
    //while len > 0 {
    //match reader.read_line(&mut line) {
    //Ok(l) => len = l,
    //Err(e) => return Err(SseClientError::ReadLineError(e.to_string())),
    //}
    //response.add_line(line.as_str());
    //if let Some(event) = response.new_event() {
    //sender.send(event.to_string()).unwrap();
    //}
    //line.clear();
    //}
    //if response.has_error() {
    //return Err(SseClientError::ReadLineError(response.to_string()));
    //}
    //Ok(())
    //});
    //Ok((receiver, handle))
    //}

    pub fn handle_event<'a>(
        &'a mut self,
        request: Request,
        event_handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
        error_handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
        read_line_error_handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let reader = self.stream_reader(request)?;
        StreamEventReader::new(reader).read_line_loop(
            event_handler,
            error_handler,
            read_line_error_handler,
        )?;
        Ok(())
    }
}

pub struct StreamEventReader<'a> {
    reader: BufReader<Stream<'a, ClientConnection, TcpStream>>,
}
impl<'a> StreamEventReader<'a> {
    pub fn new(reader: BufReader<Stream<'a, ClientConnection, TcpStream>>) -> Self {
        Self { reader }
    }
    pub fn read_line_loop(
        &mut self,
        handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
        error_handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
        read_line_error_handler: impl Fn(&str) -> std::result::Result<(), Box<dyn std::error::Error>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut event_data = EventData::new();
        let mut response = String::new();
        let mut read_len = 1;
        while read_len > 0 {
            match self.reader.read_line(&mut response) {
                Ok(len) => read_len = len,
                Err(e) => match read_line_error_handler(e.to_string().as_str()) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                },
            }
            event_data.set_response(response.as_str());
            match event_data.get_data() {
                Ok(event) => match handler(event) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                },
                Err(error) => match error_handler(error) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                },
            }
        }
        Ok(())
    }
}

pub struct EventData {
    data: String,
    error: String,
    error_word: String,
}

impl EventData {
    pub fn new() -> Self {
        Self {
            data: String::new(),
            error: String::new(),
            error_word: String::from("error"),
        }
    }
    pub fn get_data(&self) -> std::result::Result<&str, &str> {
        if !self.error.is_empty() {
            Err(&self.error)
        } else {
            Ok(&self.data)
        }
    }
    pub fn set_response(&mut self, response: &str) {
        if response.starts_with("data:") {
            self.error.clear();
            self.data = response.trim_start_matches("data:").trim().to_string();
            return;
        }
        self.data.clear();
        if response.contains(&self.error_word) {
            self.error = response.trim().to_string();
            return;
        }
    }
}
