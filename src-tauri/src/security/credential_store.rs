use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keyring::Entry;

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

#[derive(Clone)]
pub struct KeyringStore {
    service_name: String,
}

impl KeyringStore {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn entry(&self, key: &str) -> anyhow::Result<Entry> {
        Ok(Entry::new(&self.service_name, key)?)
    }
}

impl SecretStore for KeyringStore {
    fn set_secret(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.entry(key)?.set_password(value)?;
        Ok(())
    }

    fn get_secret(&self, key: &str) -> anyhow::Result<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn delete_secret(&self, key: &str) -> anyhow::Result<()> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}
