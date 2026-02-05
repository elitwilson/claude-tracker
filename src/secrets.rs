use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "claude-tracker";

pub fn store_secret(name: &str, value: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, name).context("failed to create keyring entry")?;
    entry
        .set_password(value)
        .context("failed to store secret in keychain")?;
    Ok(())
}

pub fn get_secret(name: &str) -> Result<String> {
    let entry = Entry::new(SERVICE_NAME, name).context("failed to create keyring entry")?;
    entry.get_password().map_err(|e| {
        anyhow::anyhow!(
            "secret '{}' not found (run `claude-tracker setup` to store it): {e}",
            name
        )
    })
}

#[cfg(test)]
mod tests;
