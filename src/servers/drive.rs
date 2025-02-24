use anyhow::Result;
use async_mcp::{
    server::Server,
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ListRequest, Resource, ResourcesListResponse,
        ServerCapabilities, Tool, ToolResponseContent,
    },
};
use serde_json::json;
use url::Url;

use crate::client::get_drive_client;

fn get_access_token(req: &CallToolRequest) -> Result<&str> {
    req.meta
        .as_ref()
        .and_then(|v| v.get("access_token"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid access_token"))
}

pub fn build<T: Transport>(transport: T) -> Result<Server<T>> {
    let mut server = Server::builder(transport)
        .capabilities(ServerCapabilities {
            tools: Some(json!({
                "drive": {
                    "version": "v3",
                    "description": "Google Drive API operations"
                }
            })),
            ..Default::default()
        })
        .request_handler("resources/list", |_req: ListRequest| {
            Box::pin(async move { Ok(list_drive_resources()) })
        });

    // List files
    server.register_tool(
        Tool {
            name: "list_files".to_string(),
            description: Some("List files in Google Drive with filters".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mime_type": {"type": "string"},
                    "query": {"type": "string"},
                    "page_size": {"type": "integer", "default": 10},
                    "order_by": {"type": "string", "default": "modifiedTime desc"}
                }
            }),
        },
        move |req: CallToolRequest| {
            Box::pin(async move {
                let access_token = get_access_token(&req)?;
                let args = req.arguments.clone().unwrap_or_default();

                let result = async {
                    let drive = get_drive_client(access_token);

                    let mut query = String::new();
                    if let Some(mime_type) = args.get("mime_type").and_then(|v| v.as_str()) {
                        query.push_str(&format!("mimeType='{}'", mime_type));
                    }

                    let result = drive
                        .files()
                        .list()
                        .q(&query)
                        .page_size(
                            args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(10) as i32
                        )
                        .order_by(
                            args.get("order_by")
                                .and_then(|v| v.as_str())
                                .unwrap_or("modifiedTime desc"),
                        )
                        .doit()
                        .await?;

                    Ok(CallToolResponse {
                        content: vec![ToolResponseContent::Text {
                            text: serde_json::to_string(&result.1)?,
                        }],
                        is_error: None,
                        meta: None,
                    })
                }
                .await;

                handle_result(result)
            })
        },
    );

    Ok(server.build())
}

fn list_drive_resources() -> ResourcesListResponse {
    let base = Url::parse("https://www.googleapis.com/drive/v3/").unwrap();
    ResourcesListResponse {
        resources: vec![Resource {
            uri: base,
            name: "drive".to_string(),
            description: Some("Google Drive API".to_string()),
            mime_type: Some("application/json".to_string()),
        }],
        next_cursor: None,
        meta: None,
    }
}

fn handle_result(result: Result<CallToolResponse>) -> Result<CallToolResponse> {
    match result {
        Ok(response) => Ok(response),
        Err(e) => Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: format!("Error: {}", e),
            }],
            is_error: Some(true),
            meta: None,
        }),
    }
}
