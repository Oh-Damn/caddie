use crate::devlog::{self, Snapshot};
use crate::dto::SessionDto;
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct PermissionStatus {
    #[serde(rename = "accessibilityTrusted")]
    pub accessibility_trusted: bool,
}

#[tauri::command]
pub fn session(state: State<AppState>) -> SessionDto {
    state.session()
}

#[tauri::command]
pub fn complete_onboarding(state: State<AppState>) -> Result<(), String> {
    state.complete_onboarding().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn permission_status() -> PermissionStatus {
    PermissionStatus {
        accessibility_trusted: crate::os::permissions::accessibility_trusted(),
    }
}

#[tauri::command]
pub fn request_accessibility() -> Result<(), String> {
    crate::os::permissions::request_accessibility()
}

#[tauri::command]
pub fn request_automation() -> Result<(), String> {
    crate::os::permissions::request_automation()
}

#[tauri::command]
pub fn cancel_uninstall(state: State<AppState>) {
    state.set_uninstall_prompt(false);
}

#[tauri::command]
pub fn uninstall(app: AppHandle) -> Result<(), String> {
    crate::uninstall::run(&app)
}

#[tauri::command]
pub fn is_debug() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
pub fn dev_logs() -> Snapshot {
    devlog::snapshot()
}

#[tauri::command]
pub fn dev_logs_clear() {
    devlog::clear();
}

#[tauri::command]
pub fn dev_client_error(message: String) {
    devlog::record_client(message);
}

#[derive(Serialize)]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,

    pub reads: Vec<String>,
}

#[tauri::command]
pub fn providers(state: State<AppState>) -> Vec<ProviderDto> {
    let enabled = state.enabled_providers();
    crate::providers::ALL
        .iter()
        .map(|p| ProviderDto {
            id: p.id.to_string(),
            name: p.name.to_string(),
            enabled: enabled.has(p.id),
            reads: p.sources.iter().map(|s| s.label().to_string()).collect(),
        })
        .collect()
}

#[tauri::command]
pub fn set_provider_enabled(state: State<AppState>, id: String, enabled: bool) {
    state.set_provider_enabled(&id, enabled);
}
