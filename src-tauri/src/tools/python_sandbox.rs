/// Python 沙盒环境管理
/// 在 ~/.crosschat/python/ 下创建隔离的 Python 环境，配置阿里镜像源，预装常用库

use std::path::PathBuf;
use std::process::Command;

/// 沙盒根目录
fn sandbox_dir() -> PathBuf {
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(&home).join(".crosschat").join("python")
}

/// 沙盒中的 Python 可执行文件
fn sandbox_python() -> PathBuf {
    if cfg!(target_os = "windows") {
        sandbox_dir().join("venv").join("Scripts").join("python.exe")
    } else {
        sandbox_dir().join("venv").join("bin").join("python")
    }
}

/// 系统 Python 路径（python3 或 python）
fn system_python() -> &'static str {
    if cfg!(target_os = "windows") { "python" } else { "python3" }
}

/// 确保沙盒环境已初始化
pub fn ensure_sandbox() -> Result<PathBuf, String> {
    let py_path = sandbox_python();

    // 已初始化则直接返回
    if py_path.exists() {
        return Ok(py_path);
    }

    // 创建目录
    let dir = sandbox_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;

    // 1. 创建虚拟环境
    let output = Command::new(system_python())
        .args(["-m", "venv", "--clear"])
        .arg(dir.join("venv"))
        .output()
        .map_err(|e| format!("创建 venv 失败: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("创建 Python 虚拟环境失败: {}", err));
    }

    // 2. 配置 pip 使用阿里镜像源
    let pip_conf = dir.join("pip.ini");
    let pip_conf_content = r#"[global]
index-url = https://mirrors.aliyun.com/pypi/simple/
trusted-host = mirrors.aliyun.com
"#;
    std::fs::write(&pip_conf, pip_conf_content).map_err(|e| format!("写入 pip 配置失败: {}", e))?;

    // 3. 升级 pip 并安装常用库
    let install_script = format!(
        r#"
import subprocess, sys
pip = [r"{}", "-m", "pip", "install", "--upgrade", "pip"]
subprocess.run(pip, check=True)
packages = [
    "pandas", "numpy", "openpyxl", "xlsxwriter", "xlrd",
    "requests", "httpx", "beautifulsoup4", "lxml",
    "matplotlib", "pillow", "pypdf", "python-docx",
    "python-pptx", "rich", "tqdm",
]
for pkg in packages:
    subprocess.run([r"{}", "-m", "pip", "install", pkg, "--no-warn-script-location"], check=False)
print("SANDBOX_READY")
"#,
        sandbox_python().display(),
        sandbox_python().display(),
    );

    let output = Command::new(system_python())
        .args(["-c", &install_script])
        .output()
        .map_err(|e| format!("安装依赖失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("SANDBOX_READY") {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("沙盒初始化失败:\n{}\n{}", stdout, stderr));
    }

    Ok(py_path)
}

/// 将命令中的 python/pip 替换为沙盒版本
pub fn sandboxify_command(command: &str) -> String {
    let py = sandbox_python();
    let py_str = py.display().to_string();
    let pip_str = py.parent().map(|p| p.join("pip.exe").display().to_string())
        .unwrap_or_else(|| format!("{} -m pip", py_str));

    // 精确替换：python.exe、python、python3 → sandbox python
    // pip、pip3 → sandbox pip
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.is_empty() { return command.to_string(); }

    let first = words[0].to_lowercase();
    let rest = words[1..].join(" ");

    // 检查是否是 python/pip 命令
    let is_python = first == "python" || first == "python3" || first.ends_with("python.exe") || first.ends_with("python3.exe");
    let is_pip = first == "pip" || first == "pip3" || first.ends_with("pip.exe") || first.ends_with("pip3.exe");

    if is_python {
        format!("{} {}", py_str, rest)
    } else if is_pip {
        format!("{} {}", pip_str, rest)
    } else {
        command.to_string()
    }
}

/// 检查输出是否包含 ModuleNotFoundError / ImportError，返回缺失的顶级模块名
pub fn detect_missing_module(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        // ModuleNotFoundError: No module named 'xxx'
        // ImportError: No module named 'xxx'
        if (line.contains("ModuleNotFoundError") || line.contains("ImportError"))
            && line.contains("No module named")
        {
            // 提取引号内的模块名
            let module = line
                .split("No module named")
                .nth(1)
                .and_then(|s| {
                    s.trim()
                        .trim_matches('\'')
                        .trim_matches('"')
                        .split('.')
                        .next() // 只取顶级包名（如 pandas 而非 pandas.core.frame）
                        .map(|s| s.to_string())
                });
            if let Some(mod_name) = module {
                if !mod_name.is_empty() {
                    return Some(mod_name);
                }
            }
        }
    }
    None
}

/// 自动安装缺失的模块
pub fn auto_install_module(module: &str) -> Result<String, String> {
    let py = sandbox_python();
    let output = Command::new(py.display().to_string())
        .args(["-m", "pip", "install", module])
        .output()
        .map_err(|e| format!("安装 {} 失败: {}", module, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(format!("模块 {} 安装成功\n{}", module, stdout))
    } else {
        Err(format!("模块 {} 安装失败: {}", module, stderr))
    }
}
