// scr/remote/chat.rs
use crate::modules::messages::rude_chat::remote;
use anyhow::Result;
use remote::engine::LlmEngine;
use remote::models::{Message, Role};

pub struct ChatSession {
    engine: LlmEngine,
    model: String,
    history: Vec<Message>,
    temperature: f32,
}

impl ChatSession {
    pub fn new(engine: LlmEngine, system_prompt: Option<&str>) -> Self {
        let mut history = Vec::new();

        if let Some(prompt) = system_prompt {
            history.push(Message {
                role: Role::System,
                content: prompt.to_string(),
            });
        }

        Self {
            engine,
            model: "grok-4-1-fast-non-reasoning".to_string(),
            history,
            temperature: 0.7,
        }
    }

    pub fn set_temperature(&mut self, temp: f32) {
        self.temperature = temp;
    }

    pub async fn send_message(&mut self, user_input: &str) -> Result<String> {
        self.history.push(Message {
            role: Role::User,
            content: user_input.to_string(),
        });

        let response_message = self
            .engine
            .complete(&self.model, self.history.clone(), self.temperature)
            .await?;

        self.history.push(response_message.clone());

        Ok(response_message.content)
    }
}
