use super::{EditMode, FileEdit, LlmPatch, LlmProvider};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Default)]
pub struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn generate_patch(
        &self,
        role: &str,
        _context: &str,
        _instructions: &str,
    ) -> Result<LlmPatch> {
        let mut patch = LlmPatch::default();
        let content = match role {
            "tester" => "// TODO: add a failing test\n",
            "implementor" => "// TODO: implement feature to make tests pass\n",
            _ => "// TODO: refactor without changing behavior\n",
        };
        patch.files.push(FileEdit {
            path: "red-green-refactor-mock.log".into(),
            mode: EditMode::Append,
            content: content.into(),
        });
        patch.commit_message = Some(format!("chore({role}): mock patch"));
        Ok(patch)
    }
}
