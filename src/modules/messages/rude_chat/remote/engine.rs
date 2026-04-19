// scr/remote/engine.rs
use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use std::time::Duration;
use crate::get_laozhang_token;
// Подключаем модуль models (через crate root)
use crate::modules::messages::rude_chat::remote::models::{
    ChatCompletionRequest, ChatCompletionResponse, Message,
};

#[derive(Clone)]
pub struct LlmEngine {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LlmEngine {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: "https://api.laozhang.ai/v1".to_string(),
            api_key: get_laozhang_token(),
        }
    }

    pub async fn complete(
        &self,
        model: &str,
        messages: Vec<Message>,
        temperature: f32,
    ) -> Result<Message> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let request_body = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: Some(150),
            stream: Some(false),
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let mut auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_key))?;
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to API")?;

        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .context("Failed to read response bytes")?;
        let text = String::from_utf8_lossy(&bytes);

        if !status.is_success() {
            anyhow::bail!("API Error (Status {}): {}", status, text);
        }

        let parsed: ChatCompletionResponse = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse JSON. Raw response: {}", text))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))
    }
}
