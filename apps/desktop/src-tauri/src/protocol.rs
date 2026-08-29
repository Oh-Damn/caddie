use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_PORT: u16 = 7842;
pub const PRODUCT_NAME: &str = "Caddie";
pub const SERVICE_NAME: &str = "caddie";
pub const LAN_HOST: &str = "caddie.local";

#[derive(Debug, Deserialize)]
pub struct EnvelopeIn {
    pub v: u32,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct EnvelopeOut<T: Serialize> {
    pub v: u32,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub payload: T,
}

impl<T: Serialize> EnvelopeOut<T> {
    pub fn new(id: String, kind: &'static str, payload: T) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            kind,
            payload,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
pub struct DeviceDetails {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default, rename = "platformVersion")]
    pub platform_version: String,
}

#[derive(Debug, Deserialize)]
pub struct HelloPayload {
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(default, rename = "deviceId")]
    pub device_id: Option<String>,
    #[serde(default)]
    pub details: Option<DeviceDetails>,
}

#[derive(Debug, Deserialize)]
pub struct PairPayload {
    pub secret: String,
    #[serde(default, rename = "deviceName")]
    pub device_name: String,
    #[serde(default)]
    pub details: Option<DeviceDetails>,
}

#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub action: String,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PresencePayload {
    #[serde(default)]
    pub visible: bool,
}

#[derive(Debug, Deserialize)]
pub struct QueryPayload {
    pub kind: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AppWindow {
    pub title: String,
    pub index: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct RunningApp {
    pub name: String,
    #[serde(rename = "bundleId")]
    pub bundle_id: String,
    pub windows: Vec<AppWindow>,
}

#[derive(Debug, Serialize)]
pub struct AppsPayload {
    pub items: Vec<RunningApp>,
}

#[derive(Debug, Serialize)]
pub struct HelloOkPayload {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub trusted: bool,
    #[serde(rename = "deviceId")]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PairOkPayload {
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

#[derive(Debug, Serialize)]
pub struct PairDenyPayload {
    pub reason: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct BrowserTab {
    pub title: String,
    #[serde(rename = "windowIndex")]
    pub window_index: u32,
    #[serde(rename = "tabIndex")]
    pub tab_index: u32,
    pub active: bool,
    pub audible: bool,

    #[serde(default)]
    pub media: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile: String,
    #[serde(skip)]
    pub url: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    #[serde(rename = "artworkUrl")]
    pub artwork_url: String,
    pub playing: bool,
    #[serde(rename = "sourceName")]
    pub source_name: String,
    #[serde(rename = "sourceBundleId")]
    pub source_bundle_id: String,
    #[serde(rename = "positionSec")]
    pub position_sec: f64,
    #[serde(rename = "durationSec")]
    pub duration_sec: f64,
    pub shuffle: bool,
    pub liked: bool,
    #[serde(rename = "repeatMode")]
    pub repeat_mode: RepeatMode,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct CallSession {
    pub active: bool,
    pub app: String,
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "bundleId")]
    pub bundle_id: String,
    pub title: String,
    pub muted: bool,
    #[serde(rename = "hasCamera")]
    pub has_camera: bool,
    #[serde(rename = "hasDeafen")]
    pub has_deafen: bool,
    #[serde(rename = "hasLeave")]
    pub has_leave: bool,
    #[serde(rename = "tabWindowIndex")]
    pub tab_window_index: Option<u32>,
    #[serde(rename = "tabIndex")]
    pub tab_index: Option<u32>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ApprovalSession {
    pub app: String,
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "bundleId")]
    pub bundle_id: String,
    pub title: String,
    #[serde(rename = "canAllow")]
    pub can_allow: bool,
    #[serde(rename = "canDeny")]
    pub can_deny: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConversationKind {
    Channel,
    Dm,
    Thread,
    Huddle,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub name: String,
    pub kind: ConversationKind,
    pub workspace: String,
    #[serde(rename = "windowIndex", default)]
    pub window_index: Option<u32>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AgentWindow {
    pub label: String,
    pub used: u64,

    pub limit: Option<u64>,
    pub percent: u8,
    #[serde(rename = "resetsAt")]
    pub resets_at: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AgentSession {
    pub id: String,
    pub project: String,
    pub branch: String,
    pub model: String,
    pub tokens: u64,
    pub weighted: u64,
    #[serde(rename = "subagentTokens")]
    pub subagent_tokens: u64,
    #[serde(rename = "lastActive")]
    pub last_active: String,
    pub active: bool,
    pub waiting: bool,

    pub detail: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AgentProvider {
    pub id: String,
    pub name: String,
    pub available: bool,

    pub estimated: bool,
    pub active: u32,
    pub waiting: u32,

    pub blocked: Option<String>,
    pub session: Option<AgentWindow>,
    pub week: Option<AgentWindow>,
    pub sessions: Vec<AgentSession>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AgentsSummary {
    pub percent: u8,
    pub waiting: u32,
    pub active: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentsPayload {
    pub summary: AgentsSummary,
    pub providers: Vec<AgentProvider>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatePayload {
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "bundleId")]
    pub bundle_id: String,
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "windowTitle")]
    pub window_title: String,
    pub volume: u8,
    pub muted: bool,
    #[serde(rename = "nowPlaying")]
    pub now_playing: Option<NowPlaying>,
    #[serde(rename = "browserNowPlaying", default)]
    pub browser_now_playing: Option<NowPlaying>,
    #[serde(rename = "browserMediaOwned", default)]
    pub browser_media_owned: bool,
    #[serde(default)]
    pub tabs: Vec<BrowserTab>,
    #[serde(default)]
    pub call: Option<CallSession>,
    #[serde(default)]
    pub approval: Option<ApprovalSession>,
    #[serde(default)]
    pub unread: Option<u32>,
    #[serde(rename = "currentConversation", default)]
    pub current_conversation: Option<Conversation>,
    #[serde(rename = "openConversations", default)]
    pub open_conversations: Vec<Conversation>,
    #[serde(rename = "recentConversations", default)]
    pub recent_conversations: Vec<Conversation>,
    #[serde(rename = "pinnedConversations", default)]
    pub pinned_conversations: Vec<Conversation>,
    #[serde(default)]
    pub agents: Option<AgentsSummary>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ClipboardItem {
    pub id: String,
    pub kind: String,
    pub preview: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClipboardPayload {
    pub items: Vec<ClipboardItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LayoutPayload {
    pub screen: String,
    pub title: String,
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum Widget {
    #[serde(rename = "button")]
    Button {
        id: String,
        title: String,
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(rename = "valueKey", skip_serializing_if = "Option::is_none")]
        value_key: Option<String>,
    },
    #[serde(rename = "slider")]
    Slider {
        id: String,
        title: String,
        action: String,
        min: f64,
        max: f64,
        #[serde(rename = "valueKey")]
        value_key: String,
    },
    #[serde(rename = "now_playing")]
    NowPlaying { id: String },
    #[serde(rename = "row")]
    Row { id: String, children: Vec<Widget> },
    #[serde(rename = "stack")]
    Stack { id: String, children: Vec<Widget> },
}

#[derive(Debug, Serialize)]
pub struct AckPayload {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HealthPayload {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct InfoPayload {
    pub name: String,
    pub proto: u32,
}
