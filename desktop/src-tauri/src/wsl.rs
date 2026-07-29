/// WSL detection and Windows-path conversion helpers.
///
/// When the desktop app runs inside WSL, the Linux build of
/// `tauri-plugin-opener` targets a Linux desktop environment that usually
/// does not exist. These helpers detect WSL and translate paths so we can
/// delegate opening files and revealing folders to the Windows host via
/// WSL interop (`cmd.exe`, `explorer.exe`).
use std::path::Path;

/// Returns `true` when the process is running inside the Windows Subsystem
/// for Linux.
pub fn is_wsl() -> bool {
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }

    if let Ok(contents) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let lower = contents.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }

    if let Ok(contents) = std::fs::read_to_string("/proc/version") {
        let lower = contents.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }

    false
}

/// Convert a Linux path (possibly from WSL) into a Windows path.
///
/// Uses `wslpath -w` when available, which is the canonical WSL tool for
/// this conversion. Falls back to manual mapping for `/mnt/<drive>/...`
/// paths if `wslpath` cannot be used.
pub fn to_windows_path(path: &str) -> Result<String, String> {
    let path = Path::new(path);

    // Prefer the canonical WSL helper if it is available.
    if let Ok(output) = std::process::Command::new("wslpath")
        .arg("-w")
        .arg(path)
        .output()
    {
        if output.status.success() {
            let windows_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !windows_path.is_empty() {
                return Ok(windows_path);
            }
        }
    }

    // Fallback: manually rewrite /mnt/<drive>/... paths.
    if let Some(absolute) = path.canonicalize().ok().or_else(|| {
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }) {
        if let Some(win_path) = try_mnt_to_windows(&absolute) {
            return Ok(win_path);
        }
    }

    Err(format!(
        "could not convert WSL path to Windows path: {}",
        path.display()
    ))
}

/// Convert `/mnt/c/some/path` to `C:\some\path`.
fn try_mnt_to_windows(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();

    // Expect: RootDir, Normal("mnt"), Normal("<drive>"), ...
    if components.len() < 4 {
        return None;
    }

    let first = components[0].as_os_str().to_str()?;
    if first != "/" {
        return None;
    }

    let mnt = components[1].as_os_str().to_str()?;
    let drive = components[2].as_os_str().to_str()?;

    if mnt != "mnt" || drive.len() != 1 {
        return None;
    }

    let tail = components[3..]
        .iter()
        .map(|c| c.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?
        .join("\\");

    Some(format!("{}:\\{}", drive.to_uppercase(), tail))
}

/// Open a file using the Windows host's default application.
pub async fn open_file_with_windows_host(path: &str) -> Result<(), String> {
    let win_path = to_windows_path(path)?;

    let status = tokio::process::Command::new("cmd.exe")
        .args(["/C", "start", "", &win_path])
        .status()
        .await
        .map_err(|e| format!("failed to launch Windows host opener: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Windows host opener exited with status: {}",
            status
        ))
    }
}

/// Reveal a file in the Windows host's file explorer.
///
/// `explorer.exe /select,<path>` commonly returns exit code 1 even when it
/// successfully opens the folder. We therefore only fail if the process could
/// not be spawned at all.
pub async fn reveal_file_with_windows_host(path: &str) -> Result<(), String> {
    let win_path = to_windows_path(path)?;

    let status = tokio::process::Command::new("explorer.exe")
        .arg(format!("/select, {}", win_path))
        .status()
        .await
        .map_err(|e| format!("failed to launch Windows host explorer: {}", e))?;

    // explorer.exe is a Windows shell process and often exits with 1
    // immediately after handing the request to the shell, regardless of
    // success. Treat any spawned invocation as a success.
    if status.code().is_some() {
        Ok(())
    } else {
        Err("Windows host explorer was terminated unexpectedly".to_string())
    }
}
