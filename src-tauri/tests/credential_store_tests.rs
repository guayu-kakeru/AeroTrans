use aerotrans_lib::security::credential_store::{InMemoryStore, SecretStore};

#[test]
fn stores_and_reads_api_key() {
    let store = InMemoryStore::default();
    store.set_secret("openai_api_key", "sk-test").unwrap();

    let loaded = store.get_secret("openai_api_key").unwrap();
    assert_eq!(loaded.as_deref(), Some("sk-test"));
}

#[test]
fn deletes_api_key() {
    let store = InMemoryStore::default();
    store.set_secret("openai_api_key", "sk-test").unwrap();
    store.delete_secret("openai_api_key").unwrap();

    let loaded = store.get_secret("openai_api_key").unwrap();
    assert!(loaded.is_none());
}
