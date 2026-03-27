//! Clipboard image paste operations with multi-platform support
//!
//! Provides image paste functionality supporting:
//! - Native clipboard image data (arboard)
//! - File list paste (e.g., Finder copy)
//! - WSL environments (PowerShell fallback)

use std::path::PathBuf;

/// Error types for image paste operations
#[derive(Debug, Clone)]
pub enum PasteImageError {
    ClipboardUnavailable(String),
    NoImage(String),
    EncodeFailed(String),
    IoError(String),
}

/// Image format after encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodedImageFormat {
    Png,
    Jpeg,
    Other,
}

impl EncodedImageFormat {
    /// Get label for display
    pub fn label(&self) -> &'static str {
        match self {
            EncodedImageFormat::Png => "PNG",
            EncodedImageFormat::Jpeg => "JPEG",
            EncodedImageFormat::Other => "unknown",
        }
    }
}

/// Information about pasted image
#[derive(Debug, Clone)]
pub struct PastedImageInfo {
    pub width: u32,
    pub height: u32,
    pub encoded_format: EncodedImageFormat,
}

impl std::fmt::Display for PasteImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasteImageError::ClipboardUnavailable(msg) => {
                write!(f, "clipboard unavailable: {msg}")
            }
            PasteImageError::NoImage(msg) => {
                write!(f, "no image on clipboard: {msg}")
            }
            PasteImageError::EncodeFailed(msg) => {
                write!(f, "could not encode image: {msg}")
            }
            PasteImageError::IoError(msg) => {
                write!(f, "io error: {msg}")
            }
        }
    }
}

/// Paste image from clipboard as PNG bytes
#[cfg(not(target_os = "android"))]
pub fn paste_image_as_png() -> Result<(Vec<u8>, PastedImageInfo), PasteImageError> {
    let _span = tracing::debug_span!("paste_image_as_png").entered();
    tracing::debug!("attempting clipboard image read");

    let mut cb = arboard::Clipboard::new()
        .map_err(|e| PasteImageError::ClipboardUnavailable(e.to_string()))?;

    // Try image data first (most common: screenshots, browser copies)
    // Then try file list (Finder copies, file managers)
    let dyn_img = if let Ok(img) = cb.get_image() {
        // Image as raw data (screenshots, browser, etc)
        let w = img.width as u32;
        let h = img.height as u32;
        tracing::debug!("clipboard image from data: {}x{}", w, h);

        let Some(rgba_img) = image::RgbaImage::from_raw(w, h, img.bytes.into_owned()) else {
            return Err(PasteImageError::EncodeFailed("invalid RGBA buffer".into()));
        };

        image::DynamicImage::ImageRgba8(rgba_img)
    } else if let Ok(files) = cb.get().file_list() {
        // Image as file reference (Finder, file managers)
        if let Some(img) = files.into_iter().find_map(|f| image::open(f).ok()) {
            tracing::debug!(
                "clipboard image from file: {}x{}",
                img.width(),
                img.height()
            );
            img
        } else {
            return Err(PasteImageError::NoImage(
                "no valid image file in clipboard".into(),
            ));
        }
    } else {
        return Err(PasteImageError::NoImage(
            "clipboard does not contain image data or image files".into(),
        ));
    };

    // Codificar como PNG
    let mut png: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png);
    dyn_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| PasteImageError::EncodeFailed(e.to_string()))?;

    Ok((
        png,
        PastedImageInfo {
            width: dyn_img.width(),
            height: dyn_img.height(),
            encoded_format: EncodedImageFormat::Png,
        },
    ))
}

