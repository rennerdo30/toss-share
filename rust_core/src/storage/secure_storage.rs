//! Platform secure storage for device identity keys
//!
//! Provides platform-specific secure storage using:
//! - macOS/iOS: Keychain Services (security-framework crate)
//! - Windows: Credential Manager (windows crate)
//! - Linux: Secret Service API (secret-service crate)
//! - Android: Android Keystore (requires JNI implementation)

use crate::error::CryptoError;
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

#[cfg(target_os = "windows")]
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::Foundation::ERROR_NOT_FOUND,
    Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    },
};

/// Service name for storing identity keys
const SERVICE_NAME: &str = "com.toss.device.identity";

/// Key name for device identity private key
const IDENTITY_KEY_NAME: &str = "device_identity_key";

/// Key name for storage encryption key (used to encrypt session keys)
const STORAGE_ENCRYPTION_KEY_NAME: &str = "storage_encryption_key";

/// Platform-agnostic secure storage trait
pub trait SecureStorage {
    /// Store a value securely
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError>;

    /// Retrieve a value from secure storage
    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError>;

    /// Delete a value from secure storage
    fn delete(&self, key: &str) -> Result<(), CryptoError>;
}

/// Store device identity key securely
pub fn store_identity_key(key: &[u8; 32]) -> Result<(), CryptoError> {
    let storage = get_platform_storage()?;
    storage.store(IDENTITY_KEY_NAME, key)
}

/// Retrieve device identity key from secure storage
pub fn retrieve_identity_key() -> Result<Option<[u8; 32]>, CryptoError> {
    let storage = get_platform_storage()?;
    match storage.retrieve(IDENTITY_KEY_NAME)? {
        Some(bytes) => {
            if bytes.len() != 32 {
                return Err(CryptoError::InvalidKey);
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Some(key))
        }
        None => Ok(None),
    }
}

/// Delete device identity key from secure storage
pub fn delete_identity_key() -> Result<(), CryptoError> {
    let storage = get_platform_storage()?;
    storage.delete(IDENTITY_KEY_NAME)
}

/// Get or generate the storage encryption key
/// This key is used to encrypt session keys before storing in SQLite
pub fn get_or_create_storage_encryption_key() -> Result<[u8; 32], CryptoError> {
    let storage = get_platform_storage()?;

    // Try to retrieve existing key
    if let Some(bytes) = storage.retrieve(STORAGE_ENCRYPTION_KEY_NAME)? {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    // Generate new key if not found
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

    // Store the new key
    storage.store(STORAGE_ENCRYPTION_KEY_NAME, &key)?;

    Ok(key)
}

/// Encrypt data using the storage encryption key
/// Returns: nonce (12 bytes) || ciphertext
pub fn encrypt_for_storage(plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use rand::RngCore;

    let key = get_or_create_storage_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::InvalidKey)?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data using the storage encryption key
/// Input format: nonce (12 bytes) || ciphertext
pub fn decrypt_from_storage(encrypted: &[u8]) -> Result<Vec<u8>, CryptoError> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    if encrypted.len() < 12 {
        return Err(CryptoError::Decryption(
            "encrypted data too short".to_string(),
        ));
    }

    let key = get_or_create_storage_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| CryptoError::InvalidKey)?;

    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decryption(e.to_string()))
}

/// Get platform-specific secure storage implementation
fn get_platform_storage() -> Result<Box<dyn SecureStorage>, CryptoError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(MacOSKeychainStorage::new(SERVICE_NAME)?))
    }

    #[cfg(target_os = "ios")]
    {
        Ok(Box::new(MacOSKeychainStorage::new(SERVICE_NAME)?))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(WindowsCredentialStorage::new(SERVICE_NAME)?))
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(LinuxSecretStorage::new(SERVICE_NAME)?))
    }

    #[cfg(target_os = "android")]
    {
        Ok(Box::new(AndroidKeystoreStorage::new(SERVICE_NAME)?))
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        // Fallback to in-memory storage for unsupported platforms
        Ok(Box::new(MemoryStorage::new()))
    }
}

