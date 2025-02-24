use anyhow::Result;
use async_mcp::{
    server::{Server, ServerBuilder},
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ListRequest, PromptsListResponse, Resource,
        ResourcesListResponse, ServerCapabilities, Tool, ToolResponseContent,
    },
};
use google_drive3::DriveHub;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

use crate::client::get_drive_client;

pub struct DriveServer {
    drive: Arc<
        Mutex<
            DriveHub<
                google_drive3::hyper_rustls::HttpsConnector<
                    google_drive3::hyper_util::client::legacy::connect::HttpConnector,
                >,
            >,
        >,
    >,
}

impl DriveServer {
    pub fn new(access_token: &str) -> Self {
        Self {
            drive: Arc::new(Mutex::new(get_drive_client(access_token))),
        }
    }

    pub fn build<T: Transport>(self, transport: T) -> Result<Server<T>> {
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

        self.register_tools(&mut server)?;

        Ok(server.build())
    }

    fn register_tools<T: Transport>(&self, server: &mut ServerBuilder<T>) -> Result<()> {
        let drive = self.drive.clone();

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
                let drive = drive.clone();
                Box::pin(async move {
                    let args = req.arguments.unwrap_or_default();
                    let result = async {
                        let drive = drive.lock().await;

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

        // Create folder
        // server.register_tool(
        //     Tool {
        //         name: "create_folder".to_string(),
        //         description: Some("Create a new folder in Google Drive".to_string()),
        //         input_schema: json!({
        //             "type": "object",
        //             "properties": {
        //                 "name": {"type": "string"},
        //                 "parent_id": {"type": "string", "description": "Optional parent folder ID"}
        //             },
        //             "required": ["name"]
        //         }),
        //     },
        //     move |req: CallToolRequest| {
        //         let drive = drive.clone();
        //         Box::pin(async move {
        //             let args = req.arguments.unwrap_or_default();
        //             let result = async {
        //                 let drive = drive.lock().await;

        //                 let mut file = google_drive3::api::File::default();
        //                 file.name = Some(args["name"].as_str().unwrap().to_string());
        //                 file.mime_type = Some("application/vnd.google-apps.folder".to_string());

        //                 if let Some(parent_id) = args.get("parent_id").and_then(|v| v.as_str()) {
        //                     file.parents = Some(vec![parent_id.to_string()]);
        //                 }

        //                 let result = drive.files().create(file).doit().await?;

        //                 Ok(CallToolResponse {
        //                     content: vec![ToolResponseContent::Text {
        //                         text: serde_json::to_string(&result.1)?,
        //                     }],
        //                     is_error: None,
        //                     meta: None,
        //                 })
        //             }
        //             .await;

        //             handle_result(result)
        //         })
        //     },
        // );

        Ok(())
    }
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
