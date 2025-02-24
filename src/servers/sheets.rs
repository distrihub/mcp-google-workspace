use anyhow::{Context, Result};
use async_mcp::{
    server::{Server, ServerBuilder},
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ListRequest, Resource,
        ResourcesListResponse, ServerCapabilities, Tool, ToolResponseContent,
    },
};
use google_sheets4::Sheets;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

use crate::client::get_sheets_client;

pub struct SheetsServer {
    sheets: Arc<Mutex<Sheets<
        google_sheets4::hyper_rustls::HttpsConnector<
            google_sheets4::hyper_util::client::legacy::connect::HttpConnector,
        >,
    >>>,
}

impl SheetsServer {
    pub fn new(access_token: &str) -> Self {
        Self {
            sheets: Arc::new(Mutex::new(get_sheets_client(access_token))),
        }
    }

    pub fn build<T: Transport>(self, transport: T) -> Result<Server<T>> {
        let mut server = Server::builder(transport)
            .capabilities(ServerCapabilities {
                tools: Some(json!({
                    "sheets": {
                        "version": "v4",
                        "description": "Google Sheets API operations"
                    }
                })),
                ..Default::default()
            })
            .request_handler("resources/list", |_req: ListRequest| {
                Box::pin(async move { Ok(list_sheets_resources()) })
            });

        self.register_tools(&mut server)?;

        Ok(server.build())
    }

    fn register_tools<T: Transport>(&self, server: &mut ServerBuilder<T>) -> Result<()> {
        let sheets = self.sheets.clone();
        let sheets2 = sheets.clone();  // Clone for second tool

        // Read values
        server.register_tool(
            Tool {
                name: "read_values".to_string(),
                description: Some("Read values from a Google Sheet".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "spreadsheet_id": {"type": "string"},
                        "range": {"type": "string"},
                        "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
                    },
                    "required": ["spreadsheet_id", "range"]
                }),
            },
            move |req: CallToolRequest| {
                let sheets = sheets.clone();
                Box::pin(async move {
                    let args = req.arguments.unwrap_or_default();
                    let result = async {
                        let sheets = sheets.lock().await;
                        
                        let spreadsheet_id = args["spreadsheet_id"].as_str().context("spreadsheet_id required")?;
                        let range = args["range"].as_str().context("range required")?;

                        let result = sheets
                            .spreadsheets()
                            .values_get(spreadsheet_id, range)
                            .doit()
                            .await?;

                        Ok(CallToolResponse {
                            content: vec![ToolResponseContent::Text {
                                text: serde_json::to_string(&result.1)?,
                            }],
                            is_error: None,
                            meta: None,
                        })
                    }.await;

                    handle_result(result)
                })
            },
        );

        // Write values
        server.register_tool(
            Tool {
                name: "write_values".to_string(),
                description: Some("Write values to a Google Sheet".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "spreadsheet_id": {"type": "string"},
                        "range": {"type": "string"},
                        "values": {"type": "array", "items": {"type": "array", "items": {"type": "string"}}},
                        "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
                    },
                    "required": ["spreadsheet_id", "range", "values"]
                }),
            },
            move |req: CallToolRequest| {
                let sheets = sheets2.clone();
                Box::pin(async move {
                    let args = req.arguments.unwrap_or_default();
                    let result = async {
                        let sheets = sheets.lock().await;
                        
                        let spreadsheet_id = args["spreadsheet_id"].as_str().context("spreadsheet_id required")?;
                        let range = args["range"].as_str().context("range required")?;
                        let values = args["values"].as_array().context("values required")?;

                        let mut value_range = google_sheets4::api::ValueRange::default();
                        value_range.values = Some(values.iter().map(|row| {
                            row.as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string().into())
                                .collect::<Vec<serde_json::Value>>()
                        }).collect());

                        let result = sheets
                            .spreadsheets()
                            .values_update(value_range, spreadsheet_id, range)
                            .doit()
                            .await?;

                        Ok(CallToolResponse {
                            content: vec![ToolResponseContent::Text {
                                text: serde_json::to_string(&result.1)?,
                            }],
                            is_error: None,
                            meta: None,
                        })
                    }.await;

                    handle_result(result)
                })
            },
        );

        Ok(())
    }
}

fn list_sheets_resources() -> ResourcesListResponse {
    let base = Url::parse("https://sheets.googleapis.com/v4/").unwrap();
    ResourcesListResponse {
        resources: vec![Resource {
            uri: base,
            name: "sheets".to_string(),
            description: Some("Google Sheets API".to_string()),
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