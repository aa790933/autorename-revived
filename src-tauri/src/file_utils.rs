use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_file_to_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read file {}: {}", path, e))
}

pub fn read_file_to_base64(path: &str) -> Result<String, String> {
    let bytes = read_file_to_bytes(path)?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

pub fn preserve_extension(original_path: &str, new_name: &str) -> String {
    let ext = Path::new(original_path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    if ext.is_empty() {
        new_name.to_string()
    } else {
        let stem = Path::new(new_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| new_name.to_string());
        format!("{}.{}", stem, ext)
    }
}

pub fn validate_supported_extension(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "pdf" | "docx" | "xlsx" | "pptx" | "csv" | "txt" | "md" | "rtf" | "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "gif" | "webp"
    )
}

pub fn get_supported_extensions() -> Vec<&'static str> {
    vec![
        ".pdf", ".docx", ".xlsx", ".pptx", ".csv", ".txt", ".md", ".rtf",
        ".png", ".jpg", ".jpeg", ".tiff", ".tif", ".bmp", ".gif", ".webp",
    ]
}

pub fn is_image_extension(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "gif" | "webp"
    )
}

pub fn ensure_directory(path: &str) -> Result<(), String> {
    let parent = Path::new(path).parent();
    if let Some(dir) = parent {
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn file_size(path: &str) -> Result<u64, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(metadata.len())
}

pub fn list_files_in_directory(dir: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn recursive_find_files(dir: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files)?;
    Ok(files)
}

fn collect_files_recursive(dir: &str, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(
                path.to_string_lossy().as_ref(),
                files,
            )?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub fn copy_file(src: &str, dst: &str) -> Result<(), String> {
    fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn get_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

pub fn get_file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

pub fn get_file_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default()
}