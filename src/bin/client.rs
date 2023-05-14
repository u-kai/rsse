use std::io::{stdout, Write};

use rsse::client::SseClient;

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
    let client = SseClient::default("https://api.openai.com/v1/chat/completions").unwrap();
    client
        .bearer_auth(std::env::var("OPENAI_API_KEY").unwrap().as_str())
        .post()
        .json_body(ChatRequest {
            stream: true,
            model: OpenAIModel::Gpt3Dot5Turbo,
            messages: vec![Message {
                role: Role::User,
                content: "今日は".to_string(),
            }],
        })
        .read_stream(|line| {
            let res = serde_json::from_str::<Chat>(line).unwrap();
            print!(
                "{}",
                res.choices[0]
                    .delta
                    .content
                    .as_ref()
                    .unwrap_or(&"".to_string())
            );
            stdout().flush().unwrap();
            Ok(())
        })
        .unwrap();
}
