use crate::providers::{LlmProvider, ProviderFactory, RoleProviderConfig};
use crate::vcs;
use crate::workspace;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub tester: RoleProviderConfig,
    pub implementor: RoleProviderConfig,
    pub refactorer: RoleProviderConfig,
    #[serde(default = "default_test_cmd")]
    pub test_cmd: String,
    #[serde(default = "default_max_context")]
    pub max_context_bytes: usize,
    #[serde(default = "default_impl_attempts")]
    pub implementor_max_attempts: usize,
}

fn default_test_cmd() -> String {
    "cargo test --color never".to_string()
}
fn default_max_context() -> usize {
    200_000
}
fn default_impl_attempts() -> usize {
    3
}

impl OrchestratorConfig {
    pub fn example() -> Self {
        Self {
            tester: RoleProviderConfig { provider: crate::providers::ProviderConfig { kind: crate::providers::ProviderKind::Mock, model: "mock".into(), base_url: None, api_key_env: None, organization: None, api_key_header: None, api_key_prefix: None }, system_prompt: Some("You are the Tester. Add a single failing test expressing a new behavior. Only output a JSON LlmPatch.".into()) },
            implementor: RoleProviderConfig { provider: crate::providers::ProviderConfig { kind: crate::providers::ProviderKind::Mock, model: "mock".into(), base_url: None, api_key_env: None, organization: None, api_key_header: None, api_key_prefix: None }, system_prompt: Some("You are the Implementor. Make tests pass with minimal changes. Only output a JSON LlmPatch.".into()) },
            refactorer: RoleProviderConfig { provider: crate::providers::ProviderConfig { kind: crate::providers::ProviderKind::Mock, model: "mock".into(), base_url: None, api_key_env: None, organization: None, api_key_header: None, api_key_prefix: None }, system_prompt: Some("You are the Refactorer. Improve code without changing behavior. Keep tests passing. Only output a JSON LlmPatch.".into()) },
            test_cmd: default_test_cmd(),
            max_context_bytes: default_max_context(),
            implementor_max_attempts: default_impl_attempts(),
        }
    }
}

pub fn load_orchestrator_config(path: Option<&PathBuf>) -> Result<OrchestratorConfig> {
    if let Some(p) = path {
        let s = std::fs::read_to_string(p)
            .with_context(|| format!("reading config {}", p.display()))?;
        let cfg: OrchestratorConfig = if p.extension().and_then(|e| e.to_str()) == Some("json") {
            serde_json::from_str(&s)?
        } else {
            serde_yaml::from_str(&s)?
        };
        Ok(cfg)
    } else {
        Ok(OrchestratorConfig::example())
    }
}

pub struct Orchestrator {
    project_root: PathBuf,
    cfg: OrchestratorConfig,
    tester: Box<dyn LlmProvider>,
    implementor: Box<dyn LlmProvider>,
    refactorer: Box<dyn LlmProvider>,
}

impl Orchestrator {
    pub async fn new(project_root: PathBuf, cfg: OrchestratorConfig) -> Result<Self> {
        if !project_root.exists() {
            return Err(anyhow!(
                "project root does not exist: {}",
                project_root.display()
            ));
        }
        let tester = ProviderFactory::build(&cfg.tester.provider)?;
        let implementor = ProviderFactory::build(&cfg.implementor.provider)?;
        let refactorer = ProviderFactory::build(&cfg.refactorer.provider)?;
        Ok(Self {
            project_root,
            cfg,
            tester,
            implementor,
            refactorer,
        })
    }

