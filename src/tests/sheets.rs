use crate::{
    client::{get_drive_client, get_sheets_client},
    servers::sheets,
};
use async_mcp::{
    protocol::RequestOptions,
    transport::{ClientInMemoryTransport, ServerInMemoryTransport, Transport},
    types::CallToolRequest,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, env, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sheet {
    a1_notation: String,
    sheet_id: u64,
    sheet_name: String,
}

async fn async_sheets_server(transport: ServerInMemoryTransport) {
    println!("Starting sheets server...");
    let server = sheets::build(transport).unwrap();
    println!("Server built successfully");
    server.listen().await.unwrap();
}

#[tokio::test]
async fn test_sheets_operations() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    let spreadsheet_id = env::var("TEST_SPREADSHEET_ID").unwrap();

    let client_transport = ClientInMemoryTransport::new(move |t| {
        tokio::spawn(async move { async_sheets_server(t).await })
    });
    client_transport.open().await?;

    let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
    let client_clone = client.clone();
    let _client_handle = tokio::spawn(async move { client_clone.start().await });

    // Add a small delay to ensure server is ready
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let params = CallToolRequest {
        name: "read_values".to_string(),
        arguments: Some(HashMap::new()),
        meta: Some(json!({
            "access_token": access_token,
            "spreadsheet_id": spreadsheet_id,
            "sheet": "Sheet6"
        })),
    };

    // Test read values
    let response = client
        .request(
            "tools/call",
            Some(serde_json::to_value(&params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;

    // Add better error handling
    let response_obj: serde_json::Value = serde_json::from_str(&response.to_string())?;
    if let Some(error) = response_obj.get("error") {
        println!("Error reading sheet: {}", error);
        anyhow::bail!("Failed to read sheet: {}", error);
    }

    println!("Read values response:\n{response}");

    Ok(())
}

#[tokio::test]
async fn test_google_sheets() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    // let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    // let auth_service = GoogleAuthService::new(
    //     env::var("GOOGLE_CLIENT_ID").unwrap(),
    //     env::var("GOOGLE_CLIENT_SECRET").unwrap(),
    // )?;
    // let token_response = auth_service.refresh_token(&access_token).await?;
    // println!("Access token: {:?}", token_response);
    // let access_token = token_response.access_token;

    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    let sheets = get_sheets_client(&access_token);

    let spreadsheet_id = env::var("TEST_SPREADSHEET_ID").unwrap();

    // Try to read the spreadsheet
    let result = sheets.spreadsheets().get(&spreadsheet_id).doit().await?;
    // Extract sheet names and ranges
    if let Some(sheets) = result.1.sheets {
        for sheet in sheets {
            if let Some(properties) = sheet.properties {
                let sheet_title = properties.title.unwrap_or_default();
                let grid_props = properties.grid_properties.unwrap_or_default();
                let row_count = grid_props.row_count.unwrap_or(0);
                let column_count = grid_props.column_count.unwrap_or(0);

                println!(
                    "Sheet: {}\nRange: {}!A1:{}{}\n",
                    sheet_title,
                    sheet_title,
                    (b'A' + (column_count as u8) - 1) as char,
                    row_count
                );
            }
        }
    } else {
        println!("No sheets found.");
    }

    Ok(())
}

#[tokio::test]
async fn test_list_spreadsheet_details() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();

    let drive = get_drive_client(&access_token);
    let sheets = get_sheets_client(&access_token);

    let result = drive
        .files()
        .list()
        .q("mimeType='application/vnd.google-apps.spreadsheet'")
        .order_by("modifiedTime desc")
        .page_size(10)
        .doit()
        .await?;

    if let Some(files) = result.1.files {
        for file in files {
            let id = file.id.clone().unwrap_or_default();
            println!(
                "Spreadsheet: {} (ID: {})",
                file.name.unwrap_or_default(),
                id
            );

            // Get the content of each spreadsheet
            let spreadsheet = sheets.spreadsheets().get(&id).doit().await?;

            println!("Sheets in this spreadsheet:");
            for sheet in spreadsheet.1.sheets.unwrap_or_default() {
                if let Some(props) = sheet.properties {
                    println!("- Sheet name: {}", props.title.unwrap_or_default());
                }
            }
            println!("-------------------");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_sheet_operations() -> anyhow::Result<()> {
    dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    let spreadsheet_id = env::var("TEST_SPREADSHEET_ID").unwrap();

    let client_transport = ClientInMemoryTransport::new(move |t| {
        tokio::spawn(async move { async_sheets_server(t).await })
    });
    client_transport.open().await?;

    let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
    let client_clone = client.clone();
    let _client_handle = tokio::spawn(async move { client_clone.start().await });

    // Add a small delay to ensure server is ready
    tokio::time::sleep(Duration::from_millis(100)).await;

    // First get sheet info
    let get_info_params = CallToolRequest {
        name: "get_sheet_info".to_string(),
        arguments: None,
        meta: Some(json!({
            "access_token": access_token,
            "spreadsheet_id": spreadsheet_id,
        })),
    };

    let info_response = client
        .request(
            "tools/call",
            Some(serde_json::to_value(&get_info_params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;

    println!("Sheet info:\n{}", info_response);

    // Read the current value from A1
    let read_params = CallToolRequest {
        name: "read_values".to_string(),
        arguments: None,
        meta: Some(json!({
            "access_token": access_token,
            "spreadsheet_id": spreadsheet_id,
            "sheet": "Sheet1",
            "range": "A1"
        })),
    };

    let read_response = client
        .request(
            "tools/call",
            Some(serde_json::to_value(&read_params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;

    // After read_response
    println!("Initial read response:\n{}", read_response);

    // Parse the current value and increment it
    let read_value = serde_json::from_str::<serde_json::Value>(&read_response.to_string())?;
    println!("Parsed read value: {:?}", read_value);

    let current_value = read_value["content"][0]["text"]
        .as_str()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v["values"][0][0].as_str().map(String::from))
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let new_value = current_value + 1;

    // Write the incremented value back
    let mut args = HashMap::new();
    args.insert("values".to_string(), json!([[new_value.to_string()]]));
    args.insert("range".to_string(), json!("A1"));

    let write_params = CallToolRequest {
        name: "write_values".to_string(),
        arguments: Some(args),
        meta: Some(json!({
            "access_token": access_token,
            "spreadsheet_id": spreadsheet_id,
            "sheet": "Sheet1"
        })),
    };

    let write_response = client
        .request(
            "tools/call",
            Some(serde_json::to_value(&write_params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;

    println!("Write response:\n{}", write_response);

    // Verify the new value
    let verify_response = client
        .request(
            "tools/call",
            Some(serde_json::to_value(&read_params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;

    // After verify_response
    println!("Verify response:\n{}", verify_response);

    let verify_value = serde_json::from_str::<serde_json::Value>(&verify_response.to_string())?;
    println!("Parsed verify value: {:?}", verify_value);

    let updated_value = verify_value["content"][0]["text"]
        .as_str()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v["values"][0][0].as_str().map(String::from))
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);

    assert_eq!(
        updated_value, new_value,
        "Value was not updated correctly. Expected {}, got {}",
        new_value, updated_value
    );

    println!(
        "Successfully incremented value from {} to {}",
        current_value, new_value
    );

    Ok(())
}
