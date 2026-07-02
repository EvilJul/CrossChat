//! 钥匙串命令：使用系统钥匙串（macOS Keychain / Windows Credential Manager / Linux Secret Service）
//! 安全存取各 LLM provider 的 API Key。
//!
//! 统一的 keyring service 名称为 `com.tian.crosschat`，account/user 使用传入的 `provider_id`。
//! 这是把 API Key 从前端明文 localStorage 迁移到系统钥匙串的后端部分。

use keyring::{Entry, Error as KeyringError};

use crate::error::AppError;

/// keyring 统一的 service 名称。
const KEYCHAIN_SERVICE: &str = "com.tian.crosschat";

/// 写入（或覆盖）指定 provider 的 API Key。
#[tauri::command]
pub fn set_api_key(provider_id: String, key: String) -> Result<(), AppError> {
    // 以固定 service + provider_id 作为条目定位。
    let entry = Entry::new(KEYCHAIN_SERVICE, &provider_id)
        .map_err(|e| AppError::Storage(e.to_string()))?;
    entry
        .set_password(&key)
        .map_err(|e| AppError::Storage(e.to_string()))
}

/// 读取指定 provider 的 API Key。无记录时返回 `Ok(None)`，其他错误返回 `Err`。
#[tauri::command]
pub fn get_api_key(provider_id: String) -> Result<Option<String>, AppError> {
    let entry = Entry::new(KEYCHAIN_SERVICE, &provider_id)
        .map_err(|e| AppError::Storage(e.to_string()))?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        // 无条目视为“没有存过”，返回 None 而非错误。
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Storage(e.to_string())),
    }
}

/// 删除指定 provider 的 API Key。无记录也视为成功（幂等删除）。
#[tauri::command]
pub fn delete_api_key(provider_id: String) -> Result<(), AppError> {
    let entry = Entry::new(KEYCHAIN_SERVICE, &provider_id)
        .map_err(|e| AppError::Storage(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        // 删除不存在的条目视为成功。
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Storage(e.to_string())),
    }
}
