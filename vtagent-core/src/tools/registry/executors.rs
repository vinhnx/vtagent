use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use serde_json::{Value, json};

use crate::tools::apply_patch::Patch;
use crate::tools::traits::Tool;

use super::ToolRegistry;

impl ToolRegistry {
    pub(super) fn grep_search_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.search_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn list_files_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.file_ops_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn run_terminal_cmd_executor(
        &mut self,
        args: Value,
    ) -> BoxFuture<'_, Result<Value>> {
        let tool = self.command_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn read_file_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.file_ops_tool.clone();
        Box::pin(async move { tool.read_file(args).await })
    }

    pub(super) fn write_file_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.file_ops_tool.clone();
        Box::pin(async move { tool.write_file(args).await })
    }

    pub(super) fn edit_file_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move { self.edit_file(args).await })
    }

    pub(super) fn ast_grep_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move { self.execute_ast_grep(args).await })
    }

    pub(super) fn simple_search_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.simple_search_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn bash_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.bash_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn apply_patch_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async move { self.execute_apply_patch(args).await })
    }

    pub(super) fn srgn_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.srgn_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) fn speckit_executor(&mut self, args: Value) -> BoxFuture<'_, Result<Value>> {
        let tool = self.speckit_tool.clone();
        Box::pin(async move { tool.execute(args).await })
    }

    pub(super) async fn execute_apply_patch(&self, args: Value) -> Result<Value> {
        let input = args
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Error: Missing 'input' string with patch content. Example: apply_patch({{ input: '*** Begin Patch...*** End Patch' }})"))?;
        let patch = Patch::parse(input)?;
        let results = patch.apply(&self.workspace_root).await?;
        Ok(json!({
            "success": true,
            "applied": results,
        }))
    }
}