// Platform-specific implementations

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct MacOSKeychainStorage {
    service: String,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl MacOSKeychainStorage {
    fn new(service: &str) -> Result<Self, CryptoError> {
        Ok(Self {
            service: service.to_string(),
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
impl SecureStorage for MacOSKeychainStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        // Use security-framework to store in macOS Keychain
        set_generic_password(&self.service, key, value)
            .map_err(|e| CryptoError::Storage(format!("Keychain store failed: {}", e)))
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        // Use security-framework to retrieve from macOS Keychain
        match get_generic_password(&self.service, key) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) => {
                // Check if the error is "item not found"
                let err_string = e.to_string();
                if err_string.contains("not found") || err_string.contains("-25300") {
                    Ok(None)
                } else {
                    Err(CryptoError::Storage(format!(
                        "Keychain retrieve failed: {}",
                        e
                    )))
                }
            }
        }
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        // Use security-framework to delete from macOS Keychain
        match delete_generic_password(&self.service, key) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Ignore "item not found" errors on delete
                let err_string = e.to_string();
                if err_string.contains("not found") || err_string.contains("-25300") {
                    Ok(())
                } else {
                    Err(CryptoError::Storage(format!(
                        "Keychain delete failed: {}",
                        e
                    )))
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct WindowsCredentialStorage {
    service: String,
}

#[cfg(target_os = "windows")]
impl WindowsCredentialStorage {
    fn new(service: &str) -> Result<Self, CryptoError> {
        Ok(Self {
            service: service.to_string(),
        })
    }

    fn make_target_name(&self, key: &str) -> Vec<u16> {
        let target = format!("{}:{}", self.service, key);
        target.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(target_os = "windows")]
impl SecureStorage for WindowsCredentialStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        use std::ptr;

        let target_name = self.make_target_name(key);
        let user_name: Vec<u16> = "toss_user"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let cred = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target_name.as_ptr() as *mut u16),
            Comment: PWSTR::null(),
            LastWritten: Default::default(),
            CredentialBlobSize: value.len() as u32,
            CredentialBlob: value.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: ptr::null_mut(),
            TargetAlias: PWSTR::null(),
            UserName: PWSTR(user_name.as_ptr() as *mut u16),
        };

        unsafe {
            CredWriteW(&cred, 0)
                .map_err(|e| CryptoError::Storage(format!("Credential write failed: {}", e)))
        }
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        use std::slice;

        let target_name = self.make_target_name(key);
        let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        unsafe {
            match CredReadW(
                PCWSTR(target_name.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut cred_ptr,
            ) {
                Ok(()) => {
                    let cred = &*cred_ptr;
                    let blob_size = cred.CredentialBlobSize as usize;
                    let result = if blob_size > 0 && !cred.CredentialBlob.is_null() {
                        let blob = slice::from_raw_parts(cred.CredentialBlob, blob_size);
                        Some(blob.to_vec())
                    } else {
                        None
                    };
                    CredFree(cred_ptr as *mut _);
                    Ok(result)
                }
                Err(e) => {
                    // Check for "not found" error
                    if e.code() == ERROR_NOT_FOUND.into() {
                        Ok(None)
                    } else {
                        Err(CryptoError::Storage(format!(
                            "Credential read failed: {}",
                            e
                        )))
                    }
                }
            }
        }
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        let target_name = self.make_target_name(key);

        unsafe {
            match CredDeleteW(PCWSTR(target_name.as_ptr()), CRED_TYPE_GENERIC, 0) {
                Ok(()) => Ok(()),
                Err(e) => {
                    // Ignore "not found" error on delete
                    if e.code() == ERROR_NOT_FOUND.into() {
                        Ok(())
                    } else {
                        Err(CryptoError::Storage(format!(
                            "Credential delete failed: {}",
                            e
                        )))
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
struct LinuxSecretStorage {
    service: String,
}

#[cfg(target_os = "linux")]
impl LinuxSecretStorage {
    fn new(service: &str) -> Result<Self, CryptoError> {
        Ok(Self {
            service: service.to_string(),
        })
    }

    #[allow(dead_code)]
    fn make_attributes(&self, _key: &str) -> HashMap<&str, &str> {
        let mut attrs = HashMap::new();
        attrs.insert("application", "toss");
        // Note: We can't return references to local Strings, so we use static key names
        attrs
    }
}

#[cfg(target_os = "linux")]
impl SecureStorage for LinuxSecretStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        // Use blocking runtime for secret-service which is async
        use tokio::runtime::Handle;

        // Try to use existing runtime, or create a new one for blocking
        let result = if let Ok(_handle) = Handle::try_current() {
            // We're in an async context, need to use spawn_blocking
            std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| CryptoError::Storage(format!("Runtime error: {}", e)))?;

                    rt.block_on(async { self.store_async(key, value).await })
                })
                .join()
                .unwrap()
            })
        } else {
            // Create a new runtime for this operation
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| CryptoError::Storage(format!("Runtime error: {}", e)))?;

            rt.block_on(async { self.store_async(key, value).await })
        };

        result
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        // Create a new runtime for this operation
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CryptoError::Storage(format!("Runtime error: {}", e)))?;

        rt.block_on(async { self.retrieve_async(key).await })
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        // Create a new runtime for this operation
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CryptoError::Storage(format!("Runtime error: {}", e)))?;

        rt.block_on(async { self.delete_async(key).await })
    }
}

