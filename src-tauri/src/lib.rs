mod commands;
mod mcp;
mod providers;
mod security;
mod streaming;
mod tools;

use commands::agent_cmd::{read_agent_config, write_global_agent_config};
use commands::chat::stream_chat;
use commands::checkpoint_cmd::{clear_checkpoint, load_checkpoint, save_checkpoint};
use commands::file_ops::{get_home_dir, list_directory};
use commands::mcp_cmd::{
    add_mcp_server, list_mcp_servers, refresh_mcp_tools, remove_mcp_server, toggle_mcp_server,
};
use commands::provider_cmd::test_provider_connection;
use commands::session_cmd::{
    create_session, delete_session, get_session, list_sessions, save_messages,
};
use commands::skills_cmd::get_available_skills;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            stream_chat,
            test_provider_connection,
            list_directory,
            get_home_dir,
            create_session,
            list_sessions,
            get_session,
            save_messages,
            delete_session,
            add_mcp_server,
            remove_mcp_server,
            toggle_mcp_server,
            list_mcp_servers,
            refresh_mcp_tools,
            get_available_skills,
            read_agent_config,
            write_global_agent_config,
            save_checkpoint,
            load_checkpoint,
            clear_checkpoint,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
