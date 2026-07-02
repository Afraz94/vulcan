use keyring::{Entry};

fn handle_keyring_error(err: &keyring::Error) -> String {
    
    match err {
        keyring::Error::PlatformFailure(inner) => {
            format!("System API error (e.g., Windows CryptoAPI or macOS Security framework failed): {inner}")
        }
        keyring::Error::NoStorageAccess(inner) => {
            format!("Access denied. The OS denied Vulcan permission to touch the storage layer: {inner}")
        }
        keyring::Error::NoEntry => {
            "No credential matching that username was found in the vault.".to_string()
        }
        keyring::Error::BadEncoding(bytes) => {
            format!("The stored token contains invalid text encoding. Raw byte count: {}", bytes.len())
        }
        keyring::Error::BadDataFormat(bytes, inner) => {
            format!("The item format in the storage vault is corrupted or modified: {inner} (Bytes: {})", bytes.len())
        }
        keyring::Error::BadStoreFormat(msg) => {
            format!("The underlying OS credential store is in an invalid state: {msg}")
        }
        keyring::Error::TooLong(field, max_len) => {
            format!("The {field} parameter is too long for this OS vault. Max capacity is {max_len} bytes.")
        }
        keyring::Error::Invalid(field, msg) => {
            format!("Invalid input provided for '{field}': {msg}")
        }
        keyring::Error::Ambiguous(entries) => {
            format!("Multiple matching credentials found! Found {} matching records.", entries.len())
        }
        keyring::Error::NoDefaultStore => {
            "No default secure storage system could be located. (Common on Linux headless servers without D-Bus/Gnome Keyring active).".to_string()
        }
        keyring::Error::NotSupportedByStore(feature) => {
            format!("The native OS vault does not support this operation: {feature}")
        }
        _ => "An unknown or newly introduced keyring error occurred.".to_string(),
    }
}

pub fn save_token(token: &str, username: &str) -> bool {

    let entry = match Entry::new("Vulcan", username) {
        Ok(e) => e,
        Err(err) => {
            let error_msg = handle_keyring_error(&err);
            eprintln!("❌ [VULCAN] Initialization Failed: {error_msg}");
            return false;
        }
    };
    
    match entry.set_password(token) {
        Ok(_) => {
            println!("🔒 [VULCAN] Token successfully locked in your PC's secure credential vault.");
            return true;
        },
        Err(err) => {
            let error_msg = handle_keyring_error(&err);
            eprintln!("❌ [VULCAN] OS Secure Store rejected the token write: {error_msg}");
            return false;
        }
    }
}