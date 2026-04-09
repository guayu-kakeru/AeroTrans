use reqwest::Client;
use std::sync::{Arc, Mutex};

use crate::security::credential_store::SecretStore;

pub struct AppState {
    pub db_path: String,
    pub secrets: Arc<dyn SecretStore>,
    pub http_client: Client,
    pub spotlight_hotkey_id: Mutex<Option<u32>>,
    pub selection_hotkey_id: Mutex<Option<u32>>,
    pub spotlight_hotkey_str: Mutex<Option<String>>,
    pub selection_hotkey_str: Mutex<Option<String>>,
}
