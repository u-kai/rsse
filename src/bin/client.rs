use std::{
    fs::File,
    io::{stdout, BufRead, Write},
};

use rsse::{
    client::SseClient,
    request_builder::RequestBuilder,
    response::{FromLine, SseResponse, SseResponseStore},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ChatRequest {
    model: OpenAIModel,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    role: Role,
    content: String,
}
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}
impl Role {
    fn into_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}
impl serde::Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let role: &str = self.into_str();
        serializer.serialize_str(role)
    }
}
#[derive(Debug, Clone, serde::Deserialize)]
pub enum OpenAIModel {
    Gpt3Dot5Turbo,
}
impl serde::Serialize for OpenAIModel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.into_str())
    }
}

impl OpenAIModel {
    pub fn into_str(&self) -> &'static str {
        match self {
            Self::Gpt3Dot5Turbo => "gpt-3.5-turbo",
        }
    }
}
impl Into<&'static str> for OpenAIModel {
    fn into(self) -> &'static str {
        self.into_str()
    }
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Chat {
    pub choices: Vec<ChatChoices>,
    pub created: usize,
    pub id: String,
    pub model: String,
    pub object: String,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatChoices {
    pub delta: ChatChoicesDelta,
    pub finish_reason: serde_json::Value,
    pub index: usize,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatChoicesDelta {
    pub content: Option<String>,
}

fn main() {
    let url = "https://api.openai.com/v1/chat/completions";
    let mut file = File::create("logs.txt").unwrap();
    let mut client = SseClient::default(url).unwrap();
    loop {
        let mut message = String::new();
        print!("{} > ", std::env::var("USER").unwrap_or_default());
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut message).unwrap();
        let request = RequestBuilder::new(url)
            .bearer_auth(std::env::var("OPENAI_API_KEY").unwrap().as_str())
            .post()
            .json(ChatRequest {
                stream: true,
                model: OpenAIModel::Gpt3Dot5Turbo,
                messages: vec![Message {
                    role: Role::User,
                    content: message,
                }],
            })
            .build();

        let mut reader = client.stream_reader(request).unwrap();
        let mut line = String::new();
        let mut response_store = SseResponseStore::new();
        while reader.read_line(&mut line).unwrap() > 0 {
            let response = response_store.evaluate_lines(line.as_str());
            match response {
                Ok(res) => {
                    let Some(event) = res.new_event() else {
                        //println!("res : {:#?}",res);
                        continue;
                    };
                    //file.write_all(format!("{:#?}", res).as_bytes()).unwrap();
                    let chat = serde_json::from_str::<Chat>(event);
                    match chat {
                        Ok(chat) => {
                            //println!("event {:#?}", event);
                            //println!("{:#?}", chat);
                            if let Some(choice) = chat.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    print!("{}", content);
                                    stdout().flush().unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            if event == "[DONE]" {
                                break;
                            }
                            println!("{:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
            line.clear();
        }
    }
}
