use std::{
    io::{BufRead, Write},
    net::TcpListener,
    thread::sleep,
    time::Duration,
};

pub struct SseServer {
    #[allow(dead_code)]
    addr: String,
    #[allow(dead_code)]
    responses: Vec<String>,
}
impl SseServer {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            responses: Vec::new(),
        }
    }
    #[allow(dead_code)]
    pub fn add_response(&mut self, response: &str) {
        self.responses.push(response.to_string());
    }
    #[allow(dead_code)]
    pub fn start(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(self.addr.as_str())?;
        for stream in listener.incoming() {
            let stream = stream?;
            self.handle_connection(stream)?;
        }
        Ok(())
    }
    #[allow(dead_code)]
    pub fn handle_connection(&self, mut stream: std::net::TcpStream) -> Result<(), std::io::Error> {
        let mut reader = std::io::BufReader::new(&mut stream);
        let mut line = String::new();
        while let Ok(size) = reader.read_line(&mut line) {
            if !size > 0 {
                break;
            }
        }
        let mut writer = std::io::BufWriter::new(&mut stream);
        writer.write_all(b"HTTP/1.1 200 OK\r\n")?;
        writer.write_all(b"Content-Type: text/event-stream\r\n")?;
        writer.write_all(b"\r\n")?;
        writer.flush()?;
        for s in &self.responses {
            writer.write_all(Self::make_sse_data(s).as_bytes())?;
            sleep(Duration::from_millis(500));
            writer.flush()?;
        }
        writer.write_all(b"\r\n")?;
        writer.flush()?;
        Ok(())
    }
    #[allow(dead_code)]
    fn make_sse_data(s: &str) -> String {
        format!("data: {}\r\n", s)
    }
}
impl Default for SseServer {
    fn default() -> Self {
        Self::new("localhost:8081")
    }
}
