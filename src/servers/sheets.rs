use anyhow::{Context, Result};
use async_mcp::{
    server::{Server, ServerBuilder},
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ListRequest, Resource,
        ResourcesListResponse, ServerCapabilities, Tool, ToolResponseContent,
    },
};
use serde_json::json;
use url::Url;

use crate::client::get_sheets_client;

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

    register_tools(&mut server)?;

    Ok(server.build())
}

fn register_tools<T: Transport>(server: &mut ServerBuilder<T>) -> Result<()> {
    // Read values
    server.register_tool(
        Tool {
            name: "read_values".to_string(),
            description: Some("Read values from a Google Sheet".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
                }
            }),
        },
        move |req: CallToolRequest| {
            Box::pin(async move {
                let access_token = get_access_token(&req)?;
                let args = req.arguments.clone().unwrap_or_default();
                let context = req.meta.clone().unwrap_or_default();
                
                let result = async {
                    let sheets = get_sheets_client(access_token);
                    
                    let spreadsheet_id = context.get("spreadsheet_id")
                        .and_then(|v| v.as_str())
                        .context("spreadsheet_id required in context")?;
                    
                    // Get sheet name from context and combine with range
                    let sheet = context.get("sheet")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sheet1");
                    let user_range = context.get("range")
                        .and_then(|v| v.as_str())
                        .unwrap_or("A1:ZZ");
                    let range = format!("{}!{}", sheet, user_range);
                    
                    // Safely get major_dimension with default value
                    let major_dimension = args.get("major_dimension")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ROWS");

                    let result = sheets
                        .spreadsheets()
                        .values_get(spreadsheet_id, &range)
                        .major_dimension(major_dimension)
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
                    "values": {"type": "array", "items": {"type": "array", "items": {"type": "string"}}},
                    "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
                },
                "required": ["values"]
            }),
        },
        move |req: CallToolRequest| {
            Box::pin(async move {
                let access_token = get_access_token(&req)?;
                let args = req.arguments.clone().unwrap_or_default();
                let context = req.meta.clone().unwrap_or_default();
                
                let result = async {
                    let sheets = get_sheets_client(access_token);
                    
                    let spreadsheet_id = context.get("spreadsheet_id")
                        .and_then(|v| v.as_str())
                        .context("spreadsheet_id required in context")?;
                    
                    // Get sheet name from context and combine with range
                    let sheet = context.get("sheet")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sheet1");
                    let user_range = context.get("range")
                        .and_then(|v| v.as_str())
                        .unwrap_or("A1");
                    let range = format!("{}!{}", sheet, user_range);
                    
                    let values = args["values"].as_array().context("values required")?;
                    let major_dimension = args["major_dimension"].as_str().unwrap_or("ROWS");

                    let mut value_range = google_sheets4::api::ValueRange::default();
                    value_range.major_dimension = Some(major_dimension.to_string());
                    value_range.values = Some(values.iter().map(|row| {
                        row.as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|v| v.as_str().unwrap_or_default().to_string().into())
                            .collect::<Vec<serde_json::Value>>()
                    }).collect());

                    let result = sheets
                        .spreadsheets()
                        .values_update(value_range, spreadsheet_id, &range)
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

    // Create spreadsheet
    server.register_tool(
        Tool {
            name: "create_spreadsheet".to_string(),
            description: Some("Create a new Google Sheet".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "sheets": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"}
                            }
                        }
                    }
                },
                "required": ["title"]
            }),
        },
        move |req: CallToolRequest| {
            Box::pin(async move {
                let access_token = get_access_token(&req)?;
                let args = req.arguments.clone().unwrap_or_default();
                let result = async {
                    let sheets = get_sheets_client(access_token);
                    
                    let title = args["title"].as_str().context("title required")?;
                    
                    let mut spreadsheet = google_sheets4::api::Spreadsheet::default();
                    spreadsheet.properties = Some(google_sheets4::api::SpreadsheetProperties {
                        title: Some(title.to_string()),
                        ..Default::default()
                    });

                    // Add sheets if specified
                    if let Some(sheet_configs) = args["sheets"].as_array() {
                        let sheets = sheet_configs.iter().map(|config| {
                            let title = config["title"].as_str().unwrap_or("Sheet1").to_string();
                            google_sheets4::api::Sheet {
                                properties: Some(google_sheets4::api::SheetProperties {
                                    title: Some(title),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                        }).collect();
                        spreadsheet.sheets = Some(sheets);
                    }

                    let result = sheets
                        .spreadsheets()
                        .create(spreadsheet)
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

    // Clear values
    server.register_tool(
        Tool {
            name: "clear_values".to_string(),
            description: Some("Clear values from a range in a Google Sheet".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        move |req: CallToolRequest| {
            Box::pin(async move {
                let access_token = get_access_token(&req)?;
                let context = req.meta.clone().unwrap_or_default();
                
                let result = async {
                    let sheets = get_sheets_client(access_token);
                    
                    let spreadsheet_id = context.get("spreadsheet_id")
                        .and_then(|v| v.as_str())
                        .context("spreadsheet_id required in context")?;
                    
                    // Get sheet name from context and combine with range
                    let sheet = context.get("sheet")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sheet1");
                    let user_range = context.get("range")
                        .and_then(|v| v.as_str())
                        .unwrap_or("A1:ZZ");
                    let range = format!("{}!{}", sheet, user_range);

                    let clear_request = google_sheets4::api::ClearValuesRequest::default();
                    let result = sheets
                        .spreadsheets()
                        .values_clear(clear_request, spreadsheet_id, &range)
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