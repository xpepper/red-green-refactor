use super::{LlmPatch, LlmProvider, ProviderConfig};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

pub struct OpenAiProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
    base: String,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(cfg: ProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder().build()?;
        let base = cfg.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let env_key = cfg.api_key_env.clone().unwrap_or_else(|| "OPENAI_API_KEY".to_string());
        let api_key = std::env::var(&env_key).with_context(|| format!("missing env var {}", env_key))?;
        Ok(Self { cfg, client, base, api_key })
    }
}

#[derive(Debug, Serialize)]
struct ChatReq<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message<'a> { role: &'a str, content: &'a str }

#[derive(Debug, Deserialize)]
struct ChatResp { choices: Vec<Choice> }
#[derive(Debug, Deserialize)]
struct Choice { message: ChoiceMessage }
#[derive(Debug, Deserialize)]
struct ChoiceMessage { content: String }

fn extract_json_object(s: &str) -> Option<&str> {
    // naive extraction of first top-level JSON object
    let bytes = s.as_bytes();
    let mut depth = 0isize;
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'{' {
            if depth == 0 { start = Some(i); }
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                if let Some(st) = start { return Some(&s[st..=i]); }
            }
        }
    }
    None
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate_patch(&self, role: &str, context: &str, instructions: &str) -> Result<LlmPatch> {
        let url = format!("{}/chat/completions", self.base.trim_end_matches('/'));
        let sys = "You are a code-modifying agent. Respond ONLY with a valid JSON object matching schema LlmPatch { files:[{path, mode: 'rewrite'|'append', content}], commit_message?, notes? }. No prose.";
        let user = format!("Instructions:\n{}\n\nProject context (truncated):\n{}", instructions, context);
        let req = ChatReq { model: &self.cfg.model, messages: vec![ Message{ role: "system", content: sys }, Message{ role: "user", content: &user } ], temperature: 0.2 };
        let resp = self.client.post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&req)
            .send().await?
            .error_for_status()?;
        let body: ChatResp = resp.json().await?;
        let content = body.choices.get(0).map(|c| c.message.content.as_str()).ok_or_else(|| anyhow!("no choices"))?;
        let json_str = extract_json_object(content).unwrap_or(content);
        let patch: LlmPatch = serde_json::from_str(json_str).with_context(|| format!("failed to parse model JSON: {}", json_str))?;
        Ok(patch)
    }
}

