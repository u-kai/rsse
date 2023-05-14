use rustls::ClientConfig;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut client =
        rustls::ClientConnection::new(Arc::new(config), "api.openai.com".try_into().unwrap())?;

    let mut socket = TcpStream::connect("api.openai.com:443")?;
    let mut tls_stream = rustls::Stream::new(&mut client, &mut socket);
    let body = r#"{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"今日は\n"}],"stream":true}"#;
    let req = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: api.openai.com\r\nAccept: text/event-stream\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nConnection: keep-alive\r\nContent-Length: {}\r\n\r\n{}",
        std::env::var("OPENAI_API_KEY").unwrap(),
        body.len(),
        body
    );
    tls_stream.write_all(req.as_bytes())?;

    let mut reader = BufReader::new(tls_stream);

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        println!("{}", line);
        if line.starts_with("data:") {
            let data = line.trim_start_matches("data:").trim();
            println!("New event: {}", data);
        }
        line.clear();
    }

    Ok(())
}
