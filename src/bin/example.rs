use rsse::client::SseClientBuilder;
use rsse::request::RequestBuilder;
use rsse::sse::subscriber::{SseHandler, SseMutHandler};
use rsse::url::Url;
fn main() {
    let url = Url::from_str("https://localhost/test").unwrap();
    let mut client = SseClientBuilder::new(&url)
        .proxy(&url)
        .unwrap()
        .add_ca("hello")
        .unwrap()
        .build();
    let req = RequestBuilder::new(&url).get().build();
    let mut handler = Handler {};
    client.send(&req, &handler).unwrap();
    client.send_mut(&req, &mut handler).unwrap();
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
