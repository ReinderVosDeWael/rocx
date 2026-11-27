mod diagnostics;
mod editor;
mod schema;
mod search;
mod session;

use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ErrorCode, Implementation,
        ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
};
use serde_json::json;
use tokio::sync::Mutex;

use crate::{diagnostics::ToolError, schema::ToolKind, session::Session};

#[derive(Default)]
struct RocxServer {
    session: Mutex<Session>,
}

impl ServerHandler for RocxServer {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::default();
        info.protocol_version = ProtocolVersion::V_2025_11_25;
        info.server_info = Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(
            ToolKind::ALL
                .into_iter()
                .map(ToolKind::definition)
                .collect(),
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = ToolKind::ALL
            .into_iter()
            .find(|tool| tool.name() == request.name)
            .ok_or_else(|| {
                ErrorData::new(ErrorCode::METHOD_NOT_FOUND, "Unknown rocx tool", None)
            })?;
        let call = match tool.parse(request.arguments.unwrap_or_default()) {
            Ok(call) => call,
            Err(message) => {
                return Ok(CallToolResult::structured_error(json!({
                    "category": "invalid_input",
                    "message": message,
                    "action": "Use tools/list to check required fields and allowed values."
                }))
                .into());
            }
        };
        // Session operations are synchronous, including file I/O and DOCX encoding.
        // Keep the stdio runtime free to read and write protocol frames.
        let mut session = self.session.lock().await;
        let result = tokio::task::block_in_place(|| call.dispatch(&mut session));
        Ok(match result {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => session_error(tool, &error),
        }
        .into())
    }
}

fn session_error(tool: ToolKind, error: &ToolError) -> CallToolResult {
    let action = match error.category.as_str() {
        "invalid_document" => "Open or create a document in this session.",
        "invalid_anchor" | "wrong_anchor_type" => {
            "Inspect the document again and use an anchor of the required type."
        }
        "invalid_inline_handle" => {
            "Inspect the document again and use an inline_handle from an inline view."
        }
        "invalid_cursor" if matches!(tool, ToolKind::Search) => {
            "Omit the cursor to restart, or use a next_cursor from the same document and query."
        }
        "invalid_cursor" => "Omit the cursor to restart, or use a next_cursor from the same view.",
        "wrong_owner" if matches!(tool, ToolKind::Search) => {
            "Use a next_cursor issued for the selected document, or omit it to restart."
        }
        "wrong_owner" if matches!(tool, ToolKind::Inspect) => {
            "Use an anchor, inline_handle, or next_cursor issued for the selected document. To restart a paged inspection, omit the cursor."
        }
        "wrong_owner" => "Use anchors and previews belonging to the selected document.",
        "invalid_preview" => "Create a preview for the selected document.",
        "invalid_batch" | "rejected_batch" => {
            "Check the edit targets and values, then retry the batch."
        }
        "capacity" => "Close a document to release session capacity.",
        "source_path" | "destination_exists" => "Choose a new output path.",
        "no_source" => "Use save_copy to write this preview to a new file.",
        "source_alias" => "Open the regular source file directly and create a new preview.",
        "source_changed" => "Reopen the current file and create a new preview, or use save_copy.",
        "invalid_path" | "io" => "Check the server-host path and permissions before retrying.",
        "invalid_document_data" if matches!(tool, ToolKind::Open) => {
            "Check that the input is a valid DOCX and fits the supplied import limits."
        }
        "invalid_document_data" | "inspection_failed" => "Check that the input is a valid DOCX.",
        "invalid_paragraphs" => "Check the supplied paragraph text.",
        _ => "Check the request and session state before retrying.",
    };
    CallToolResult::structured_error(error.response(action))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Native editor and XML traversal recurse over supported field trees.
    // Near-limit inputs can overflow the default worker stack in debug builds.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?
        .block_on(serve())
}

async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    RocxServer::default()
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_errors_explain_how_to_restart_the_entries_view() {
        for error in ["invalid_cursor", "wrong_owner"] {
            let result = session_error(
                ToolKind::Inspect,
                &format!("{error}: inspect entries again").into(),
            );
            let content = result.structured_content.unwrap();
            assert_eq!(result.is_error, Some(true));
            assert_eq!(content["category"], error);
            let action = content["action"].as_str().unwrap();
            assert!(action.contains("next_cursor") && action.contains("restart"));
        }
        let content = session_error(
            ToolKind::Preview,
            &String::from("wrong_owner: foreign anchor").into(),
        )
        .structured_content
        .unwrap();
        assert_eq!(
            content["action"],
            "Use anchors and previews belonging to the selected document."
        );
        let content = session_error(
            ToolKind::Inspect,
            &String::from("wrong_owner: foreign anchor").into(),
        )
        .structured_content
        .unwrap();
        assert!(content["action"].as_str().unwrap().contains("anchor"));
        let content = session_error(
            ToolKind::Inspect,
            &String::from("invalid_inline_handle: not issued").into(),
        )
        .structured_content
        .unwrap();
        assert!(
            content["action"]
                .as_str()
                .unwrap()
                .contains("inline_handle")
        );
    }
}
