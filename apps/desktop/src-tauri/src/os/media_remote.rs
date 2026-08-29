#![cfg(target_os = "macos")]

use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const JS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct SystemNowPlaying {
    pub bundle_id: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position_sec: f64,
    pub duration_sec: f64,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default, rename = "bundleId")]
    bundle_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    playing: bool,
    #[serde(default, rename = "positionSec")]
    position_sec: f64,
    #[serde(default, rename = "durationSec")]
    duration_sec: f64,
}

pub fn system_now_playing() -> Option<SystemNowPlaying> {
    let out = osascript_js(MEDIA_REMOTE_JS).ok()?;
    if out.is_empty() {
        return None;
    }
    let raw: Raw = serde_json::from_str(&out).ok()?;
    if raw.bundle_id.is_empty() && raw.title.is_empty() {
        return None;
    }
    Some(SystemNowPlaying {
        bundle_id: raw.bundle_id,
        title: raw.title,
        artist: raw.artist,
        playing: raw.playing,
        position_sec: raw.position_sec,
        duration_sec: raw.duration_sec,
    })
}

fn osascript_js(source: &str) -> Result<String, String> {
    let mut child = Command::new("osascript")
        .args(["-l", "JavaScript"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let deadline = Instant::now() + JS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

const MEDIA_REMOTE_JS: &str = r#"
function run() {
  ObjC.import('Foundation');
  var bundle = $.NSBundle.bundleWithPath('/System/Library/PrivateFrameworks/MediaRemote.framework');
  bundle.load;
  var Req = $.NSClassFromString('MRNowPlayingRequest');
  if (!Req) return '';

  function str(v) {
    if (v === undefined || v === null) return '';
    try {
      var u = ObjC.unwrap(v);
      if (u === undefined || u === null) return '';
      return String(u);
    } catch (e) {
      return String(v);
    }
  }
  function num(v) {
    var n = parseFloat(str(v));
    return isFinite(n) ? n : 0;
  }

  var bundleId = '';
  var title = '';
  var artist = '';
  var playing = false;
  var positionSec = 0;
  var durationSec = 0;

  try {
    var client = Req.localNowPlayingClient;
    if (client) {
      bundleId = str(client.bundleIdentifier);
      if (!bundleId) bundleId = str(client.parentApplicationBundleIdentifier);
    }
  } catch (e) {}

  try {
    var path = Req.localNowPlayingPlayerPath;
    if (path) {
      if (!bundleId) bundleId = str(path.bundleID);
      try {
        if (!bundleId && path.client) bundleId = str(path.client.bundleIdentifier);
      } catch (e) {}
    }
  } catch (e) {}

  try {
    var item = Req.localNowPlayingItem;
    if (item) {
      var info = item.nowPlayingInfo;
      if (info) {
        title = str(info.objectForKey('kMRMediaRemoteNowPlayingInfoTitle'));
        artist = str(info.objectForKey('kMRMediaRemoteNowPlayingInfoArtist'));
        durationSec = num(info.objectForKey('kMRMediaRemoteNowPlayingInfoDuration'));
        positionSec = num(info.objectForKey('kMRMediaRemoteNowPlayingInfoElapsedTime'));
        playing = num(info.objectForKey('kMRMediaRemoteNowPlayingInfoPlaybackRate')) > 0.01;
        if (!playing) {
          var flag = str(info.objectForKey('kMRMediaRemoteNowPlayingInfoIsPlaying'));
          playing = flag === '1' || flag.toLowerCase() === 'true';
        }
        var state = num(info.objectForKey('kMRMediaRemoteNowPlayingInfoPlaybackState'));
        if (state === 1) playing = true;
        if (state === 2) playing = false;
      }
    }
  } catch (e) {}

  if (!bundleId && !title) return '';
  return JSON.stringify({
    bundleId: bundleId,
    title: title,
    artist: artist,
    playing: playing,
    positionSec: positionSec,
    durationSec: durationSec
  });
}
"#;
