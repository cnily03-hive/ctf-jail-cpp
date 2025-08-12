use rune::{modules::any, ContextError, Module};
use std::{fs, io, path::PathBuf, sync::OnceLock};

// Use OnceLock to store global context_path
static CONTEXT_PATH: OnceLock<String> = OnceLock::new();

/// Set global context path
pub fn set_context_path(path: String) {
    let _ = CONTEXT_PATH.set(path);
}

/// Get global context path
fn get_context_path() -> Result<&'static str, io::Error> {
    CONTEXT_PATH
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Context path not set"))
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

/// Read file from context directory
#[rune::function]
pub fn read_ctx_file(file_path: &str) -> Result<String, io::Error> {
    let context_path = get_context_path()?;
    let path = PathBuf::from(context_path).join(file_path);

    // Security check: ensure file is within context directory
    let canonical_context = fs::canonicalize(context_path)?;
    let canonical_file = fs::canonicalize(&path).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("File does not exist: {}", file_path),
        )
    })?;
    println!("{}", path.to_string_lossy());

    if !canonical_file.starts_with(&canonical_context) {
        println!("No");
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "Access to files outside context directory is not allowed: {}",
                file_path
            ),
        ));
    }

    fs::read_to_string(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to read file {}: {}", file_path, e),
        )
    })
}

/// Copy file from context directory to sandbox
#[rune::function]
pub fn copy_ctx_file(filename: &str, dest_path: &str) -> Result<String, io::Error> {
    let context_path = get_context_path()?;
    let source = PathBuf::from(context_path).join(filename);
    let dest = PathBuf::from(dest_path).join(filename);

    // Security check: ensure source file is within context directory
    let canonical_context = fs::canonicalize(context_path)?;
    let canonical_source = fs::canonicalize(&source).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("File does not exist: {}", filename),
        )
    })?;

    if !canonical_source.starts_with(&canonical_context) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Access to files outside context directory is not allowed",
        ));
    }

    // Ensure destination directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&source, &dest)?;
    Ok(format!("File {} has been copied to sandbox", filename))
}