#[cfg(target_os = "linux")]
impl LinuxSecretStorage {
    async fn store_async(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        use secret_service::{EncryptionType, SecretService};

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| CryptoError::Storage(format!("Secret Service connect failed: {}", e)))?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|e| CryptoError::Storage(format!("Get default collection failed: {}", e)))?;

        // Unlock collection if needed
        if collection.is_locked().await.unwrap_or(true) {
            collection
                .unlock()
                .await
                .map_err(|e| CryptoError::Storage(format!("Unlock collection failed: {}", e)))?;
        }

        let mut attributes = HashMap::new();
        attributes.insert("application", "toss");
        attributes.insert("service", &self.service);
        attributes.insert("key", key);

        let label = format!("Toss: {}", key);
        collection
            .create_item(
                &label,
                attributes,
                value,
                true,         // replace
                "text/plain", // content_type
            )
            .await
            .map_err(|e| CryptoError::Storage(format!("Create item failed: {}", e)))?;

        Ok(())
    }

    async fn retrieve_async(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        use secret_service::{EncryptionType, SecretService};

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| CryptoError::Storage(format!("Secret Service connect failed: {}", e)))?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|e| CryptoError::Storage(format!("Get default collection failed: {}", e)))?;

        // Unlock collection if needed
        if collection.is_locked().await.unwrap_or(true) {
            collection
                .unlock()
                .await
                .map_err(|e| CryptoError::Storage(format!("Unlock collection failed: {}", e)))?;
        }

        let mut attributes = HashMap::new();
        attributes.insert("application", "toss");
        attributes.insert("service", &self.service as &str);
        attributes.insert("key", key);

        let items = collection
            .search_items(attributes)
            .await
            .map_err(|e| CryptoError::Storage(format!("Search items failed: {}", e)))?;

        if let Some(item) = items.first() {
            // Unlock item if needed
            if item.is_locked().await.unwrap_or(true) {
                item.unlock()
                    .await
                    .map_err(|e| CryptoError::Storage(format!("Unlock item failed: {}", e)))?;
            }

            let secret = item
                .get_secret()
                .await
                .map_err(|e| CryptoError::Storage(format!("Get secret failed: {}", e)))?;

            Ok(Some(secret))
        } else {
            Ok(None)
        }
    }

    async fn delete_async(&self, key: &str) -> Result<(), CryptoError> {
        use secret_service::{EncryptionType, SecretService};

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| CryptoError::Storage(format!("Secret Service connect failed: {}", e)))?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|e| CryptoError::Storage(format!("Get default collection failed: {}", e)))?;

        // Unlock collection if needed
        if collection.is_locked().await.unwrap_or(true) {
            collection
                .unlock()
                .await
                .map_err(|e| CryptoError::Storage(format!("Unlock collection failed: {}", e)))?;
        }

        let mut attributes = HashMap::new();
        attributes.insert("application", "toss");
        attributes.insert("service", &self.service as &str);
        attributes.insert("key", key);

        let items = collection
            .search_items(attributes)
            .await
            .map_err(|e| CryptoError::Storage(format!("Search items failed: {}", e)))?;

        for item in items {
            item.delete()
                .await
                .map_err(|e| CryptoError::Storage(format!("Delete item failed: {}", e)))?;
        }

        Ok(())
    }
}

/// Android secure storage using file-based encryption
///
/// On Android, the encryption key is managed by Flutter via Android Keystore.
/// This Rust implementation stores encrypted data in files.
/// The encryption key is passed from Dart when initializing secure storage.
#[cfg(target_os = "android")]
struct AndroidKeystoreStorage {
    service: String,
    data_dir: std::path::PathBuf,
}

#[cfg(target_os = "android")]
static ANDROID_ENCRYPTION_KEY: once_cell::sync::OnceCell<[u8; 32]> =
    once_cell::sync::OnceCell::new();

