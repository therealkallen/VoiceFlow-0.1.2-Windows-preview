use shared_protocol::ProviderPreset;
use speech_engine::{
    ProviderCredentialLookup, ResolvedProvider, resolve_provider_config_with_credential_lookup,
};
use std::env;
use std::fmt;
use std::sync::Arc;

const SERVICE_NAME: &str = "VoiceFlow Speech Input";

pub trait ProviderCredentialStore: Send + Sync {
    fn get(&self, preset: &ProviderPreset) -> ProviderCredentialLookup;
    fn set(&self, preset: &ProviderPreset, api_key: &str) -> Result<(), ProviderCredentialError>;
    fn delete(&self, preset: &ProviderPreset) -> Result<(), ProviderCredentialError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderCredentialErrorKind {
    Unavailable,
    EntryCreationFailed,
    WriteFailed,
    DeleteFailed,
    ReadAfterWriteFailed,
    Rejected,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderCredentialError {
    kind: ProviderCredentialErrorKind,
}

impl ProviderCredentialError {
    pub fn unavailable() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::Unavailable,
        }
    }

    pub fn entry_creation_failed() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::EntryCreationFailed,
        }
    }

    pub fn write_failed() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::WriteFailed,
        }
    }

    pub fn delete_failed() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::DeleteFailed,
        }
    }

    pub fn read_after_write_failed() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::ReadAfterWriteFailed,
        }
    }

    pub fn rejected() -> Self {
        Self {
            kind: ProviderCredentialErrorKind::Rejected,
        }
    }

    pub fn kind(&self) -> ProviderCredentialErrorKind {
        self.kind
    }

    pub fn safe_message(&self) -> &'static str {
        match self.kind {
            ProviderCredentialErrorKind::Unavailable => "credential store unavailable",
            ProviderCredentialErrorKind::EntryCreationFailed => {
                "credential store entry could not be created"
            }
            ProviderCredentialErrorKind::WriteFailed => "credential store write failed",
            ProviderCredentialErrorKind::DeleteFailed => "credential store delete failed",
            ProviderCredentialErrorKind::ReadAfterWriteFailed => {
                "credential store did not confirm the saved API key"
            }
            ProviderCredentialErrorKind::Rejected => "credential store rejected the request",
        }
    }

    pub fn safe_category(&self) -> &'static str {
        match self.kind {
            ProviderCredentialErrorKind::Unavailable => "backend_unavailable",
            ProviderCredentialErrorKind::EntryCreationFailed => "entry_creation_failed",
            ProviderCredentialErrorKind::WriteFailed => "write_failed",
            ProviderCredentialErrorKind::DeleteFailed => "delete_failed",
            ProviderCredentialErrorKind::ReadAfterWriteFailed => "read_after_write_failed",
            ProviderCredentialErrorKind::Rejected => "rejected",
        }
    }
}

impl fmt::Debug for ProviderCredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderCredentialError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for ProviderCredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message())
    }
}

#[derive(Clone, Debug, Default)]
pub struct KeyringProviderCredentialStore;

impl KeyringProviderCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl ProviderCredentialStore for KeyringProviderCredentialStore {
    fn get(&self, preset: &ProviderPreset) -> ProviderCredentialLookup {
        let entry = match keyring::Entry::new(SERVICE_NAME, account_for_preset(preset)) {
            Ok(entry) => entry,
            Err(_) => return ProviderCredentialLookup::StoreError,
        };
        match entry.get_password() {
            Ok(value) if !value.trim().is_empty() => ProviderCredentialLookup::Found(value),
            Ok(_) => ProviderCredentialLookup::Missing,
            Err(error) if matches!(error, keyring::Error::NoEntry) => {
                ProviderCredentialLookup::Missing
            }
            Err(_) => ProviderCredentialLookup::StoreError,
        }
    }

    fn set(&self, preset: &ProviderPreset, api_key: &str) -> Result<(), ProviderCredentialError> {
        let trimmed = api_key.trim();
        if trimmed.is_empty() {
            return Err(ProviderCredentialError::rejected());
        }
        let entry = keyring::Entry::new(SERVICE_NAME, account_for_preset(preset))
            .map_err(|_| ProviderCredentialError::entry_creation_failed())?;
        entry
            .set_password(trimmed)
            .map_err(|_| ProviderCredentialError::write_failed())
    }

    fn delete(&self, preset: &ProviderPreset) -> Result<(), ProviderCredentialError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account_for_preset(preset))
            .map_err(|_| ProviderCredentialError::entry_creation_failed())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(error) if matches!(error, keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(ProviderCredentialError::delete_failed()),
        }
    }
}

pub fn production_provider_credential_store() -> Arc<dyn ProviderCredentialStore> {
    Arc::new(KeyringProviderCredentialStore::new())
}

#[derive(Clone, Debug, Default)]
pub struct MissingProviderCredentialStore;

impl ProviderCredentialStore for MissingProviderCredentialStore {
    fn get(&self, _preset: &ProviderPreset) -> ProviderCredentialLookup {
        ProviderCredentialLookup::Missing
    }

    fn set(&self, _preset: &ProviderPreset, _api_key: &str) -> Result<(), ProviderCredentialError> {
        Err(ProviderCredentialError::unavailable())
    }

    fn delete(&self, _preset: &ProviderPreset) -> Result<(), ProviderCredentialError> {
        Err(ProviderCredentialError::unavailable())
    }
}

pub fn resolve_provider_config_with_credentials(
    settings: &shared_protocol::RuntimeSettings,
    store: &dyn ProviderCredentialStore,
) -> ResolvedProvider {
    resolve_provider_config_with_credential_lookup(
        settings,
        |key| env::var(key).ok(),
        |preset| store.get(preset),
    )
}

