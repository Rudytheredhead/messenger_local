//! Moduł Antigravity: lekki natywny czat z lokalnym agentem CLI `agy.exe` (bez WebView2).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use serde::Serialize;

use crate::modules::IslandModule;

pub const LABEL: &str = "antigravity";

#[derive(Serialize, Clone, Debug)]
pub struct AntigravityReply {
    pub reply: String,
    pub conversation_id: Option<String>,
}

pub struct Antigravity;

impl Antigravity {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Antigravity {
    fn id(&self) -> &'static str {
        LABEL
    }
    fn name(&self) -> &'static str {
        "Antigravity"
    }
    fn icon(&self) -> &'static str {
        "✨"
    }
    fn priority(&self) -> u8 {
        98
    }
}

/// Stan lokalnej sesji rozmowy z `agy.exe`.
#[derive(Default)]
pub struct AntigravitySession {
    lock: Mutex<()>,
}

impl AntigravitySession {
    pub fn reset(&self) {}

    pub fn ask(
        &self,
        prompt: &str,
        model: Option<&str>,
        effort: Option<&str>,
        conversation_id: Option<&str>,
        paths: &[String],
    ) -> Result<AntigravityReply, String> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err("Wpisz wiadomość.".to_string());
        }

        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let exe = resolve_agy_binary();

        let mut cmd = Command::new(&exe);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Some(home) = std::env::var_os("USERPROFILE") {
            cmd.current_dir(home);
        }

        let incoming_cid = conversation_id
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(ToString::to_string);

        if let Some(cid) = incoming_cid.as_deref() {
            cmd.args(["--conversation", cid]);
        }

        if let Some(m) = model.map(str::trim).filter(|m| !m.is_empty()) {
            cmd.args(["--model", m]);
        }

        if let Some(e) = effort.map(str::trim).filter(|e| !e.is_empty()) {
            cmd.args(["--effort", e]);
        }

        let clean_paths: Vec<String> = paths
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(ToString::to_string)
            .collect();

        for dir in collect_workspace_dirs(&clean_paths) {
            cmd.args(["--add-dir", &dir]);
        }

        let full_prompt = if clean_paths.is_empty() {
            trimmed.to_string()
        } else {
            let list = clean_paths
                .iter()
                .map(|p| format!("- {p}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("Dołączone ścieżki plików/folderów:\n{list}\n\n{trimmed}")
        };

        cmd.args(["-p", &full_prompt, "--output-format", "json"]);

        let output = cmd.output().map_err(|e| {
            format!(
                "Nie udało się uruchomić Antigravity CLI ({}): {e}",
                exe.display()
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            let msg = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("Proces agy zakończył się kodem {}", output.status)
            };
            return Err(msg);
        }

        let parsed = serde_json::from_str::<serde_json::Value>(&stdout).ok();
        let parsed_cid = parsed
            .as_ref()
            .and_then(|v| v.get("conversation_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .or(incoming_cid);

        let reply_text = parsed
            .as_ref()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if stdout.is_empty() {
                    "(Brak odpowiedzi tekstowej)".to_string()
                } else {
                    stdout
                }
            });

        Ok(AntigravityReply {
            reply: reply_text,
            conversation_id: parsed_cid,
        })
    }
}

fn collect_workspace_dirs(paths: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();
    for raw in paths {
        let p = Path::new(raw);
        let dir = if p.is_dir() {
            p
        } else if let Some(parent) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
            parent
        } else {
            p
        };
        let dir_str = dir.to_string_lossy().into_owned();
        if seen.insert(dir_str.clone()) {
            dirs.push(dir_str);
        }
    }
    dirs
}

fn find_agy_binary() -> Option<PathBuf> {
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let candidate = PathBuf::from(profile).join(".gemini").join("bin").join("agy.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        #[cfg(windows)]
        let names = ["agy.exe", "agy.cmd", "agy.bat"];
        #[cfg(not(windows))]
        let names = ["agy"];
        for dir in std::env::split_paths(&path_var) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Sprawdza bez uruchamiania procesu, czy lokalne CLI `agy` jest dostępne w systemie.
pub fn is_available() -> bool {
    find_agy_binary().is_some()
}

fn resolve_agy_binary() -> PathBuf {
    find_agy_binary().unwrap_or_else(|| PathBuf::from("agy"))
}

/// Otwiera systemowe okno wyboru wielu plików (Windows STA PowerShell).
pub fn pick_files_dialog() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.OpenFileDialog
$dlg.Multiselect = $true
$dlg.Title = 'Wybierz pliki dla Antigravity'
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    $dlg.FileNames | ForEach-Object { [Console]::Out.WriteLine($_) }
}
"#;
        let out = run_sta_powershell(script)?;
        Ok(out
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(ToString::to_string)
            .collect())
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

/// Otwiera systemowe okno wyboru folderu (Windows STA PowerShell).
pub fn pick_folder_dialog() -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
$dlg = New-Object System.Windows.Forms.FolderBrowserDialog
$dlg.Description = 'Wybierz folder projektu dla Antigravity'
$dlg.UseDescriptionForTitle = $true
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.WriteLine($dlg.SelectedPath)
}
"#;
        let out = run_sta_powershell(script)?;
        let selected = out.lines().map(str::trim).find(|l| !l.is_empty());
        Ok(selected.map(ToString::to_string))
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}

#[cfg(windows)]
fn run_sta_powershell(script: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Nie udało się otworzyć okna wyboru: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Błąd okna wyboru plików.".to_string()
        } else {
            err
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