#[cfg(target_os = "android")]
impl AndroidKeystoreStorage {
    fn new(service: &str) -> Result<Self, CryptoError> {
        // Get Android data directory from environment or use default
        let data_dir = std::env::var("TOSS_DATA_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                // Fallback to app-specific directory
                std::path::PathBuf::from("/data/data/dev.renner.toss/files/toss_secure")
            });

        // Create directory if it doesn't exist
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).map_err(|e| {
                CryptoError::Storage(format!("Failed to create secure storage directory: {}", e))
            })?;
        }

        Ok(Self {
            service: service.to_string(),
            data_dir,
        })
    }

    fn get_file_path(&self, key: &str) -> std::path::PathBuf {
        // Use SHA256 hash of key as filename to avoid filesystem issues
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.service.as_bytes());
        hasher.update(b":");
        hasher.update(key.as_bytes());
        let hash = hasher.finalize();
        let filename = hex::encode(&hash[..16]); // Use first 16 bytes (32 hex chars)
        self.data_dir.join(format!("{}.enc", filename))
    }

    fn get_encryption_key() -> Result<[u8; 32], CryptoError> {
        ANDROID_ENCRYPTION_KEY.get().copied().ok_or_else(|| {
            CryptoError::Storage(
                "Android encryption key not set. Call set_android_encryption_key first."
                    .to_string(),
            )
        })
    }

    fn encrypt_data(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use rand::RngCore;

        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::Encryption(e.to_string()))?;

        // Format: nonce (12 bytes) || ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    fn decrypt_data(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        if encrypted.len() < 12 {
            return Err(CryptoError::Decryption("Data too short".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;

        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptoError::Decryption(e.to_string()))
    }
}

#[cfg(target_os = "android")]
impl SecureStorage for AndroidKeystoreStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        let encryption_key = Self::get_encryption_key()?;
        let encrypted = Self::encrypt_data(value, &encryption_key)?;

        let file_path = self.get_file_path(key);
        std::fs::write(&file_path, &encrypted)
            .map_err(|e| CryptoError::Storage(format!("Failed to write secure file: {}", e)))?;

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        let file_path = self.get_file_path(key);

        if !file_path.exists() {
            return Ok(None);
        }

        let encrypted = std::fs::read(&file_path)
            .map_err(|e| CryptoError::Storage(format!("Failed to read secure file: {}", e)))?;

        let encryption_key = Self::get_encryption_key()?;
        let decrypted = Self::decrypt_data(&encrypted, &encryption_key)?;

        Ok(Some(decrypted))
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        let file_path = self.get_file_path(key);

        if file_path.exists() {
            std::fs::remove_file(&file_path).map_err(|e| {
                CryptoError::Storage(format!("Failed to delete secure file: {}", e))
            })?;
        }

        Ok(())
    }
}

/// Set the Android encryption key from Flutter/Dart
/// This key should be retrieved from Android Keystore via Flutter platform channel
#[cfg(target_os = "android")]
pub fn set_android_encryption_key(key: [u8; 32]) -> Result<(), CryptoError> {
    ANDROID_ENCRYPTION_KEY
        .set(key)
        .map_err(|_| CryptoError::Storage("Android encryption key already set".to_string()))
}

/// Set the Android data directory from Flutter/Dart
#[cfg(target_os = "android")]
pub fn set_android_data_dir(dir: &str) {
    std::env::set_var("TOSS_DATA_DIR", dir);
}

// No-op implementations for non-Android platforms
#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn set_android_encryption_key(_key: [u8; 32]) -> Result<(), CryptoError> {
    Ok(())
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub fn set_android_data_dir(_dir: &str) {
    // No-op on non-Android platforms
}

// Fallback in-memory storage for unsupported platforms or temporary use
#[allow(dead_code)]
struct MemoryStorage {
    data: Mutex<HashMap<String, Vec<u8>>>,
}

#[allow(dead_code)]
impl MemoryStorage {
    fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl SecureStorage for MemoryStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        let mut data = self.data.lock().unwrap();
        data.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        let data = self.data.lock().unwrap();
        Ok(data.get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        let mut data = self.data.lock().unwrap();
        data.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new();

        // Test store and retrieve
        storage.store("test_key", b"test_value").unwrap();
        let value = storage.retrieve("test_key").unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));

        // Test delete
        storage.delete("test_key").unwrap();
        let value = storage.retrieve("test_key").unwrap();
        assert_eq!(value, None);
    }
}
