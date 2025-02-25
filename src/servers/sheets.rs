use anyhow::{Context, Result};
use async_mcp::{
    server::{Server, ServerBuilder},
    transport::Transport,
    types::{
        CallToolRequest, CallToolResponse, ListRequest, Resource, ResourcesListResponse,
        ServerCapabilities, Tool, ToolResponseContent,
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
    // Tool Definitions
    let read_values_tool = Tool {
        name: "read_values".to_string(),
        description: Some("Read values from a Google Sheet".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "sheet": {"type": "string", "description": "Sheet name"},
                "range": {"type": "string", "description": "Range to read (e.g. 'A1:B2')", "default": "A1:ZZ"},
                "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
            },
            "required": ["sheet"]
        }),
    };

    let write_values_tool = Tool {
        name: "write_values".to_string(),
        description: Some("Write values to a Google Sheet".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "sheet": {"type": "string", "description": "Sheet name"},
                "range": {"type": "string", "description": "Range to write to (e.g. 'A1:B2')"},
                "values": {
                    "description": "2D array of values to write",
                    "type": "array",
                    "items": {
                        "type": "array",
                        "items": {
                        "type": ["string", "number", "boolean", "null"],
                        "description": "A single cell value"
                        }
                    }
                },
                "major_dimension": {"type": "string", "enum": ["ROWS", "COLUMNS"], "default": "ROWS"}
            },
            "required": ["values", "range", "sheet"]
        }),
    };

    let create_spreadsheet_tool = Tool {
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
    };

    let clear_values_tool = Tool {
        name: "clear_values".to_string(),
        description: Some("Clear values from a range in a Google Sheet".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "sheet": {"type": "string", "description": "Sheet name", "default": "Sheet1"},
                "range": {"type": "string", "description": "Range to clear (e.g. 'A1:B2')", "default": "A1:ZZ"}
            },
            "required": ["sheet", "range"]
        }),
    };

    let get_sheet_info_tool = Tool {
        name: "get_sheet_info".to_string(),
        description: Some("Get information about all sheets in a spreadsheet, including their titles and maximum ranges (e.g. 'A1:Z1000'). This is useful for discovering what sheets exist and their dimensions.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    };

    // Tool Implementations
    server.register_tool(read_values_tool, move |req: CallToolRequest| {
        Box::pin(async move {
            let access_token = get_access_token(&req)?;
            let args = req.arguments.clone().unwrap_or_default();
            let context = req.meta.clone().unwrap_or_default();

            let result = async {
                let sheets = get_sheets_client(access_token);

                let spreadsheet_id = context
                    .get("spreadsheet_id")
                    .and_then(|v| v.as_str())
                    .context("spreadsheet_id required in context")?;

                let sheet = args["sheet"].as_str().context("sheet name required")?;
                let user_range = args["range"].as_str().unwrap_or("A1:ZZ");
                let range = format!("{}!{}", sheet, user_range);

                let major_dimension = args
                    .get("major_dimension")
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
            }
            .await;

            handle_result(result)
        })
    });

    server.register_tool(write_values_tool, move |req: CallToolRequest| {
        Box::pin(async move {
            let access_token = get_access_token(&req)?;
            let args = req.arguments.clone().unwrap_or_default();
            let context = req.meta.clone().unwrap_or_default();

            let result = async {
                let sheets = get_sheets_client(access_token);

                let spreadsheet_id = context
                    .get("spreadsheet_id")
                    .and_then(|v| v.as_str())
                    .context("spreadsheet_id required in context")?;

                let sheet = args["sheet"].as_str().context("sheet name required")?;
                let user_range = args["range"].as_str().context("range is required")?;
                let range = format!("{}!{}", sheet, user_range);

                let values = args
                    .get("values")
                    .and_then(|v| v.as_array())
                    .context("values required")?;
                let major_dimension = args
                    .get("major_dimension")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ROWS");

                let mut value_range = google_sheets4::api::ValueRange::default();
                value_range.major_dimension = Some(major_dimension.to_string());
                value_range.values = Some(
                    values
                        .iter()
                        .map(|row| {
                            row.as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string().into())
                                .collect::<Vec<serde_json::Value>>()
                        })
                        .collect(),
                );

                let result = sheets
                    .spreadsheets()
                    .values_update(value_range, spreadsheet_id, &range)
                    .value_input_option("RAW")
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
    });

    server.register_tool(create_spreadsheet_tool, move |req: CallToolRequest| {
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
                    let sheets = sheet_configs
                        .iter()
                        .map(|config| {
                            let title = config["title"].as_str().unwrap_or("Sheet1").to_string();
                            google_sheets4::api::Sheet {
                                properties: Some(google_sheets4::api::SheetProperties {
                                    title: Some(title),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                        })
                        .collect();
                    spreadsheet.sheets = Some(sheets);
                }

                let result = sheets.spreadsheets().create(spreadsheet).doit().await?;

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
    });

    server.register_tool(clear_values_tool, move |req: CallToolRequest| {
        Box::pin(async move {
            let access_token = get_access_token(&req)?;
            let args = req.arguments.clone().unwrap_or_default();
            let context = req.meta.clone().unwrap_or_default();

            let result = async {
                let sheets = get_sheets_client(access_token);

                let spreadsheet_id = context
                    .get("spreadsheet_id")
                    .and_then(|v| v.as_str())
                    .context("spreadsheet_id required in context")?;

                let sheet = args
                    .get("sheet")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Sheet1");
                let user_range = args
                    .get("range")
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
            }
            .await;

            handle_result(result)
        })
    });

    server.register_tool(get_sheet_info_tool, move |req: CallToolRequest| {
        Box::pin(async move {
            let access_token = get_access_token(&req)?;
            let context = req.meta.clone().unwrap_or_default();

            let result = async {
                let sheets = get_sheets_client(access_token);

                let spreadsheet_id = context
                    .get("spreadsheet_id")
                    .and_then(|v| v.as_str())
                    .context("spreadsheet_id required in context")?;

                let result = sheets.spreadsheets().get(spreadsheet_id).doit().await?;

                let spreadsheet = result.1;

                // Extract sheet information
                let sheet_info = spreadsheet
                    .sheets
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|sheet| {
                        let props = sheet.properties?;
                        let title = props.title?;
                        let grid_props = props.grid_properties?;

                        // Calculate the maximum range based on grid properties
                        let max_col = grid_props.column_count.unwrap_or(26) as u8;
                        let max_row = grid_props.row_count.unwrap_or(1000);
                        let max_range = format!("A1:{}{}", (b'A' + max_col - 1) as char, max_row);

                        Some(serde_json::json!({
                            "title": title,
                            "maxRange": max_range,
                        }))
                    })
                    .collect::<Vec<_>>();

                Ok(CallToolResponse {
                    content: vec![ToolResponseContent::Text {
                        text: serde_json::to_string(&sheet_info)?,
                    }],
                    is_error: None,
                    meta: None,
                })
            }
            .await;

            handle_result(result)
        })
    });

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
