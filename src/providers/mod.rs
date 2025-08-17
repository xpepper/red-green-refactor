use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod openai;
pub mod gemini;
pub mod mock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Gemini,
    Mock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    /// For OpenAI-compatible APIs (DeepSeek, Groq, local servers) or Gemini base URL override
    pub base_url: Option<String>,
    /// Name of the env var containing the API key (e.g., OPENAI_API_KEY, GEMINI_API_KEY)
    pub api_key_env: Option<String>,
    /// Optional organization or project id header
    pub organization: Option<String>,
    /// Optional custom API key header name (e.g., "api-key" for GitHub Models)
    pub api_key_header: Option<String>,
    /// Optional API key prefix value (defaults to "Bearer ", set to "" for raw keys)
    pub api_key_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleProviderConfig {
    pub provider: ProviderConfig,
    /// Optional system prompt addon specific to your project
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    /// Path relative to project root
    pub path: String,
    /// How to apply the content
    pub mode: EditMode,
    /// Full new content (for Rewrite) or appended content (for Append)
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditMode { Rewrite, Append }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmPatch {
    pub files: Vec<FileEdit>,
    pub commit_message: Option<String>,
    pub notes: Option<String>,
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate_patch(&self, role: &str, context: &str, instructions: &str) -> Result<LlmPatch>;
}

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn build(cfg: &ProviderConfig) -> Result<Box<dyn LlmProvider>> {
        match cfg.kind {
            ProviderKind::OpenAi => Ok(Box::new(openai::OpenAiProvider::new(cfg.clone())?)),
            ProviderKind::Gemini => Ok(Box::new(gemini::GeminiProvider::new(cfg.clone())?)),
            ProviderKind::Mock => Ok(Box::new(mock::MockProvider)),
        }
    }
}
