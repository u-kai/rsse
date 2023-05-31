use rsse::debug;

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let data = vec!["hello", "world"];
    debug!(data);
}
