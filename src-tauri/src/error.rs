//! 统一应用错误类型 `AppError`。
//!
//! 面向**激活的** Tauri command（chat_cmd / file_ops / session_cmd / keychain_cmd），
//! 用来替代原先散落的 `Result<T, String>`，把错误分门别类，同时保持前端兼容。
//!
//! ## 前端兼容（关键约束）
//!
//! 前端（如 `CanvasView.tsx`）用 `typeof e === "string" ? e : "请求失败"` 展示错误。
//! Tauri 会把 command 的 `Err(E)` 通过 `E: Serialize` 序列化后传给前端。
//! 若 `AppError` 用 `#[derive(Serialize)]`，会被序列化成 **对象**（`{ "InvalidInput": "..." }`），
//! 前端 `typeof e === "string"` 判定为 false，丢失具体错误信息、只显示"请求失败"。
//!
//! 因此这里**手动实现 `Serialize`，直接输出 `self.to_string()` 字符串**，
//! 保证前端拿到的始终是可读中文字符串，与旧的 `Result<T, String>` 行为一致。

use thiserror::Error;

/// 统一应用错误类型。
///
/// 每个变体承载一段中文描述信息，`Display`（由 `thiserror` 的 `#[error(...)]` 生成）
/// 决定最终序列化给前端的字符串内容。
#[derive(Debug, Error)]
pub enum AppError {
    /// 输入参数非法（空值、格式错误、越界等）。
    #[error("输入非法: {0}")]
    InvalidInput(String),

    /// 目标资源不存在（文件、会话等）。
    #[error("{0}")]
    NotFound(String),

    /// 访问被拒绝（命中受保护路径黑名单等）。
    #[error("{0}")]
    Forbidden(String),

    /// 本地 I/O 错误（读写文件、目录操作等）。
    #[error("{0}")]
    Io(String),

    /// 网络请求错误（发起 HTTP 请求失败等）。
    #[error("{0}")]
    Network(String),

    /// 远端 API 返回错误（非 2xx 响应、上游报错等）。
    #[error("{0}")]
    Api(String),

    /// 存储层错误（数据库读写、钥匙串存取等）。
    #[error("{0}")]
    Storage(String),

    /// 解析错误（JSON 反序列化、响应格式不符等）。
    #[error("{0}")]
    Parse(String),
}

// 手动实现 Serialize：始终序列化为 `to_string()` 的字符串。
// 不要改成 `#[derive(Serialize)]`——那会序列化成对象，破坏前端字符串判断。
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
