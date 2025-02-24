mod auth;
pub mod client;
pub mod logging;
mod servers;

#[cfg(test)]
mod tests;

// Re-export servers
pub use auth::GoogleAuthService;
pub use servers::drive::DriveServer;
pub use servers::sheets::SheetsServer;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Environment variable missing: {0}")]
    EnvVarMissing(String),

    #[error("Google API error: {0}")]
    GoogleApi(String),

    #[error("Token parse error: {0}")]
    TokenParse(String),

    #[error("User info error: {0}")]
    UserInfo(String),

    #[error("JWT error: {0}")]
    Jwt(String),
}
