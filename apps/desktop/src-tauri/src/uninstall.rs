use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

const IDS: &[&str] = &[
    "dev.caddie.desktop",
    "dev.deskthing.desktop",
    "dev.companion.desktop",
];
const NAMES: &[&str] = &["Caddie", "caddie", "DeskThing", "deskthing", "Companion"];

pub fn bundled_app() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(|p| p.to_path_buf())
}

pub fn run(app: &tauri::AppHandle) -> Result<(), String> {
    let _ = app.autolaunch().disable();
    wipe_data(app);
    let current = bundled_app();
    trash_now(&leftover_apps(current.as_deref()));
    if let Some(bundle) = current {
        schedule_trash(&[bundle]);
    }
    app.exit(0);
    Ok(())
}

fn leftover_apps(current: Option<&Path>) -> Vec<PathBuf> {
    [
        PathBuf::from("/Applications/Caddie.app"),
        PathBuf::from("/Applications/caddie.app"),
        PathBuf::from("/Applications/DeskThing.app"),
        PathBuf::from("/Applications/deskthing.app"),
        PathBuf::from("/Applications/Companion.app"),
    ]
    .into_iter()
    .filter(|p| p.exists() && current != Some(p.as_path()))
    .collect()
}

fn wipe_data(app: &tauri::AppHandle) {
    for dir in [
        app.path().app_data_dir(),
        app.path().app_config_dir(),
        app.path().app_cache_dir(),
        app.path().app_log_dir(),
        app.path().app_local_data_dir(),
    ]
    .into_iter()
    .flatten()
    {
        remove_dir(&dir);
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    for id in IDS {
        remove_dir(&home.join("Library/Application Support").join(id));
        remove_dir(&home.join("Library/Caches").join(id));
        remove_dir(&home.join("Library/Logs").join(id));
        remove_dir(&home.join("Library/WebKit").join(id));
        remove_dir(&home.join("Library/HTTPStorages").join(id));
        remove_dir(
            &home
                .join("Library/Saved Application State")
                .join(format!("{id}.savedState")),
        );
        let _ = std::fs::remove_file(home.join("Library/Preferences").join(format!("{id}.plist")));
        let _ = std::fs::remove_file(
            home.join("Library/LaunchAgents")
                .join(format!("{id}.plist")),
        );
    }
    for name in NAMES {
        remove_dir(&home.join("Library/Application Support").join(name));
        remove_dir(&home.join("Library/Caches").join(name));
        remove_dir(&home.join("Library/Logs").join(name));
    }
}

fn remove_dir(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn trash_now(paths: &[PathBuf]) {
    for path in paths {
        let _ = finder_delete(path);
    }
}

fn finder_delete(path: &Path) -> Result<(), String> {
    let posix = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let status = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"tell application "Finder" to delete POSIX file "{posix}""#
        ))
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Finder could not move the app to Trash".into())
    }
}

#[cfg(unix)]
fn schedule_trash(paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    let pid = std::process::id();
    let mut osa = String::from("tell application \"Finder\"\n");
    for path in paths {
        let posix = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        osa.push_str(&format!(
            "  try\n    delete POSIX file \"{posix}\"\n  end try\n"
        ));
    }
    osa.push_str("end tell\n");
    let script_path = std::env::temp_dir().join("caddie-uninstall.scpt");
    let sh_path = std::env::temp_dir().join("caddie-uninstall.sh");
    if std::fs::write(&script_path, osa).is_err() {
        return;
    }
    let sh = format!(
        "while kill -0 {pid} 2>/dev/null; do sleep 0.2; done\nosascript {:?}\nrm -f {:?} {:?}\n",
        script_path, script_path, sh_path
    );
    if std::fs::write(&sh_path, sh).is_err() {
        return;
    }
    use std::os::unix::process::CommandExt;
    let _ = Command::new("/bin/sh")
        .arg(&sh_path)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(not(unix))]
fn schedule_trash(_paths: &[PathBuf]) {}
