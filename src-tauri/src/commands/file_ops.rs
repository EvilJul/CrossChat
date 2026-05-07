use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::python_env;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilePreviewInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_executable: bool,
    pub file_type: String,
    pub preview_content: Option<String>,
}

/// 列出目录下的文件和子目录
#[tauri::command]
pub async fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    if !dir.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    if !dir.is_dir() {
        return Err(format!("不是目录: {}", path));
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&dir).map_err(|e| format!("无法读取目录: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("获取元数据失败: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();

        // 跳过隐藏文件
        if name.starts_with('.') {
            continue;
        }

        entries.push(FileEntry {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    // 目录在前，文件在后，按名称排序
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));

    Ok(entries)
}

/// 读取文件内容（用于预览面板）
#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }
    // 限制预览文件大小为 10MB
    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    if metadata.len() > 10_485_760 {
        return Err("文件过大（超过 10MB），无法预览".to_string());
    }
    std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

/// 获取文件预览信息（支持可执行文件）
#[tauri::command]
pub async fn get_file_preview_info(path: String) -> Result<FilePreviewInfo, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }

    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    let name = file_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // 检查是否为可执行文件
    let is_executable = is_executable_file(&file_path);
    let file_type = get_file_type(&file_path);

    // 如果文件过大（超过10MB），不读取内容
    if metadata.len() > 10_485_760 {
        return Ok(FilePreviewInfo {
            name,
            path: path.clone(),
            size: metadata.len(),
            is_executable,
            file_type,
            preview_content: None,
        });
    }

    // 根据文件类型决定预览内容
    let preview_content = if is_executable {
        Some(format!("可执行文件: {}\n大小: {} 字节\n类型: {}", name, metadata.len(), file_type))
    } else if is_text_file(&file_path) {
        std::fs::read_to_string(&file_path).ok()
    } else if is_office_file(&file_path) {
        read_office_file_content(&file_path)
    } else {
        Some(format!("二进制文件: {}\n大小: {} 字节\n类型: {}", name, metadata.len(), file_type))
    };

    Ok(FilePreviewInfo {
        name,
        path: path.clone(),
        size: metadata.len(),
        is_executable,
        file_type,
        preview_content,
    })
}

/// 检查文件是否为可执行文件
fn is_executable_file(path: &PathBuf) -> bool {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Windows可执行文件扩展名
    let executable_extensions = ["exe", "msi", "bat", "cmd", "com", "scr", "ps1", "vbs", "js"];

    executable_extensions.contains(&extension.as_str())
}

/// 获取文件类型描述
fn get_file_type(path: &PathBuf) -> String {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "exe" => "Windows 可执行文件".to_string(),
        "msi" => "Windows 安装程序".to_string(),
        "bat" => "Windows 批处理文件".to_string(),
        "cmd" => "Windows 命令脚本".to_string(),
        "com" => "DOS 可执行文件".to_string(),
        "scr" => "Windows 屏幕保护程序".to_string(),
        "ps1" => "PowerShell 脚本".to_string(),
        "vbs" => "VBScript 脚本".to_string(),
        "js" => "JavaScript 文件".to_string(),
        "py" => "Python 脚本".to_string(),
        "rs" => "Rust 源代码".to_string(),
        "ts" => "TypeScript 源代码".to_string(),
        "tsx" => "TypeScript React 源代码".to_string(),
        "jsx" => "JavaScript React 源代码".to_string(),
        "html" | "htm" => "HTML 文件".to_string(),
        "css" => "CSS 样式表".to_string(),
        "json" => "JSON 数据文件".to_string(),
        "xml" => "XML 数据文件".to_string(),
        "md" => "Markdown 文档".to_string(),
        "txt" => "文本文件".to_string(),
        "log" => "日志文件".to_string(),
        "csv" => "CSV 数据文件".to_string(),
        "sql" => "SQL 脚本".to_string(),
        "sh" => "Shell 脚本".to_string(),
        "bash" => "Bash 脚本".to_string(),
        "zip" | "rar" | "7z" | "tar" | "gz" => "压缩文件".to_string(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "图像文件".to_string(),
        "mp3" | "wav" | "flac" | "aac" => "音频文件".to_string(),
        "mp4" | "avi" | "mkv" | "mov" => "视频文件".to_string(),
        "pdf" => "PDF 文档".to_string(),
        "doc" | "docx" => "Word 文档".to_string(),
        "xls" | "xlsx" => "Excel 表格".to_string(),
        "ppt" | "pptx" => "PowerPoint 演示文稿".to_string(),
        _ => "未知类型".to_string(),
    }
}

/// 检查文件是否为文本文件
fn is_text_file(path: &PathBuf) -> bool {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let text_extensions = [
        "txt", "log", "md", "markdown", "json", "xml", "html", "htm", "css", "js", "ts",
        "tsx", "jsx", "py", "rs", "go", "java", "c", "cpp", "h", "hpp", "cs", "php",
        "rb", "swift", "kt", "scala", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
        "sql", "csv", "tsv", "yaml", "yml", "toml", "ini", "cfg", "conf", "env",
        "gitignore", "dockerignore", "editorconfig", "prettierrc", "eslintrc",
    ];

    text_extensions.contains(&extension.as_str())
}

