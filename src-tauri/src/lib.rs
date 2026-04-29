mod agent;
mod commands;
mod mcp;
mod memory;
mod metrics;
mod providers;
mod security;
mod skills;
mod streaming;
mod tools;

use commands::agent_cmd::{read_agent_config, write_global_agent_config};
use commands::chat::stream_chat;
use commands::stream_cmd::{start_stream_chat, poll_stream_chunks};
use commands::checkpoint_cmd::{clear_checkpoint, load_checkpoint, save_checkpoint};
use commands::file_ops::{delete_file_or_dir, get_home_dir, list_directory, read_file_content};
use commands::mcp_cmd::{
    add_mcp_server, list_mcp_servers, refresh_mcp_tools, remove_mcp_server, toggle_mcp_server,
};
use commands::mcp_health_cmd::{check_mcp_health, get_all_mcp_health};
use commands::memory_cmd::{cleanup_memories, get_recent_memories, search_memories};
use commands::metrics_cmd::{cleanup_metrics, get_tool_stats};
use commands::provider_cmd::test_provider_connection;
use commands::session_cmd::{
    create_session, delete_session, get_session, list_sessions, save_messages,
};
use commands::skills_cmd::{get_available_skills, list_skills, remove_skill, toggle_skill};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            stream_chat,
            start_stream_chat,
            poll_stream_chunks,
            test_provider_connection,
            list_directory,
            get_home_dir,
            read_file_content,
            delete_file_or_dir,
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
            check_mcp_health,
            get_all_mcp_health,
            get_available_skills,
            list_skills,
            toggle_skill,
            remove_skill,
            read_agent_config,
            write_global_agent_config,
            save_checkpoint,
            load_checkpoint,
            clear_checkpoint,
            get_recent_memories,
            search_memories,
            cleanup_memories,
            get_tool_stats,
            cleanup_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
