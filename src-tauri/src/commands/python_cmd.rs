use crate::python_env;

/// 获取Python环境信息
#[tauri::command]
pub fn get_python_info() -> Result<serde_json::Value, String> {
    let is_available = python_env::is_python_available();
    let version = if is_available {
        python_env::get_python_version().ok()
    } else {
        None
    };

    let env_path = python_env::get_python_env_path().to_string_lossy().to_string();
    let executable_path = python_env::get_python_executable().to_string_lossy().to_string();

    Ok(serde_json::json!({
        "available": is_available,
        "version": version,
        "env_path": env_path,
        "executable_path": executable_path,
    }))
}

/// 运行Python脚本
#[tauri::command]
pub fn run_python_script(script: String, args: Vec<String>) -> Result<String, String> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    python_env::run_python_script(&script, &args_refs)
}
