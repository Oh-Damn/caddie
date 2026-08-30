use serde::Serialize;

#[derive(Serialize)]
pub struct DeviceDto {
    pub name: String,
    pub model: String,
    pub platform: String,
    #[serde(rename = "platformVersion")]
    pub platform_version: String,
    pub ip: String,
    pub mac: String,
    #[serde(rename = "pairedAt")]
    pub paired_at: String,
    #[serde(rename = "lastSeen")]
    pub last_seen: String,
    pub connected: bool,
}

#[derive(Serialize)]
pub struct SessionDto {
    #[serde(rename = "httpUrl")]
    pub http_url: String,
    #[serde(rename = "fallbackHttpUrl")]
    pub fallback_http_url: String,
    #[serde(rename = "wsUrl")]
    pub ws_url: String,
    #[serde(rename = "pairingSecret")]
    pub pairing_secret: String,
    pub fingerprint: String,
    pub port: u16,
    #[serde(rename = "clientCount")]
    pub client_count: usize,

    pub live: bool,
    pub device: Option<DeviceDto>,
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "onboardingComplete")]
    pub onboarding_complete: bool,
    #[serde(rename = "accessibilityTrusted")]
    pub accessibility_trusted: bool,
    #[serde(rename = "uninstallPrompt")]
    pub uninstall_prompt: bool,
    pub bundled: bool,
}
