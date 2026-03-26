//! Clipboard text copy operations with multi-platform support
//!
//! Provides robust text-to-clipboard functionality supporting:
//! - Native clipboard (macOS, Windows, Linux via arboard)
//! - SSH sessions (OSC 52 escape sequences)
//! - WSL environments (PowerShell fallback)

use base64::Engine;
use std::io::Write;

/// Copy text to clipboard with automatic environment detection
///
/// Strategy:
/// 1. If SSH session detected → OSC 52 escape sequence
/// 2. Try native clipboard (arboard)
/// 3. On Linux with WSL detected → PowerShell fallback
#[cfg(not(target_os = "android"))]
pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    // 1. Detect SSH - use OSC 52
    if std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some() {
        return copy_via_osc52(text);
    }

    // 2. Try native clipboard (arboard)
    let error = match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text.to_string()) {
            Ok(()) => return Ok(()),
            Err(err) => format!("clipboard unavailable: {err}"),
        },
        Err(err) => format!("clipboard unavailable: {err}"),
    };

    // 3. Fallback WSL (Linux apenas)
    #[cfg(target_os = "linux")]
    let error = if is_probably_wsl() {
        match copy_via_wsl_clipboard(text) {
            Ok(()) => return Ok(()),
            Err(wsl_err) => format!("{error}; WSL fallback failed: {wsl_err}"),
        }
    } else {
        error
    };

    Err(error)
}

/// Copy text via OSC 52 escape sequence (for SSH sessions)
#[cfg(not(target_os = "android"))]
fn copy_via_osc52(text: &str) -> Result<(), String> {
    let sequence = osc52_sequence(text, std::env::var_os("TMUX").is_some());

    // Unix: escrever diretamente no /dev/tty
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;

        let mut tty = OpenOptions::new()
            .write(true)
            .open("/dev/tty")
            .map_err(|e| format!("failed to open /dev/tty: {e}"))?;
        tty.write_all(sequence.as_bytes())
            .map_err(|e| format!("failed to write OSC 52: {e}"))?;
        tty.flush()
            .map_err(|e| format!("failed to flush OSC 52: {e}"))?;
    }

    // Windows: usar stdout
    #[cfg(windows)]
    {
        use std::io::stdout;
        stdout()
            .write_all(sequence.as_bytes())
            .map_err(|e| format!("failed to write OSC 52: {e}"))?;
        stdout()
            .flush()
            .map_err(|e| format!("failed to flush OSC 52: {e}"))?;
    }

    Ok(())
}

/// Generate OSC 52 escape sequence
fn osc52_sequence(text: &str, tmux: bool) -> String {
    let payload = base64::engine::general_purpose::STANDARD.encode(text);
    if tmux {
        // Tmux passthrough
        format!("\x1bPtmux;\x1b\x1b]52;c;{payload}\x07\x1b\\")
    } else {
        // Standard OSC 52
        format!("\x1b]52;c;{payload}\x07")
    }
}

/// Copy via PowerShell (WSL fallback)
#[cfg(all(not(target_os = "android"), target_os = "linux"))]
fn copy_via_wsl_clipboard(text: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("powershell.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .args([
            "-NoProfile",
            "-Command",
            "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; \
             $ErrorActionPreference = 'Stop'; \
             $text = [Console]::In.ReadToEnd(); \
             Set-Clipboard -Value $text",
        ])
        .spawn()
        .map_err(|e| format!("failed to spawn powershell.exe: {e}"))?;

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        return Err("failed to open powershell.exe stdin".to_string());
    };

    stdin
        .write_all(text.as_bytes())
        .map_err(|e| format!("failed to write to powershell.exe: {e}"))?;

    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for powershell.exe: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("powershell.exe exited with status {}", output.status)
        } else {
            format!("powershell.exe failed: {stderr}")
        })
    }
}

/// Detect if running under WSL (Windows Subsystem for Linux)
#[cfg(target_os = "linux")]
pub(crate) fn is_probably_wsl() -> bool {
    // Verificar /proc/version para "microsoft" ou "WSL"
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let version_lower = version.to_lowercase();
        if version_lower.contains("microsoft") || version_lower.contains("wsl") {
            return true;
        }
    }

    // Verificar variáveis de ambiente WSL
    std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some()
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    #[test]
    fn osc52_sequence_encodes_text_for_terminal_clipboard() {
        assert_eq!(osc52_sequence("hello", false), "\u{1b}]52;c;aGVsbG8=\u{7}");
    }

    #[test]
    fn osc52_sequence_wraps_tmux_passthrough() {
        assert_eq!(
            osc52_sequence("hello", true),
            "\u{1b}Ptmux;\u{1b}\u{1b}]52;c;aGVsbG8=\u{7}\u{1b}\\"
        );
    }
}