fn account_for_preset(preset: &ProviderPreset) -> &'static str {
    match preset {
        ProviderPreset::Bailian => "provider-api-key/bailian/v1",
        ProviderPreset::VolcengineArk => "provider-api-key/volcengine-ark/v1",
        ProviderPreset::TencentHunyuan => "provider-api-key/tencent-hunyuan/v1",
        ProviderPreset::CustomOpenAiCompatible => "provider-api-key/custom-openai-compatible/v1",
    }
}

#[cfg(test)]
#[derive(Clone, Default)]
pub struct InMemoryProviderCredentialStore {
    keys: Arc<std::sync::Mutex<std::collections::HashMap<ProviderPreset, String>>>,
    fail_reads: Arc<std::sync::atomic::AtomicBool>,
    fail_writes: Arc<std::sync::atomic::AtomicBool>,
    lookup_count: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl InMemoryProviderCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_read_error(&self, enabled: bool) {
        self.fail_reads
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_write_error(&self, enabled: bool) {
        self.fail_writes
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn lookup_count(&self) -> usize {
        self.lookup_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
impl ProviderCredentialStore for InMemoryProviderCredentialStore {
    fn get(&self, preset: &ProviderPreset) -> ProviderCredentialLookup {
        self.lookup_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_reads.load(std::sync::atomic::Ordering::SeqCst) {
            return ProviderCredentialLookup::StoreError;
        }
        self.keys
            .lock()
            .ok()
            .and_then(|keys| keys.get(preset).cloned())
            .map(ProviderCredentialLookup::Found)
            .unwrap_or(ProviderCredentialLookup::Missing)
    }

    fn set(&self, preset: &ProviderPreset, api_key: &str) -> Result<(), ProviderCredentialError> {
        if self.fail_writes.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ProviderCredentialError::write_failed());
        }
        self.keys
            .lock()
            .map_err(|_| ProviderCredentialError::unavailable())?
            .insert(preset.clone(), api_key.trim().to_string());
        Ok(())
    }

    fn delete(&self, preset: &ProviderPreset) -> Result<(), ProviderCredentialError> {
        if self.fail_writes.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ProviderCredentialError::delete_failed());
        }
        self.keys
            .lock()
            .map_err(|_| ProviderCredentialError::unavailable())?
            .remove(preset);
        Ok(())
    }
}

#[cfg(test)]
impl fmt::Debug for InMemoryProviderCredentialStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let providers = self
            .keys
            .lock()
            .map(|keys| keys.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        formatter
            .debug_struct("InMemoryProviderCredentialStore")
            .field("entry_count", &providers.len())
            .field("providers", &providers)
            .field("lookup_count", &self.lookup_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_debug_redacts_api_keys() {
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "sentinel-secret-key")
            .expect("test store should accept key");

        let debug = format!("{store:?}");

        assert!(debug.contains("Bailian"));
        assert!(!debug.contains("sentinel-secret-key"));
    }

    #[test]
    fn in_memory_store_roundtrips_and_isolates_provider_keys() {
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "bailian-secret")
            .expect("bailian key should save");
        store
            .set(&ProviderPreset::VolcengineArk, "volcengine-secret")
            .expect("volcengine key should save");
        store
            .set(&ProviderPreset::TencentHunyuan, "tencent-secret")
            .expect("tencent key should save");
        store
            .set(&ProviderPreset::CustomOpenAiCompatible, "custom-secret")
            .expect("custom key should save");

        assert_eq!(
            store.get(&ProviderPreset::Bailian),
            ProviderCredentialLookup::Found("bailian-secret".to_string())
        );
        assert_eq!(
            store.get(&ProviderPreset::VolcengineArk),
            ProviderCredentialLookup::Found("volcengine-secret".to_string())
        );
        assert_eq!(
            store.get(&ProviderPreset::TencentHunyuan),
            ProviderCredentialLookup::Found("tencent-secret".to_string())
        );
        assert_eq!(
            store.get(&ProviderPreset::CustomOpenAiCompatible),
            ProviderCredentialLookup::Found("custom-secret".to_string())
        );

        store
            .delete(&ProviderPreset::TencentHunyuan)
            .expect("tencent key should clear");
        assert_eq!(
            store.get(&ProviderPreset::TencentHunyuan),
            ProviderCredentialLookup::Missing
        );
        assert_eq!(
            store.get(&ProviderPreset::VolcengineArk),
            ProviderCredentialLookup::Found("volcengine-secret".to_string())
        );
    }

    #[test]
    #[ignore = "manual Windows Credential Manager verification"]
    fn windows_keyring_roundtrips_temporary_secret() {
        let unique = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        );
        let account = format!("provider-api-key/manual-roundtrip/{unique}");
        let secret = format!("temporary-secret-{unique}");

        let entry = keyring::Entry::new(SERVICE_NAME, &account)
            .unwrap_or_else(|_| panic!("windows keyring roundtrip failed: entry_creation_failed"));
        let mut result = match entry.set_password(&secret) {
            Ok(()) => Ok(()),
            Err(_) => Err("write_failed"),
        };
        if result.is_ok() {
            result = match entry.get_password() {
                Ok(value) if value == secret => Ok(()),
                Ok(_) => Err("read_mismatch"),
                Err(_) => Err("read_failed"),
            };
        }
        let _ = entry.delete_credential();
        if let Err(category) = result {
            panic!("windows keyring roundtrip failed: {category}");
        }
    }
}
