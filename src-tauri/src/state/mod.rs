use std::sync::Arc;

use reqwest::Client;

use crate::security::credential_store::SecretStore;

pub struct AppState {
    pub db_path: String,
    pub secrets: Arc<dyn SecretStore>,
    pub http_client: Client,
}
