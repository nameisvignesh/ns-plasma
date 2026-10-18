use std::time::Duration;

use reqwest::{
    Method,
    blocking::Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider base URL is empty")]
    EmptyBaseUrl,
    #[error("provider model is empty")]
    EmptyModel,
    #[error("API key environment variable `{0}` is not set")]
    MissingApiKey(String),
    #[error("provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("provider returned HTTP {status}: {body}")]
    Http {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("provider response was missing a text choice")]
    MissingText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ProviderSettings {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key_env: Option<String>,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            name: "OpenAI-compatible provider".to_owned(),
            base_url: "http://localhost:11434/v1".to_owned(),
            model: String::new(),
            api_key_env: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
}

pub struct ProviderClient {
    settings: ProviderSettings,
    client: Client,
}

impl ProviderClient {
    pub fn new(settings: ProviderSettings) -> Result<Self, ProviderError> {
        if settings.base_url.trim().is_empty() {
            return Err(ProviderError::EmptyBaseUrl);
        }
        Ok(Self {
            settings,
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()?,
        })
    }

    pub fn chat(&self, messages: &[ChatMessage]) -> Result<String, ProviderError> {
        if self.settings.model.trim().is_empty() {
            return Err(ProviderError::EmptyModel);
        }
        let body = serde_json::json!({
            "model": self.settings.model,
            "messages": messages,
            "stream": false
        });
        let response =
            self.request(Method::POST, self.endpoint("chat/completions"), Some(&body))?;
        let parsed: ChatResponse = response.json()?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|text| !text.is_empty())
            .ok_or(ProviderError::MissingText)
    }

    pub fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.request(Method::GET, self.endpoint("models"), None)?;
        let mut models = response
            .json::<ModelsResponse>()?
            .data
            .into_iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        models.sort();
        Ok(models)
    }

    pub fn test_connection(&self) -> Result<(), ProviderError> {
        self.request(Method::GET, self.endpoint("models"), None)
            .map(|_| ())
    }

    fn endpoint(&self, suffix: &str) -> String {
        format!(
            "{}/{}",
            self.settings.base_url.trim_end_matches('/'),
            suffix
        )
    }

    fn request(
        &self,
        method: Method,
        url: String,
        body: Option<&serde_json::Value>,
    ) -> Result<reqwest::blocking::Response, ProviderError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(environment) = self
            .settings
            .api_key_env
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let key = std::env::var(environment)
                .map_err(|_| ProviderError::MissingApiKey(environment.to_owned()))?;
            let value = HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| ProviderError::MissingApiKey(environment.to_owned()))?;
            headers.insert(AUTHORIZATION, value);
        }
        let mut request = self.client.request(method, url).headers(headers);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send()?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .unwrap_or_else(|_| "<unreadable response>".to_owned());
            return Err(ProviderError::Http { status, body });
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_has_one_separator() {
        let client = ProviderClient::new(ProviderSettings {
            base_url: "http://localhost:1234/v1/".to_owned(),
            ..ProviderSettings::default()
        })
        .expect("valid client");
        assert_eq!(client.endpoint("models"), "http://localhost:1234/v1/models");
    }
}
