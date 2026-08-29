use super::{AppInfo, Os};
use crate::protocol::{BrowserTab, NowPlaying, RunningApp};

pub struct StubOs;

impl Os for StubOs {
    fn frontmost_app(&self) -> Option<AppInfo> {
        Some(AppInfo {
            name: "Desktop".into(),
            bundle_id: "stub".into(),
        })
    }

    fn frontmost_window_title(&self) -> String {
        String::new()
    }

    fn volume_get(&self) -> Result<u8, String> {
        Ok(50)
    }

    fn volume_set(&self, _value: u8) -> Result<(), String> {
        Ok(())
    }

    fn muted_get(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn muted_set(&self, _muted: bool) -> Result<(), String> {
        Ok(())
    }

    fn media_play_pause(&self, _target: Option<&str>) -> Result<(), String> {
        Ok(())
    }

    fn media_next(&self, _target: Option<&str>) -> Result<(), String> {
        Ok(())
    }

    fn media_prev(&self, _target: Option<&str>) -> Result<(), String> {
        Ok(())
    }

    fn media_shuffle(&self) -> Result<(), String> {
        Ok(())
    }

    fn media_like(&self, _target: Option<&str>) -> Result<bool, String> {
        Ok(true)
    }

    fn media_like_key(&self) -> Result<(), String> {
        Ok(())
    }

    fn media_like_restore(&self) {}

    fn media_seek_back(&self) -> Result<(), String> {
        Ok(())
    }

    fn media_seek_forward(&self) -> Result<(), String> {
        Ok(())
    }

    fn media_repeat(&self) -> Result<(), String> {
        Ok(())
    }

    fn media_airplay(&self) -> Result<(), String> {
        Ok(())
    }

    fn now_playing(
        &self,
        _front: Option<&super::AppInfo>,
        _running: &[RunningApp],
    ) -> Option<NowPlaying> {
        None
    }

    fn browser_now_playing(&self, _front: &super::AppInfo) -> Option<NowPlaying> {
        None
    }

    fn tab_playback(&self, _bundle_id: &str, _window_index: u32, _tab_index: u32) -> Option<bool> {
        None
    }

    fn tab_media_toggle(
        &self,
        _bundle_id: &str,
        _tab: &crate::protocol::BrowserTab,
    ) -> Result<(), String> {
        Err("unsupported".into())
    }

    fn send_shortcut(&self, _action: &str) -> Result<(), String> {
        Ok(())
    }

    fn focus_app(&self, _bundle_id: &str, _window_index: Option<u32>) -> Result<(), String> {
        Ok(())
    }

    fn list_apps(&self) -> Vec<RunningApp> {
        Vec::new()
    }

    fn browser_tabs(&self, _bundle_id: &str) -> Vec<crate::protocol::BrowserTab> {
        Vec::new()
    }

    fn activate_browser_tab(
        &self,
        _bundle_id: &str,
        _tab: &crate::protocol::BrowserTab,
    ) -> Result<(), String> {
        Ok(())
    }

    fn app_icon_png(&self, _bundle_id: &str, _cache_dir: &std::path::Path) -> Option<Vec<u8>> {
        None
    }

    fn volume_state(&self) -> Option<super::VolumeState> {
        None
    }

    fn quick_state(&self) -> Option<super::QuickState> {
        None
    }

    fn input_volume_get(&self) -> Result<u8, String> {
        Ok(75)
    }

    fn input_volume_set(&self, _value: u8) -> Result<(), String> {
        Ok(())
    }

    fn clipboard_get(&self) -> Option<String> {
        None
    }

    fn clipboard_set(&self, _text: &str) -> Result<(), String> {
        Ok(())
    }

    fn clipboard_probe(&self) -> Option<super::ClipboardProbe> {
        None
    }

    fn clipboard_image_get(&self, _dest: &std::path::Path) -> Option<super::ClipboardImage> {
        None
    }

    fn clipboard_image_set(&self, _src: &std::path::Path) -> Result<(), String> {
        Err("unsupported".into())
    }

    fn detect_call(
        &self,
        _running: &[RunningApp],
        _tabs: &[BrowserTab],
        _tabs_bundle: &str,
    ) -> Option<crate::protocol::CallSession> {
        None
    }

    fn dock_badge(&self, _app_name: &str) -> Option<u32> {
        None
    }

    fn type_text(&self, _text: &str) -> Result<(), String> {
        Ok(())
    }
}
