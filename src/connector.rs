use std::{
    cell::RefCell,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{IpAddr, TcpStream},
    str::FromStr,
    sync::Arc,
};

use rustls::{Certificate, ClientConfig, ClientConnection, ServerName, Stream, StreamOwned};
use rustls_pemfile::{read_one, Item};

use crate::{request_builder::RequestBuilder, url::Url};

#[derive(Debug)]
pub struct HttpConnector {
    url: Url,
    client: ClientConnection,
    stream: TcpStream,
}
impl HttpConnector {
    pub fn default(url: &str) -> Self {
        let url = Url::from_str(url).unwrap();
        let ip = url.host().try_into().unwrap();
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        //let file = File::open("").unwrap();
        //let mut reader = BufReader::new(file);
        //match read_one(&mut reader).unwrap().unwrap() {
        //Item::X509Certificate(cert) => {
        //let cert = Certificate(cert);
        //root_store.add(&cert).unwrap();
        //}
        //_ => println!("error"),
        //}
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let client = ClientConnection::new(Arc::new(config), ip).unwrap();

        let stream = TcpStream::connect(url.to_addr_str()).unwrap();

        Self {
            client,
            stream,
            url,
        }
    }
    pub async fn connect(&mut self, proxy_url: &str) {
        let proxy_url = Url::from_str(proxy_url).unwrap();
        let request = RequestBuilder::new(self.url.clone()).connect().build();
        let mut stream = tokio::net::TcpStream::connect(proxy_url.to_addr_str())
            .await
            .unwrap();
        let mut stream = TcpStream::connect(proxy_url.to_addr_str()).unwrap();
        let mut buf = vec![0; 4096];
        //stream.write_all(request.bytes()).await.unwrap();
        //stream.read(&mut buf).await.unwrap();
        stream.write_all(request.bytes()).unwrap();
        stream.read(&mut buf).unwrap();

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
        let ip = self.url.host().try_into().unwrap();
        println!("{:?}", ip);
        //let ip = ServerName::IpAddress(IpAddr::from_str("127.0.0.1:8080").unwrap());
        let request = RequestBuilder::new(self.url.clone()).get().build();
        println!("{:#?}", request);
        let req = request.bytes();

        //let connector = TlsConnector::from(Arc::new(config));
        //let tokio_stream = tokio::net::TcpStream::from_std(stream).unwrap();
        let mut client = ClientConnection::new(Arc::new(config), ip).unwrap();
        let mut tls_stream = Stream::new(&mut client, &mut stream);
        tls_stream.write_all(req).unwrap(); //.await.unwrap();
        let mut res = String::new();
        let mut reader = BufReader::new(tls_stream);
        while reader.read_line(&mut res).unwrap() > 0 {
            println!("{}", res);
            res.clear();
        }

        assert!(false);
    }
}
