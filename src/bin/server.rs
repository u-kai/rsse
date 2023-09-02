use rsse::SseServer;

fn main() {
    let mut server = SseServer::default();

    for s in "Hello World!".chars() {
        server.add_response(s.to_string().as_str());
    }
    match server.start() {
        Ok(_) => println!("Server stopped"),
        Err(e) => println!("Server error: {}", e),
    };
}
