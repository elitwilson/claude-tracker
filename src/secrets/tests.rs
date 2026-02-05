use keyring::Entry;
use super::*;

/// Drop guard that removes the test secret from the keychain when it goes
/// out of scope — even if the test panics.
struct TestSecret(String);

impl TestSecret {
    fn new(suffix: &str) -> Self {
        let name = format!("__test_{}", suffix);
        // Best-effort removal of any leftover from a prior run.
        if let Ok(entry) = Entry::new(SERVICE_NAME, &name) {
            let _ = entry.delete_password();
        }
        Self(name)
    }

    fn name(&self) -> &str {
        &self.0
    }
}

impl Drop for TestSecret {
    fn drop(&mut self) {
        if let Ok(entry) = Entry::new(SERVICE_NAME, &self.0) {
            let _ = entry.delete_password();
        }
    }
}

#[test]
fn store_and_retrieve_roundtrip() {
    let secret = TestSecret::new("roundtrip");

    store_secret(secret.name(), "hello-world").unwrap();

    assert_eq!(get_secret(secret.name()).unwrap(), "hello-world");
}

#[test]
fn get_missing_secret_returns_error() {
    let secret = TestSecret::new("missing");

    let result = get_secret(secret.name());

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found"));
}

#[test]
fn overwrite_replaces_value() {
    let secret = TestSecret::new("overwrite");

    store_secret(secret.name(), "first").unwrap();
    store_secret(secret.name(), "second").unwrap();

    assert_eq!(get_secret(secret.name()).unwrap(), "second");
}
