use crate::{
    client::{get_drive_client, get_sheets_client},
    SheetsServer,
};
use async_mcp::{
    protocol::RequestOptions,
    transport::{ClientInMemoryTransport, ServerInMemoryTransport, Transport},
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sheet {
    a1_notation: String,
    sheet_id: u64,
    sheet_name: String,
}

async fn async_sheets_server(transport: ServerInMemoryTransport, access_token: String) {
    let server = SheetsServer::new(&access_token).build(transport).unwrap();
    server.listen().await.unwrap();
}

#[tokio::test]
async fn test_sheets_operations() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();

    let client_transport = ClientInMemoryTransport::new(move |t| {
        let token = access_token.clone();
        tokio::spawn(async move { async_sheets_server(t, token).await })
    });
    client_transport.open().await?;

    let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
    let client_clone = client.clone();
    let _client_handle = tokio::spawn(async move { client_clone.start().await });

    // Test read values
    let response = client
        .request(
            "read_values",
            Some(json!({
                "spreadsheet_id": "your-test-spreadsheet-id",
                "range": "Sheet1!A1:D10"
            })),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;
    println!("Read values response:\n{response}");

    Ok(())
}

#[tokio::test]
async fn test_google_sheets() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    let sheets = get_sheets_client(&access_token);

    let sheet_id = "1yO2ZVWb-EEhv-sbUFrJm5awOBYuFQlAzWHYNeflJses";

    // Try to read the spreadsheet
    let result = sheets.spreadsheets().get(sheet_id).doit().await?;
    println!("Spreadsheet data: {:?}", result);

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
