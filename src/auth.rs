use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::debug;

use crate::InvokeError;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub token_type: String,
}

#[derive(Clone)]
pub struct GoogleAuthService {
    pub client: Client,
    pub google_client_id: String,
    pub google_client_secret: String,
}
impl Default for GoogleAuthService {
    fn default() -> Self {
        let google_client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| InvokeError::EnvVarMissing("GOOGLE_CLIENT_ID".to_string()))
            .unwrap();
        let google_client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| InvokeError::EnvVarMissing("GOOGLE_CLIENT_SECRET".to_string()))
            .unwrap();

        Self::new(google_client_id, google_client_secret).unwrap()
    }
}
impl GoogleAuthService {
    pub fn new(client_id: String, client_secret: String) -> Result<Self, InvokeError> {
        Ok(Self {
            client: Client::new(),
            google_client_id: client_id,
            google_client_secret: client_secret,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, InvokeError> {
        let payload = json!({
            "client_id": self.google_client_id,
            "client_secret": self.google_client_secret,
            "refresh_token": refresh_token,
            "grant_type": "refresh_token"
        });

        self.exchange_token(&payload).await
    }

    async fn exchange_token(
        &self,
        payload: &serde_json::Value,
    ) -> Result<TokenResponse, InvokeError> {
        debug!("Token exchange payload: {:?}", payload);

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .json(payload)
            .send()
            .await
            .map_err(|e| InvokeError::GoogleApi(e.to_string()))?;

        if !response.status().is_success() {
            let error = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(InvokeError::GoogleApi(error));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| InvokeError::TokenParse(e.to_string()))
    }
}