/// 检查文件是否为Office文件
fn is_office_file(path: &PathBuf) -> bool {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let office_extensions = ["doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf"];
    office_extensions.contains(&extension.as_str())
}

/// 读取Office文件内容
fn read_office_file_content(path: &PathBuf) -> Option<String> {
    let extension = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let path_str = path.to_string_lossy().to_string();

    match extension.as_str() {
        "xlsx" | "xls" => read_excel_file(&path_str),
        "docx" | "doc" => read_word_file(&path_str),
        "pptx" | "ppt" => read_powerpoint_file(&path_str),
        "pdf" => read_pdf_file(&path_str),
        _ => Some(format!("不支持的Office文件类型: {}", extension)),
    }
}

/// 读取Excel文件
fn read_excel_file(path: &str) -> Option<String> {
    let python_script = r#"
import sys
import json

try:
    import openpyxl
    path = sys.argv[1]
    wb = openpyxl.load_workbook(path, read_only=True, data_only=True)
    result = []
    for sheet_name in wb.sheetnames:
        ws = wb[sheet_name]
        sheet_data = []
        for row in ws.iter_rows(max_row=50, values_only=True):
            row_data = [str(cell) if cell is not None else "" for cell in row]
            sheet_data.append("\t".join(row_data))
        result.append(f"=== 工作表: {sheet_name} ===\n" + "\n".join(sheet_data))
    wb.close()
    print("\n\n".join(result))
except ImportError:
    print("错误: 需要安装openpyxl库。请运行: pip install openpyxl")
except Exception as e:
    print(f"读取Excel文件失败: {e}")
"#;

    match python_env::run_python_script(python_script, &[path]) {
        Ok(content) => Some(content),
        Err(e) => Some(format!("读取Excel文件失败: {}", e)),
    }
}

/// 读取Word文件
fn read_word_file(path: &str) -> Option<String> {
    let python_script = r#"
import sys

try:
    from docx import Document
    path = sys.argv[1]
    doc = Document(path)
    result = []
    for para in doc.paragraphs:
        if para.text.strip():
            result.append(para.text)
    print("\n".join(result))
except ImportError:
    print("错误: 需要安装python-docx库。请运行: pip install python-docx")
except Exception as e:
    print(f"读取Word文件失败: {e}")
"#;

    match python_env::run_python_script(python_script, &[path]) {
        Ok(content) => Some(content),
        Err(e) => Some(format!("读取Word文件失败: {}", e)),
    }
}

/// 读取PowerPoint文件
fn read_powerpoint_file(path: &str) -> Option<String> {
    let python_script = r#"
import sys

try:
    from pptx import Presentation
    path = sys.argv[1]
    prs = Presentation(path)
    result = []
    for i, slide in enumerate(prs.slides, 1):
        slide_text = [f"--- 幻灯片 {i} ---"]
        for shape in slide.shapes:
            if hasattr(shape, "text") and shape.text.strip():
                slide_text.append(shape.text)
        result.append("\n".join(slide_text))
    print("\n\n".join(result))
except ImportError:
    print("错误: 需要安装python-pptx库。请运行: pip install python-pptx")
except Exception as e:
    print(f"读取PowerPoint文件失败: {e}")
"#;

    match python_env::run_python_script(python_script, &[path]) {
        Ok(content) => Some(content),
        Err(e) => Some(format!("读取PowerPoint文件失败: {}", e)),
    }
}

/// 读取PDF文件
fn read_pdf_file(path: &str) -> Option<String> {
    let python_script = r#"
import sys

try:
    import PyPDF2
    path = sys.argv[1]
    with open(path, 'rb') as f:
        reader = PyPDF2.PdfReader(f)
        result = []
        for i, page in enumerate(reader.pages[:50], 1):
            text = page.extract_text()
            if text.strip():
                result.append(f"--- 第 {i} 页 ---\n{text}")
        print("\n\n".join(result))
except ImportError:
    print("错误: 需要安装PyPDF2库。请运行: pip install PyPDF2")
except Exception as e:
    print(f"读取PDF文件失败: {e}")
"#;

    match python_env::run_python_script(python_script, &[path]) {
        Ok(content) => Some(content),
        Err(e) => Some(format!("读取PDF文件失败: {}", e)),
    }
}

/// 删除文件或空目录（用于工作区右键菜单）
#[tauri::command]
pub async fn delete_file_or_dir(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.exists() { return Err("文件不存在".into()); }
    if p.is_dir() {
        std::fs::remove_dir_all(&p).map_err(|e| format!("删除目录失败: {}", e))?;
    } else {
        std::fs::remove_file(&p).map_err(|e| format!("删除文件失败: {}", e))?;
    }
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(format!("已删除: {}", name))
}

/// 获取当前用户的主目录
#[tauri::command]
pub fn get_home_dir() -> String {
    dirs_next_home().unwrap_or_else(|| "/".into())
}

fn dirs_next_home() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}
