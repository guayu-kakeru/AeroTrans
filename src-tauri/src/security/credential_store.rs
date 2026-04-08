use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait SecretStore: Send + Sync {
    fn set_secret(&self, key: &str, value: &str) -> anyhow::Result<()>;
    fn get_secret(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn delete_secret(&self, key: &str) -> anyhow::Result<()>;
}

#[derive(Default, Clone)]
pub struct InMemoryStore {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretStore for InMemoryStore {
    fn set_secret(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.data
            .lock()
            .expect("in-memory store mutex poisoned")
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .data
            .lock()
            .expect("in-memory store mutex poisoned")
            .get(key)
            .cloned())
    }

    fn delete_secret(&self, key: &str) -> anyhow::Result<()> {
        self.data
            .lock()
            .expect("in-memory store mutex poisoned")
            .remove(key);
        Ok(())
    }
}
