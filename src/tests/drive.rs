use crate::{client::get_drive_client, logging::init_logging, servers::drive};
use async_mcp::{
    protocol::RequestOptions,
    transport::{ClientInMemoryTransport, ServerInMemoryTransport, Transport},
    types::CallToolRequest,
};
use dotenv::dotenv;
use serde_json::json;
use std::{collections::HashMap, env, time::Duration};

async fn async_drive_server(transport: ServerInMemoryTransport) {
    let server = drive::build(transport).unwrap();
    server.listen().await.unwrap();
}

#[tokio::test]
async fn test_drive_operations() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();

    let client_transport = ClientInMemoryTransport::new(move |t| {
        tokio::spawn(async move { async_drive_server(t).await })
    });
    client_transport.open().await?;

    let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
    let client_clone = client.clone();
    let _client_handle = tokio::spawn(async move { client_clone.start().await });

    let params = CallToolRequest {
        name: "list_files".to_string(),
        arguments: Some(HashMap::from([
            (
                "mime_type".to_string(),
                "application/vnd.google-apps.folder".to_string().into(),
            ),
            ("page_size".to_string(), 5.into()),
        ])),
        meta: Some(json!({
            "access_token": access_token
        })),
    };
    // Test list files
    let response = client
        .request(
            "list_files",
            Some(serde_json::to_value(&params).unwrap()),
            RequestOptions::default().timeout(Duration::from_secs(5)),
        )
        .await?;
    println!("List files response:\n{response}");

    Ok(())
}

#[tokio::test]
async fn test_list_spreadsheets() -> Result<(), Box<dyn std::error::Error>> {
    init_logging("debug");
    dotenv().ok();

    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();
    let drive = get_drive_client(&access_token);

    // Add more detailed query parameters and debug output
    let result = drive
        .files()
        .list()
        .q("mimeType='application/vnd.google-apps.spreadsheet'")
        .order_by("modifiedTime desc")
        .page_size(10) // Limit to 10 results for testing
        .doit()
        .await?;

    if let Some(files) = result.1.files {
        for file in files {
            println!(
                "Spreadsheet: {} (ID: {})",
                file.name.unwrap_or_default(),
                file.id.unwrap_or_default()
            );
            println!(
                "Last modified: {:?}",
                file.modified_time.unwrap_or_default()
            );
            println!("-------------------");
        }
    }

    Ok(())
}
