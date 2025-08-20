use super::{LlmPatch, LlmProvider, ProviderConfig, extract_json_object};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GeminiProvider {
    cfg: ProviderConfig,
    client: reqwest::Client,
    base: String,
    api_key: String,
}

impl GeminiProvider {
    pub fn new(cfg: ProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder().build()?;
        let base = cfg
            .base_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
        let env_key = cfg
            .api_key_env
            .clone()
            .unwrap_or_else(|| "GEMINI_API_KEY".to_string());
        let api_key =
            std::env::var(&env_key).with_context(|| format!("missing env var {env_key}"))?;
        Ok(Self {
            cfg,
            client,
            base,
            api_key,
        })
    }
}

#[derive(Debug, Serialize)]
struct ContentPart<'a> {
    text: &'a str,
}
#[derive(Debug, Serialize)]
struct Content<'a> {
    role: &'a str,
    parts: Vec<ContentPart<'a>>,
}
#[derive(Debug, Serialize)]
struct GenReq<'a> {
    contents: Vec<Content<'a>>,
    generation_config: GenCfg,
}
#[derive(Debug, Serialize)]
struct GenCfg {
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GenResp {
    candidates: Vec<Cand>,
}
#[derive(Debug, Deserialize)]
struct Cand {
    content: CandContent,
}
#[derive(Debug, Deserialize)]
struct CandContent {
    parts: Vec<CandPart>,
}
#[derive(Debug, Deserialize)]
struct CandPart {
    text: Option<String>,
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate_patch(
        &self,
        role: &str,
        context: &str,
        instructions: &str,
    ) -> Result<LlmPatch> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base.trim_end_matches('/'),
            self.cfg.model,
            self.api_key
        );
        let sys = "You are a code-modifying agent. Respond ONLY with a valid JSON object matching schema LlmPatch { files:[{path, mode: 'rewrite'|'append', content}], commit_message?, notes? }. No prose.";
        let user = format!(
            "Role: {role}\nInstructions:\n{instructions}\n\nProject context (truncated):\n{context}"
        );
        let req = GenReq {
            contents: vec![Content {
                role: "user",
                parts: vec![ContentPart { text: sys }, ContentPart { text: &user }],
            }],
            generation_config: GenCfg { temperature: 0.2 },
        };
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?;
        let body: GenResp = resp.json().await?;
        let text = body
            .candidates
            .iter()
            .flat_map(|c| c.content.parts.iter())
            .filter_map(|p| p.text.as_ref())
            .next()
            .ok_or_else(|| anyhow!("no candidates"))?;
        let json_str = extract_json_object(text).unwrap_or(text);
        let patch: LlmPatch = serde_json::from_str(json_str)
            .with_context(|| format!("failed to parse model JSON: {json_str}"))?;
        Ok(patch)
    }
}
