use rsse::{client::SseClientBuilder, sse::response::SseResponse};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct GptRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}
fn main() {
    let mut client = SseClientBuilder::new(
        &"https://api.openai.com/v1/chat/completions"
            .try_into()
            .unwrap(),
    )
    // if you want to use proxy, you can use this method
    // .proxy("http://localhost:8080")
    // if you want to user root ca, you can use this method
    //.add_ca("ca.pem")
    .post()
    .json(GptRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello, I'm a human.".to_string(),
        }],
        stream: true,
    })
    .bearer_auth(env!("OPENAI_API_KEY"))
    .build();

    // call one time
    client
        .send_fn(|res| {
            println!("{:#?}", res);
            rsse::sse::subscriber::HandleProgress::<()>::Done
        })
        .unwrap();

    // call multiple times
    let mut res_str = String::new();
    client
        .send_mut_fn(|res| {
            println!("{:#?}", res);
            if let SseResponse::Data(data) = res {
                res_str.push_str(&data);
            }
            //res.push_str();
            rsse::sse::subscriber::HandleProgress::<()>::Done
        })
        .unwrap();
    println!("res_str:{}", res_str);
}