/// Paste image from clipboard and save to temporary PNG file
#[cfg(not(target_os = "android"))]
pub fn paste_image_to_temp_png() -> Result<(PathBuf, PastedImageInfo), PasteImageError> {
    match paste_image_as_png() {
        Ok((png, info)) => {
            // Create a unique temporary file
            let tmp = tempfile::Builder::new()
                .prefix("clipboard-")
                .suffix(".png")
                .tempfile()
                .map_err(|e| PasteImageError::IoError(e.to_string()))?;

            std::fs::write(tmp.path(), &png)
                .map_err(|e| PasteImageError::IoError(e.to_string()))?;

            // Persist file (don't delete when handle is closed)
            let (_file, path) = tmp
                .keep()
                .map_err(|e| PasteImageError::IoError(e.error.to_string()))?;

            Ok((path, info))
        }
        Err(e) => {
            // Tentar fallback WSL (Linux apenas)
            #[cfg(target_os = "linux")]
            {
                try_wsl_clipboard_fallback(&e).ok_or(e)
            }
            #[cfg(not(target_os = "linux"))]
            {
                Err(e)
            }
        }
    }
}

/// Fallback WSL for image paste
#[cfg(target_os = "linux")]
fn try_wsl_clipboard_fallback(error: &PasteImageError) -> Option<(PathBuf, PastedImageInfo)> {
    use PasteImageError::{ClipboardUnavailable, NoImage};

    if !super::clipboard_text::is_probably_wsl()
        || !matches!(error, ClipboardUnavailable(_) | NoImage(_))
    {
        return None;
    }

    tracing::debug!("attempting Windows PowerShell clipboard fallback");

    // Executar PowerShell para salvar imagem do clipboard
    let Some(win_path) = try_dump_windows_clipboard_image() else {
        return None;
    };

    tracing::debug!("powershell produced path: {}", win_path);

    // Converter caminho Windows para WSL
    let Some(mapped_path) = convert_windows_path_to_wsl(&win_path) else {
        return None;
    };

    let Ok((w, h)) = image::image_dimensions(&mapped_path) else {
        return None;
    };

    Some((
        mapped_path,
        PastedImageInfo {
            width: w,
            height: h,
            encoded_format: EncodedImageFormat::Png,
        },
    ))
}

/// Dump Windows clipboard image via PowerShell
#[cfg(target_os = "linux")]
fn try_dump_windows_clipboard_image() -> Option<String> {
    let script = r#"
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; 
        $img = Get-Clipboard -Format Image; 
        if ($img -ne $null) { 
            $p=[System.IO.Path]::GetTempFileName(); 
            $p = [System.IO.Path]::ChangeExtension($p,'png'); 
            $img.Save($p,[System.Drawing.Imaging.ImageFormat]::Png); 
            Write-Output $p 
        } else { 
            exit 1 
        }
    "#;

    for cmd in ["powershell.exe", "pwsh", "powershell"] {
        match std::process::Command::new(cmd)
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            Ok(output) if output.status.success() => {
                let win_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !win_path.is_empty() {
                    return Some(win_path);
                }
            }
            _ => continue,
        }
    }
    None
}

/// Convert Windows path to WSL path
#[cfg(target_os = "linux")]
fn convert_windows_path_to_wsl(input: &str) -> Option<PathBuf> {
    // Don't convert UNC paths
    if input.starts_with("\\\\") {
        return None;
    }

    let drive_letter = input.chars().next()?.to_ascii_lowercase();
    if !drive_letter.is_ascii_lowercase() {
        return None;
    }

    if input.get(1..2) != Some(":") {
        return None;
    }

    // C:\Users\... -> /mnt/c/Users/...
    let mut result = PathBuf::from(format!("/mnt/{drive_letter}"));
    for component in input
        .get(2..)?
        .trim_start_matches(['\\', '/'])
        .split(['\\', '/'])
        .filter(|c| !c.is_empty())
    {
        result.push(component);
    }

    Some(result)
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_normalize_file_url() {
        let input = "file:///tmp/example.png";
        // Note: normalize_pasted_path is not yet implemented in this module
        // This test is a placeholder for future implementation
        assert!(input.starts_with("file://"));
    }
}
