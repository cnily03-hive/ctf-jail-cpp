use rune::{ContextError, Module};
use std::{fs, io, path::PathBuf, sync::OnceLock};

// 使用 OnceLock 来存储全局的 context_path
static CONTEXT_PATH: OnceLock<String> = OnceLock::new();

/// 设置全局的context路径
pub fn set_context_path(path: String) {
    let _ = CONTEXT_PATH.set(path);
}

/// 获取全局的context路径
fn get_context_path() -> Result<&'static str, io::Error> {
    CONTEXT_PATH
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Context路径未设置"))
        .map(|s| s.as_str())
}

/// Context module for jailbox, providing file operations
#[rune::module(::jailapi::context)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.function_meta(read_ctx_file)?;
    module.function_meta(copy_ctx_file)?;
    Ok(module)
}

/// 读取上下文目录中的文件
#[rune::function]
pub fn read_ctx_file(file_path: &str) -> Result<String, io::Error> {
    let context_path = get_context_path()?;
    let path = PathBuf::from(context_path).join(file_path);

    // 安全检查：确保文件在context目录内
    let canonical_context = fs::canonicalize(context_path)?;
    let canonical_file = fs::canonicalize(&path).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("文件不存在: {}", file_path),
        )
    })?;

    if !canonical_file.starts_with(&canonical_context) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "不允许访问上下文目录外的文件",
        ));
    }

    fs::read_to_string(&path)
        .map_err(|e| io::Error::new(e.kind(), format!("读取文件失败 {}: {}", file_path, e)))
}

/// 复制上下文目录中的文件到沙箱
#[rune::function]
pub fn copy_ctx_file(filename: &str, dest_path: &str) -> Result<String, io::Error> {
    let context_path = get_context_path()?;
    let source = PathBuf::from(context_path).join(filename);
    let dest = PathBuf::from(dest_path).join(filename);

    // 安全检查：确保源文件在context目录内
    let canonical_context = fs::canonicalize(context_path)?;
    let canonical_source = fs::canonicalize(&source).map_err(|_| {
        io::Error::new(io::ErrorKind::NotFound, format!("文件不存在: {}", filename))
    })?;

    if !canonical_source.starts_with(&canonical_context) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "不允许访问上下文目录外的文件",
        ));
    }

    // 确保目标目录存在
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&source, &dest)?;
    Ok(format!("文件 {} 已复制到沙箱", filename))
}
