# Google Drive & Sheets MCP Server

A Model Context Protocol (MCP) server built in Rust for interacting with Google Drive and Google Sheets. This tool provides MCP-compatible interfaces for Google Workspace services, allowing it to be used as part of larger agent workflows and compositions.

## Features

### Google Drive Operations
- List files in Google Drive with filtering options:
  - Filter by MIME type
  - Custom search queries
  - Configurable page size
  - Custom ordering

### Google Sheets Operations
- Read data from Google Sheets with options:
  - Specify range
  - Choose major dimension (ROWS or COLUMNS)
- Write data to Google Sheets
- Create new spreadsheets with:
  - Custom title
  - Multiple sheets
- Clear values from ranges in spreadsheets

## MCP Integration

This server implements the Model Context Protocol (MCP), making it compatible with agent frameworks like [Distri](https://github.com/distrihub/distri). Each service exposes its capabilities as MCP tools:

### Drive Tools
- `list_files`: List and filter Drive files with customizable parameters
- Available capabilities exposed via `resources/list` endpoint

### Sheets Tools
- `read_values`: Read spreadsheet data with dimension control
- `write_values`: Write data to spreadsheets
- `create_spreadsheet`: Create new spreadsheets
- `clear_values`: Clear ranges in spreadsheets
- Available capabilities exposed via `resources/list` endpoint

## Prerequisites

- Rust (latest stable version)
- Google Cloud Project with Drive and Sheets APIs enabled
- OAuth 2.0 credentials configured for your Google Cloud Project

## Installation

Install the `mcp-google` binary directly from GitHub using Cargo:
```bash
cargo install --git https://github.com/distrihub/mcp-google-workspace.git
```

This will install the `mcp-google` command to your system.

## Configuration

Before using the server, you need to:

1. Set up a Google Cloud Project
2. Enable Google Drive and Google Sheets APIss
3. Create OAuth 2.0 credentials
4. Set up your environment variables:
   - `ACCESS_TOKEN`: Your Google OAuth access token
   - `GOOGLE_CLIENT_ID`: Your OAuth client ID
   - `GOOGLE_CLIENT_SECRET`: Your OAuth client secret
   - `GOOGLE_REFRESH_TOKEN`: Your OAuth refresh token

## Usage

### As MCP Server

The servers can be started independently and will communicate using the MCP protocol over stdio:

Start the Drive MCP server:
```bash
mcp-google drive --access-token <your-access-token>
```

Start the Sheets MCP server:
```bash
mcp-google sheets --access-token <your-access-token>
```

### Using with Distri

This server can be used as part of a Distri agent configuration:

```yaml
agents:
  google_workspace:
    drive:
      type: mcp
      command: ["mcp-google", "drive", "--access-token", "${ACCESS_TOKEN}"]
    sheets:
      type: mcp
      command: ["mcp-google", "sheets", "--access-token", "${ACCESS_TOKEN}"]
```

### Token Management

Refresh your OAuth token:
```bash
mcp-google refresh \
  --client-id <your-client-id> \
  --client-secret <your-client-secret> \
  --refresh-token <your-refresh-token>
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Note

This is an MCP-compatible server that interacts with Google services. Make sure you have appropriate permissions and credentials before using the tool.