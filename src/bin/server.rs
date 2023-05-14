fn main() {}
//use std::time::Duration;
//use tokio::time::interval;
//use warp::Filter;

//#[tokio::main]
//async fn main() {
//let sse_route = warp::path("events").and(warp::get()).map(|| {
//let mut counter = 0;
//let event_stream = interval(Duration::from_secs(1)).map(move |_| {
//counter += 1;
//warp::sse::data(format!("event #{}", counter))
//});
//warp::sse::reply(warp::sse::keep_alive().stream(event_stream))
//});

//warp::serve(sse_route).run(([127, 0, 0, 1], 8000)).await;
//}
