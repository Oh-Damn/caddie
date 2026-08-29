use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use crate::protocol::{Conversation, DeviceDetails};

const PIN_CAP: usize = 12;
const RECENTS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub device_id: String,
    pub name: String,
    pub created_at: String,
    #[serde(default)]
    pub details: DeviceDetails,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub last_ip: String,
    #[serde(default)]
    pub last_mac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stored {
    pub server_id: String,
    pub trusted: Vec<TrustedDevice>,
    #[serde(default)]
    pub onboarding_complete: bool,
    #[serde(default)]
    pub conversation_recents: HashMap<String, Vec<Conversation>>,

    #[serde(default)]
    pub recents_version: u32,
    #[serde(default)]
    pub conversation_pins: HashMap<String, Vec<Conversation>>,

    #[serde(default)]
    pub agent_session_ceiling: Option<f64>,
    #[serde(default)]
    pub agent_week_ceiling: Option<f64>,

    #[serde(default)]
    pub agent_week_anchor: Option<u32>,

    #[serde(default)]
    pub providers_off: BTreeSet<String>,
}

impl Stored {
    pub fn load_or_init(path: &PathBuf) -> Result<Self, std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            let mut stored: Self = serde_json::from_str(&raw).unwrap_or_else(|_| Self::fresh());
            if stored.recents_version != RECENTS_VERSION {
                stored.conversation_recents.clear();
                stored.recents_version = RECENTS_VERSION;
                stored.save(path)?;
            }
            return Ok(stored);
        }
        let stored = Self::fresh();
        stored.save(path)?;
        Ok(stored)
    }

    fn fresh() -> Self {
        Self {
            server_id: uuid::Uuid::new_v4().to_string(),
            trusted: Vec::new(),
            onboarding_complete: false,
            conversation_recents: HashMap::new(),
            recents_version: RECENTS_VERSION,
            conversation_pins: HashMap::new(),
            agent_session_ceiling: None,
            agent_week_ceiling: None,
            agent_week_anchor: None,
            providers_off: BTreeSet::new(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.trusted.iter().any(|d| d.device_id == device_id)
    }

    pub fn complete_onboarding(&mut self) {
        self.onboarding_complete = true;
    }

    pub fn trust(&mut self, device_id: String, name: String, details: DeviceDetails) {
        if self.trusted.len() == 1 && self.trusted[0].device_id == device_id {
            return;
        }
        self.trusted.clear();
        self.trusted.push(TrustedDevice {
            device_id,
            name,
            created_at: chrono::Utc::now().to_rfc3339(),
            details,
            last_seen: String::new(),
            last_ip: String::new(),
            last_mac: String::new(),
        });
    }

    pub fn mark_seen(
        &mut self,
        device_id: &str,
        name: &str,
        details: &DeviceDetails,
        ip: &str,
        mac: &str,
    ) -> bool {
        let Some(device) = self.trusted.iter_mut().find(|d| d.device_id == device_id) else {
            return false;
        };
        let mut changed = false;
        if !name.is_empty() && device.name != name {
            device.name = name.to_string();
            changed = true;
        }
        if *details != DeviceDetails::default() && device.details != *details {
            device.details = details.clone();
            changed = true;
        }
        if !ip.is_empty() && device.last_ip != ip {
            device.last_ip = ip.to_string();
            changed = true;
        }
        if !mac.is_empty() && device.last_mac != mac {
            device.last_mac = mac.to_string();
            changed = true;
        }
        device.last_seen = chrono::Utc::now().to_rfc3339();
        changed
    }

    pub fn device(&self) -> Option<&TrustedDevice> {
        self.trusted.first()
    }

    pub fn pin_conversation(&mut self, bundle_id: &str, conv: Conversation) {
        if bundle_id.is_empty() {
            return;
        }
        let list = self
            .conversation_pins
            .entry(bundle_id.to_string())
            .or_default();
        if let Some(idx) = list
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(&conv.name) && c.kind == conv.kind)
        {
            list.remove(idx);
        }
        let mut stored = conv;
        stored.window_index = None;
        list.insert(0, stored);
        list.truncate(PIN_CAP);
    }

    pub fn unpin_conversation(&mut self, bundle_id: &str, name: &str) {
        let Some(list) = self.conversation_pins.get_mut(bundle_id) else {
            return;
        };
        list.retain(|c| !c.name.eq_ignore_ascii_case(name));
        if list.is_empty() {
            self.conversation_pins.remove(bundle_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stored;

    #[test]
    fn trust_keeps_one_device() {
        let mut stored = Stored::fresh();
        stored.trust("a".into(), "iPhone".into(), Default::default());
        stored.trust("b".into(), "iPad".into(), Default::default());
        assert_eq!(stored.trusted.len(), 1);
        assert!(stored.is_trusted("b"));
        assert!(!stored.is_trusted("a"));
    }

    #[test]
    fn trust_is_idempotent_for_same_device() {
        let mut stored = Stored::fresh();
        stored.trust("a".into(), "iPhone".into(), Default::default());
        let first = stored.trusted[0].created_at.clone();
        stored.trust("a".into(), "iPhone".into(), Default::default());
        assert_eq!(stored.trusted.len(), 1);
        assert_eq!(stored.trusted[0].created_at, first);
    }

    #[test]
    fn mark_seen_updates_location_and_keeps_pairing_date() {
        let mut stored = Stored::fresh();
        stored.trust("a".into(), "iPhone".into(), Default::default());
        let paired = stored.trusted[0].created_at.clone();
        assert!(stored.mark_seen(
            "a",
            "Abhi's iPhone",
            &Default::default(),
            "192.168.1.42",
            "9e:d6:e3:cc:d3:c5"
        ));
        let device = stored.device().unwrap();
        assert_eq!(device.name, "Abhi's iPhone");
        assert_eq!(device.last_ip, "192.168.1.42");
        assert_eq!(device.created_at, paired);
        assert!(!device.last_seen.is_empty());
        assert!(!stored.mark_seen("missing", "x", &Default::default(), "", ""));
    }
}
