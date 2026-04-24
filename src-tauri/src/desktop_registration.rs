use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context};

const APP_PATHS_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\AeroTrans.exe";

pub fn ensure_current_user_registration() -> anyhow::Result<()> {
    #[cfg(not(target_os = "windows"))]
    {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
    let exe_path = env::current_exe().context("failed to resolve current executable path")?;
    let app_data_dir =
        env::var_os("APPDATA").ok_or_else(|| anyhow!("APPDATA environment variable is missing"))?;
    let shortcut_path = start_menu_shortcut_path(Path::new(&app_data_dir));

    ensure_app_paths_registry(&exe_path)?;
    ensure_start_menu_shortcut(&shortcut_path, &exe_path)?;

    Ok(())
    }
}

pub fn build_app_paths_commands(exe_path: &Path) -> anyhow::Result<Vec<String>> {
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow!("executable path is missing a parent directory"))?;
    let exe_str = exe_path.to_string_lossy();
    let dir_str = exe_dir.to_string_lossy();

    Ok(vec![
        format!(r#"reg add "{APP_PATHS_KEY}" /ve /d "{exe_str}" /f"#),
        format!(r#"reg add "{APP_PATHS_KEY}" /v Path /d "{dir_str}" /f"#),
    ])
}

pub fn start_menu_shortcut_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("AeroTrans.lnk")
}

pub fn build_shortcut_script(shortcut_path: &Path, exe_path: &Path) -> anyhow::Result<String> {
    let working_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow!("executable path is missing a parent directory"))?;
    let shortcut_str = ps_single_quoted(shortcut_path);
    let exe_str = ps_single_quoted(exe_path);
    let working_dir_str = ps_single_quoted(working_dir);

    Ok(format!(
        "$shortcutPath = {shortcut_str}; \
         $targetPath = {exe_str}; \
         $workingDir = {working_dir_str}; \
         $shell = New-Object -ComObject WScript.Shell; \
         $shortcut = $shell.CreateShortcut($shortcutPath); \
         $shortcut.TargetPath = $targetPath; \
         $shortcut.WorkingDirectory = $workingDir; \
         $shortcut.IconLocation = \"$targetPath,0\"; \
         $shortcut.Description = 'AeroTrans desktop translator'; \
         $shortcut.Save();"
    ))
}

fn ensure_app_paths_registry(exe_path: &Path) -> anyhow::Result<()> {
    for command in build_app_paths_commands(exe_path)? {
        run_command("cmd", ["/C", command.as_str()])?;
    }

    Ok(())
}

fn ensure_start_menu_shortcut(shortcut_path: &Path, exe_path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = shortcut_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let script = build_shortcut_script(shortcut_path, exe_path)?;
    run_command(
        "powershell.exe",
        [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script.as_str(),
        ],
    )
}

fn run_command<I, S>(program: &str, args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to launch {program}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Err(anyhow!(
        "{program} exited with {}. stdout: {} stderr: {}",
        output.status,
        if stdout.is_empty() { "<empty>" } else { &stdout },
        if stderr.is_empty() { "<empty>" } else { &stderr }
    ))
}

fn ps_single_quoted(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\'', "''");
    format!("'{text}'")
}
