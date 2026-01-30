use tauri::{AppHandle, Manager};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce
};
use argon2::{
    password_hash::{
        rand_core::OsRng as ArgonOsRng,
        PasswordHasher, SaltString
    },
    Argon2
};
use rand::RngCore;

const NONCE_SIZE: usize = 24;

#[tauri::command]
pub async fn encrypt_file(app: AppHandle, file_path: String, pin: String) -> Result<String, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File not found".into());
    }

    // 1. Read file
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

    // 2. Generate Salt & Derive Key
    let salt = SaltString::generate(&mut ArgonOsRng);
    let argon2 = Argon2::default();
    
    // Hash password to get key
    let password_hash = argon2.hash_password(pin.as_bytes(), &salt)
        .map_err(|e| e.to_string())?;
    
    let hash_bytes = password_hash.hash.ok_or("Hash failed")?;
    let key_bytes = hash_bytes.as_bytes();
    
    // Ensure key is 32 bytes (Argon2 default should be 32)
    if key_bytes.len() != 32 {
        return Err("Derived key length invalid".into());
    }
    
    let key = chacha20poly1305::Key::from_slice(key_bytes);
    let cipher = XChaCha20Poly1305::new(key);

    // 3. Encrypt
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes); // 24-bytes; unique per message

    let ciphertext = cipher.encrypt(nonce, buffer.as_ref())
        .map_err(|e| e.to_string())?;

    // 4. Save to Vault
    let vault_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("vault");
    if !vault_dir.exists() {
        fs::create_dir_all(&vault_dir).map_err(|e| e.to_string())?;
    }
    
    let file_name = path.file_name().unwrap().to_string_lossy();
    let vault_path = vault_dir.join(format!("{}.void", file_name));
    
    let mut out_file = File::create(&vault_path).map_err(|e| e.to_string())?;
    
    // File Format:
    // [Salt Len (1 byte)] [Salt String bytes] [Nonce (24 bytes)] [Ciphertext]
    
    let salt_str = salt.as_str();
    let salt_bytes = salt_str.as_bytes();
    let salt_len = salt_bytes.len();
    
    if salt_len > 255 {
        return Err("Salt too long".into());
    }
    
    out_file.write_all(&[salt_len as u8]).map_err(|e| e.to_string())?;
    out_file.write_all(salt_bytes).map_err(|e| e.to_string())?;
    out_file.write_all(&nonce_bytes).map_err(|e| e.to_string())?;
    out_file.write_all(&ciphertext).map_err(|e| e.to_string())?;
    
    // 5. Secure Wipe
    secure_wipe(path)?;
    
    Ok(vault_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn decrypt_file(file_path: String, pin: String) -> Result<Vec<u8>, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("Vault file not found".into());
    }
    
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    
    if buffer.len() < 1 + NONCE_SIZE {
        return Err("Invalid file format".into());
    }
    
    // Parse Header
    let salt_len = buffer[0] as usize;
    if buffer.len() < 1 + salt_len + NONCE_SIZE {
        return Err("Invalid file format (short)".into());
    }
    
    let salt_bytes = &buffer[1..1+salt_len];
    let salt_str = std::str::from_utf8(salt_bytes).map_err(|_| "Invalid salt encoding")?;
    let salt = SaltString::from_b64(salt_str).map_err(|e| e.to_string())?;
    
    let nonce_start = 1 + salt_len;
    let nonce_bytes = &buffer[nonce_start..nonce_start+NONCE_SIZE];
    let ciphertext = &buffer[nonce_start+NONCE_SIZE..];
    
    // Derive Key
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(pin.as_bytes(), &salt)
        .map_err(|e| e.to_string())?;
        
    let hash_bytes = password_hash.hash.ok_or("Hash failed")?;
    let key_bytes = hash_bytes.as_bytes();
    
    if key_bytes.len() != 32 {
        return Err("Derived key length invalid".into());
    }
    
    let key = chacha20poly1305::Key::from_slice(key_bytes);
    let cipher = XChaCha20Poly1305::new(key);
    let nonce = XNonce::from_slice(nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed: Incorrect PIN or corrupted file")?;
        
    Ok(plaintext)
}

fn secure_wipe(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let len = metadata.len();
    let mut file = OpenOptions::new().write(true).open(path).map_err(|e| e.to_string())?;
    
    // Overwrite with zeros
    let zeros = vec![0u8; 4096];
    let mut written = 0;
    while written < len {
        let to_write = std::cmp::min(zeros.len() as u64, len - written);
        file.write_all(&zeros[..to_write as usize]).map_err(|e| e.to_string())?;
        written += to_write;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    
    // Delete
    fs::remove_file(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_vault_files(app: AppHandle) -> Result<Vec<String>, String> {
    let vault_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("vault");
    if !vault_dir.exists() {
        return Ok(vec![]);
    }
    
    let mut files = Vec::new();
    for entry in fs::read_dir(vault_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("void") {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                files.push(name.to_string());
            }
        }
    }
    Ok(files)
}
