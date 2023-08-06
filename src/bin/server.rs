use rsse::SseServer;

fn main() {
    let mut server = SseServer::default();
    for s in "Hello World!".chars() {
        server.add_response(s.to_string().as_str());
    }
    server.start().unwrap();
}
