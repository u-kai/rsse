use crate::request::Request;

type Result<T> = std::result::Result<T, String>;
trait SseConnection {
    fn consume(&mut self) -> Result<Option<String>>;
}
trait SseConnector {
    type Connection: SseConnection;
    fn connect(&mut self, req: &Request) -> Result<Self::Connection>;
}
trait SseMutHandler {
    fn handle(&mut self, buf: &str);
}
struct Subscriber<C: SseConnection, T: SseConnector<Connection = C>> {
    connector: T,
}
impl<C: SseConnection, T: SseConnector<Connection = C>> Subscriber<C, T> {
    fn new(connector: T) -> Self {
        Self { connector }
    }
    fn subscribe_mut(&mut self, req: &Request, handler: &mut impl SseMutHandler) -> Result<()> {
        let mut connection = self.connector.connect(req)?;
        loop {
            match connection.consume()? {
                Some(buf) => handler.handle(&buf),
                None => return Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use crate::request::RequestBuilder;

    use super::*;
    struct FakeSseConnector {
        response: String,
        called_time: usize,
    }
    impl FakeSseConnector {
        fn new() -> Self {
            Self {
                response: String::new(),
                called_time: 0,
            }
        }
        fn set_success_sse(&mut self, response: &str) {
            self.response = response.to_string();
        }
        fn connected_time(&self) -> usize {
            self.called_time
        }
    }
    struct FakeSseConnection {
        index: usize,
        response: String,
    }
    impl SseConnection for FakeSseConnection {
        fn consume(&mut self) -> Result<Option<String>> {
            let c = self
                .response
                .get(self.index..self.index + 1)
                .map(String::from);
            self.index += 1;
            println!("{:?}", c);
            Ok(c)
        }
    }
    impl SseConnector for FakeSseConnector {
        type Connection = FakeSseConnection;
        fn connect(&mut self, _req: &Request) -> Result<FakeSseConnection> {
            self.called_time += 1;
            Ok(FakeSseConnection {
                index: 0,
                response: self.response.clone(),
            })
        }
    }
    struct MockHandler {
        called: usize,
    }
    impl MockHandler {
        fn new() -> Self {
            Self { called: 0 }
        }
        fn called_time(&self) -> usize {
            self.called
        }
    }
    impl SseMutHandler for MockHandler {
        fn handle(&mut self, _message: &str) {
            self.called += 1;
        }
    }
    #[test]
    fn subscribeしてsseのコネクションを作成する() {
        let mut connector = FakeSseConnector::new();
        let response = "hello world";
        connector.set_success_sse(response);
        let mut handler = MockHandler::new();
        let mut sut = Subscriber::new(connector);
        let request = RequestBuilder::new("https://www.fake").get().build();

        sut.subscribe_mut(&request, &mut handler).unwrap();

        assert_eq!(sut.connector.connected_time(), 1);
        assert_eq!(handler.called_time(), response.len());
    }
}
