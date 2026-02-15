use hex;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

static WS_ID: OnceCell<String> = OnceCell::new();
pub struct WsId;

impl WsId {
    /*
     * We need a unique id for every workspace and it should be defined once per execution
     * once set it can be read but not set again.
     */
    pub fn set(path: &PathBuf) {
        let path_str: String = path.to_string_lossy().to_string();
        // Create a SHA256 hasher
        let mut hasher = Sha256::new();
        // Update the hasher with the path string
        hasher.update(path_str.as_bytes());
        // Finalize the hash and convert it to a hexadecimal string
        let result = hasher.finalize();
        let hex_string: String = hex::encode(result);
        // Return the first 10 characters of the hexadecimal string
        WS_ID.set(hex_string.chars().take(10).collect());
    }

    pub fn get() -> String {
        WS_ID
            .get()
            .map(|s| s.clone())
            .unwrap_or_else(|| "0".to_string())
    }
}
