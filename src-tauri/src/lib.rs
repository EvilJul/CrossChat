mod core;
mod ports;
mod adapters;
mod application;
mod migration;
mod commands;

use std::sync::Arc;
use tauri::Manager;

use commands::file_ops::{
    delete_file_or_dir, get_home_dir, list_directory, read_file_content,
    get_file_preview_info, read_file_bytes, write_file_bytes,
};
use commands::chat_cmd::{send_chat_message, fetch_models};
use commands::keychain_cmd::{set_api_key, get_api_key, delete_api_key};
use commands::session_cmd::{
    create_session, delete_session, get_session, list_sessions, save_messages,
    set_session_status, rename_session, set_session_pinned,
};
use migration::migrate_data;

use adapters::store::SqliteThreadStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let data_dir = dirs::data_dir()
                .ok_or("Cannot find data directory")?
                .join(".crosschat");
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data dir: {}", e))?;

            let db_path = data_dir.join("threads.db");
            let db_url = format!("sqlite:{}", db_path.display());

            let thread_store = Arc::new(
                SqliteThreadStore::new(&db_url)
                    .map_err(|e| format!("Failed to init store: {}", e))?
            );
            app.manage(thread_store.clone());

            // Migration + other init in background
            tauri::async_runtime::spawn(async move {
                if !data_dir.join(".migrated").exists() {
                    println!("Running data migration...");
                    match migration::migrate_sessions(thread_store.as_ref()).await {
                        Ok(report) => {
                            println!("Migration complete: {} of {} sessions migrated",
                                report.success, report.total);
                            if !report.errors.is_empty() {
                                eprintln!("Migration errors: {:?}", report.errors);
                            }
                            let _ = std::fs::write(data_dir.join(".migrated"), "1");
                        }
                        Err(e) => eprintln!("Migration failed: {}", e),
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_chat_message,
            fetch_models,
            list_directory,
            get_home_dir,
            read_file_content,
            get_file_preview_info,
            read_file_bytes,
            write_file_bytes,
            delete_file_or_dir,
            create_session,
            list_sessions,
            get_session,
            save_messages,
            delete_session,
            set_session_status,
            rename_session,
            set_session_pinned,
            migrate_data,
            set_api_key,
            get_api_key,
            delete_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
