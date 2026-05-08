use std::path::PathBuf;
use std::sync::OnceLock;

static PYTHON_ENV_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 获取Python沙盒环境路径
pub fn get_python_env_path() -> &'static PathBuf {
    PYTHON_ENV_PATH.get_or_init(|| {
        // 首先检查开发环境中的Python路径（使用绝对路径）
        if let Ok(current_dir) = std::env::current_dir() {
            let dev_path = current_dir.join("src-tauri/resources/python");
            if dev_path.exists() {
                return dev_path;
            }
        }

        // 检查打包后的资源路径
        let resource_path = get_resource_path().join("python");
        if resource_path.exists() {
            return resource_path;
        }

        // 默认返回当前目录下的python目录
        PathBuf::from("python")
    })
}

/// 获取资源目录路径
fn get_resource_path() -> PathBuf {
    // 在开发环境中，资源目录在src-tauri/resources
    let dev_path = PathBuf::from("src-tauri/resources");
    if dev_path.exists() {
        return dev_path;
    }

    // 在打包后的应用中，资源目录在可执行文件同级的resources目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let resource_path = exe_dir.join("resources");
            if resource_path.exists() {
                return resource_path;
            }
        }
    }

    // 默认返回当前目录
    PathBuf::from(".")
}

/// 获取Python可执行文件路径
pub fn get_python_executable() -> PathBuf {
    let python_dir = get_python_env_path();

    #[cfg(target_os = "windows")]
    {
        python_dir.join("python.exe")
    }

    #[cfg(not(target_os = "windows"))]
    {
        python_dir.join("bin").join("python3")
    }
}

/// 检查Python环境是否可用
pub fn is_python_available() -> bool {
    let python_exe = get_python_executable();
    python_exe.exists()
}

/// 获取Python版本信息
pub fn get_python_version() -> Result<String, String> {
    let python_exe = get_python_executable();
    if !python_exe.exists() {
        return Err("Python环境不存在".to_string());
    }

    let output = std::process::Command::new(&python_exe)
        .arg("--version")
        .output()
        .map_err(|e| format!("执行Python失败: {}", e))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("获取Python版本失败: {}", error))
    }
}

/// 运行Python脚本
pub fn run_python_script(script: &str, args: &[&str]) -> Result<String, String> {
    let python_exe = get_python_executable();
    if !python_exe.exists() {
        return Err("Python环境不存在".to_string());
    }

    let python_dir = get_python_env_path();

    let mut cmd = std::process::Command::new(&python_exe);
    cmd.arg("-c").arg(script);

    for arg in args {
        cmd.arg(arg);
    }

    // 设置Python使用UTF-8编码，避免中文乱码
    cmd.env("PYTHONIOENCODING", "utf-8");

    // 不设置PYTHONPATH，让Python使用python311._pth配置
    // Python脚本会通过sys.executable动态获取路径

    // 设置PATH环境变量，确保能找到Python相关的DLL
    let current_path = std::env::var("PATH").unwrap_or_default();
    let python_path = format!("{};{}", python_dir.display(), current_path);
    cmd.env("PATH", &python_path);

    let output = cmd.output()
        .map_err(|e| format!("执行Python脚本失败: {}", e))?;

    // 输出stderr用于调试
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Python stderr: {}", stderr);
    }

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(result)
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Python脚本执行失败: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_python_env_path() {
        let path = get_python_env_path();
        println!("Python环境路径: {:?}", path);
    }

    #[test]
    fn test_is_python_available() {
        let available = is_python_available();
        println!("Python是否可用: {}", available);
    }
}
