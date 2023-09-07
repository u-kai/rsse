use rsse::client::SseClientBuilder;
use rsse::http::url::Url;
use rsse::sse::subscriber::{SseHandler, SseMutHandler};
fn main() {
    let url = Url::from_str("https://localhost/test").unwrap();
    let proxy_url = Url::from_str("https://localhost/proxy").unwrap();
    let mut client = SseClientBuilder::new(&url)
        .post()
        .json(r#"{}"#)
        .proxy(&proxy_url)
        .unwrap()
        .add_ca("hello")
        .unwrap()
        .build();
    let mut handler = Handler {};
    client.send(&handler).unwrap();
    client.send_mut(&mut handler).unwrap();
    client.get().send(&handler).unwrap();
}

struct Handler {}
impl SseHandler<(), ()> for Handler {
    fn handle(
        &self,
        _res: rsse::sse::response::SseResponse,
    ) -> rsse::sse::subscriber::HandleProgress<()> {
        rsse::sse::subscriber::HandleProgress::Progress
    }
    fn result(&self) -> std::result::Result<(), ()> {
        Ok(())
    }
}
impl SseMutHandler<(), ()> for Handler {
    fn handle(
        &mut self,
        _res: rsse::sse::response::SseResponse,
    ) -> rsse::sse::subscriber::HandleProgress<()> {
        rsse::sse::subscriber::HandleProgress::Progress
    }
    fn result(&self) -> std::result::Result<(), ()> {
        Ok(())
    }
}
