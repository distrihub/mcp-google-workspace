use anyhow::Result;
use async_mcp::transport::ServerStdioTransport;
use clap::{Parser, Subcommand};
use mcp_google_workspace::{
    logging::init_logging,
    servers::{drive, sheets},
    GoogleAuthService,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Google Drive server
    Drive,
    /// Start the Google Sheets server
    Sheets,
    Refresh {
        /// Google OAuth client ID
        #[arg(long, env = "GOOGLE_CLIENT_ID")]
        client_id: String,
        /// Google OAuth client secret
        #[arg(long, env = "GOOGLE_CLIENT_SECRET")]
        client_secret: String,
        /// Refresh token
        #[arg(long, env = "GOOGLE_REFRESH_TOKEN")]
        refresh_token: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("debug");

    let cli = Cli::parse();

    match cli.command {
        Commands::Drive => {
            let server = drive::build(ServerStdioTransport)?;
            let server_handle = tokio::spawn(async move { server.listen().await });

            server_handle
                .await?
                .map_err(|e| anyhow::anyhow!("Drive server error: {:#?}", e))?;
        }
        Commands::Sheets => {
            let server = sheets::build(ServerStdioTransport)?;
            let server_handle = tokio::spawn(async move { server.listen().await });

            server_handle
                .await?
                .map_err(|e| anyhow::anyhow!("Sheets server error: {:#?}", e))?;
        }
        Commands::Refresh {
            client_id,
            client_secret,
            refresh_token,
        } => {
            let auth_service = GoogleAuthService::new(client_id, client_secret).unwrap();
            let token_response = auth_service.refresh_token(&refresh_token).await.unwrap();
            println!("Token response: {:#?}", token_response);
        }
    }

    Ok(())
}
