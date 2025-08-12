use rune::{Any, ContextError, Module};
use std::path::{Component, Path};
use std::{fs, io, path::PathBuf};

/// Context module for jailbox, providing file operations
#[rune::module(::jailapi::context)]
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?;
    module.ty::<Context>()?;
    module.ty::<DataBucket>()?;
    module.function_meta(Context::bucket)?;
    module.function_meta(DataBucket::read)?;
    module.function_meta(DataBucket::list)?;
    Ok(module)
}

#[derive(Clone, Debug, Any)]
#[rune(item = ::jailapi::context)]
pub struct Context {
    bucket: DataBucket,
}

#[derive(Clone, Debug, Any)]
#[rune(item = ::jailapi::context)]
pub struct DataBucket {
    path: String,
}

impl Context {
    pub fn new(bucket_path: String) -> Self {
        Context {
            bucket: DataBucket::new(bucket_path),
        }
    }

    #[rune::function]
    pub fn bucket(&self) -> DataBucket {
        self.bucket.clone()
    }
}

impl DataBucket {
    pub fn new(path: String) -> Self {
        DataBucket { path }
    }

    #[rune::function]
    pub fn read(&self, file_path: &str) -> Result<String, io::Error> {
        let safe_file_path = normalize_path(file_path);
        let abs_path = to_abs_pathbuf(&safe_file_path, Some(&self.path));

        // Security check: ensure file is within data bucket directory
        if !security_path_within(&safe_file_path, &self.path) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Access to this path is not allowed: {}", file_path),
            ));
        }

        if !file_exists(&safe_file_path, &self.path) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", file_path),
            ));
        }

        fs::read_to_string(&abs_path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to read file {}: {}", file_path, e),
            )
        })
    }

    #[rune::function]
    pub fn list(&self, dpath: &str) -> Result<Vec<String>, io::Error> {
        let safe_path = normalize_path(dpath);
        let abs_path = to_abs_pathbuf(&safe_path, Some(&self.path));

        // Security check: ensure path is within data bucket directory
        if !security_path_within(&safe_path, &self.path) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Access to this path is not allowed: {}", dpath),
            ));
        }

        if !abs_path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Directory not found: {}", dpath),
            ));
        }

        Ok(fs::read_dir(&abs_path)
            .map_err(|e| io::Error::new(e.kind(), format!("Failed to read directory: {}", e)))?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().into_string().unwrap_or_default())
            .collect::<Vec<_>>())
    }
}

fn file_exists<P: AsRef<Path>, Q: AsRef<Path>>(test_path: P, context_path: Q) -> bool {
    let fullpath = context_path.as_ref().join(test_path.as_ref());
    fullpath.exists()
}

/// dummy check
fn security_path_within<P: AsRef<Path>, Q: AsRef<Path>>(test_path: P, super_path: Q) -> bool {
    let super_abs = to_abs_pathbuf::<_, &Path>(super_path.as_ref(), None);
    let cur_abs = to_abs_pathbuf(test_path, Some(super_path.as_ref()));
    println!(
        "Security check: super_abs: {}, cur_abs: {}",
        super_abs.to_string_lossy(),
        cur_abs.to_string_lossy()
    );
    cur_abs.starts_with(&super_abs)
}

/// dummy absolute
fn to_abs_pathbuf<P: AsRef<Path>, Q: AsRef<Path>>(
    target_path: P,
    context_path: Option<Q>,
) -> PathBuf {
    let context_path = match context_path {
        Some(context_path) => {
            let context_path = context_path.as_ref();

            if context_path.is_absolute() {
                context_path.to_path_buf()
            } else {
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                current_dir.join(context_path)
            }
        }
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let target_path = target_path.as_ref();

    if target_path.is_absolute() {
        return normalize_path(target_path);
    }

    normalize_path(context_path.join(target_path))
}

/// dummy normalize path (`/path/to/foo/../bar` to `/path/to/bar`)
fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path_buf = path.as_ref();
    let mut components = Vec::new();

    for component in path_buf.components() {
        match component {
            Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            Component::CurDir => {}
            _ => {
                components.push(component);
            }
        }
    }

    components.iter().collect()
}