    pub async fn red_green_refactor_cycle(&mut self) -> Result<()> {
        info!("Starting Red (Tester) step (model {})", &self.cfg.tester.provider.model);
        vcs::ensure_repo(&self.project_root).await?;

        let context = workspace::collect_context(&self.project_root, self.cfg.max_context_bytes)?;
        let tester_instr = self.build_tester_instructions();
        let patch = self
            .tester
            .generate_patch("tester", &context, &tester_instr)
            .await?;
        let touched = workspace::apply_patch(&self.project_root, &patch).await?;
        vcs::commit_paths(
            &self.project_root,
            &touched,
            patch
                .commit_message
                .as_deref()
                .unwrap_or("test: add failing test"),
        )
        .await?;
        let tester_head = vcs::get_head_commit(&self.project_root).await?;

        let (ok, out) = workspace::run_tests(&self.project_root, &self.cfg.test_cmd).await?;
        if ok {
            warn!("Tester step produced passing tests; proceeding anyway")
        } else {
            info!("Tests are red as expected")
        }

        info!("Starting Green (Implementor) step (model {})", &self.cfg.implementor.provider.model);
        let mut last_fail_output = out.clone();
        let mut impl_success = false;
        for attempt in 1..=self.cfg.implementor_max_attempts {
            let context2 =
                workspace::collect_context(&self.project_root, self.cfg.max_context_bytes)?;
            let impl_instr = self.build_implementor_instructions(&last_fail_output);
            let patch2 = self
                .implementor
                .generate_patch("implementor", &context2, &impl_instr)
                .await?;
            let touched2 = workspace::apply_patch(&self.project_root, &patch2).await?;
            let msg = patch2
                .commit_message
                .as_deref()
                .unwrap_or("feat: make tests pass");
            let msg = &format!("{msg} (attempt {attempt})");
            vcs::commit_paths(&self.project_root, &touched2, msg).await?;

            let (ok2, out2) = workspace::run_tests(&self.project_root, &self.cfg.test_cmd).await?;
            if ok2 {
                impl_success = true;
                break;
            }
            last_fail_output = out2;
            warn!(
                "Implementor attempt {} failed; retrying if attempts remain",
                attempt
            );
        }

        if !impl_success {
            warn!(
                "All implementor attempts failed; preserving attempts and resetting to tester commit"
            );
            let branch_name = format!(
                "attempts/implementor-{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            );
            let _ = vcs::create_branch_at_head(&self.project_root, &branch_name).await; // best effort
            vcs::reset_hard_to(&self.project_root, &tester_head).await?;
            // End this cycle here; next cycle will try again from a clean tester state
            return Ok(());
        }
        info!("Tests green");

        info!("Starting Refactor step (model {})", &self.cfg.refactorer.provider.model);
        let context3 = workspace::collect_context(&self.project_root, self.cfg.max_context_bytes)?;
        let ref_instr = self.build_refactorer_instructions();
        let patch3 = self
            .refactorer
            .generate_patch("refactorer", &context3, &ref_instr)
            .await?;
        let touched3 = workspace::apply_patch(&self.project_root, &patch3).await?;
        vcs::commit_paths(
            &self.project_root,
            &touched3,
            patch3
                .commit_message
                .as_deref()
                .unwrap_or("refactor: improve design"),
        )
        .await?;

        let (ok3, out3) = workspace::run_tests(&self.project_root, &self.cfg.test_cmd).await?;
        if !ok3 {
            warn!("Refactor step broke tests, reverting commit");
            vcs::reset_hard_head_minus_one(&self.project_root).await?;
            return Err(anyhow!(
                "Refactor step failed tests and was reverted. Output:\n{}",
                out3
            ));
        }
        info!("Refactor preserved green");
        Ok(())
    }

    fn build_tester_instructions(&self) -> String {
        let mut instructions = String::new();
        if let Some(system_prompt) = &self.cfg.tester.system_prompt {
            instructions.push_str(system_prompt);
            instructions.push_str("\n\n");
        }
        instructions.push_str("Task: Add exactly one failing unit test (red) for the next small behavior in the kata. Do not modify implementation code. Output ONLY JSON of schema LlmPatch.");
        instructions
    }

    fn build_implementor_instructions(&self, failing_output: &str) -> String {
        let mut instructions = String::new();
        if let Some(system_prompt) = &self.cfg.implementor.system_prompt {
            instructions.push_str(system_prompt);
            instructions.push_str("\n\n");
        }
        instructions.push_str("Task: Make the test suite pass with the simplest change. Keep edits minimal and focused. Use baby steps. Output ONLY JSON (LlmPatch).\n\nTest failures to fix:\n");
        instructions.push_str(failing_output);
        instructions
    }

    fn build_refactorer_instructions(&self) -> String {
        let mut instructions = String::new();
        if let Some(system_prompt) = &self.cfg.refactorer.system_prompt {
            instructions.push_str(system_prompt);
            instructions.push_str("\n\n");
        }
        instructions.push_str("Task: Refactor to improve clarity, remove duplication, and prepare for change. Don't change behavior. After edits, all tests must still pass. Keep steps small. Output ONLY JSON (LlmPatch).");
        instructions
    }
}
