use crate::{client::get_drive_client, logging::init_logging, DriveServer};
use async_mcp::{
    protocol::RequestOptions,
    transport::{ClientInMemoryTransport, ServerInMemoryTransport, Transport},
};
use dotenv::dotenv;
use serde_json::json;
use std::{env, time::Duration};

async fn async_drive_server(transport: ServerInMemoryTransport, access_token: String) {
    let server = DriveServer::new(&access_token).build(transport).unwrap();
    server.listen().await.unwrap();
}

#[tokio::test]
async fn test_drive_operations() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let access_token = env::var("GOOGLE_ACCESS_TOKEN").unwrap();

    let client_transport = ClientInMemoryTransport::new(move |t| {
        let token = access_token.clone();
        tokio::spawn(async move { async_drive_server(t, token).await })
    });
    client_transport.open().await?;

    let client = async_mcp::client::ClientBuilder::new(client_transport.clone()).build();
    let client_clone = client.clone();
    let _client_handle = tokio::spawn(async move { client_clone.start().await });

    // Test list files
    let response = client
        .request(
            "list_files",
            Some(json!({
                "mime_type": "application/vnd.google-apps.folder",
                "page_size": 5
            })),
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
