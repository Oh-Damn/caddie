#![cfg(target_os = "macos")]

use super::AppInfo;

pub enum WatchEvent {

    App(AppInfo),

    Media,
}
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct AppWatch {
    child: Arc<Mutex<Option<Child>>>,
    stop: Arc<Mutex<bool>>,
}

impl AppWatch {
    pub fn spawn<F>(on_change: F) -> Self
    where
        F: Fn(WatchEvent) + Send + Sync + 'static,
    {
        let child = Arc::new(Mutex::new(None));
        let stop = Arc::new(Mutex::new(false));
        let handler = Arc::new(on_change);

        let child_slot = child.clone();
        let stop_flag = stop.clone();
        std::thread::spawn(move || loop {
            if *stop_flag.lock().expect("lock") {
                return;
            }
            match run_once(&child_slot, handler.as_ref()) {
                Ok(()) => {}
                Err(err) => tracing::warn!("app watcher: {err}"),
            }
            if *stop_flag.lock().expect("lock") {
                return;
            }

            std::thread::sleep(std::time::Duration::from_secs(3));
        });

        Self { child, stop }
    }
}

impl Drop for AppWatch {
    fn drop(&mut self) {
        *self.stop.lock().expect("lock") = true;
        if let Some(mut child) = self.child.lock().expect("lock").take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_once<F>(slot: &Arc<Mutex<Option<Child>>>, on_change: &F) -> Result<(), String>
where
    F: Fn(WatchEvent) + Send + Sync,
{
    let mut child = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", WATCH_JS])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout".to_string())?;
    *slot.lock().expect("lock") = Some(child);

    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let mut parts = line.splitn(3, '\t');
        match parts.next().unwrap_or("") {
            "media" => on_change(WatchEvent::Media),
            "app" => {
                let bundle_id = parts.next().unwrap_or("").trim().to_string();
                let name = parts.next().unwrap_or("").trim().to_string();
                if bundle_id.is_empty() || super::is_companion(&bundle_id) {
                    continue;
                }
                on_change(WatchEvent::App(AppInfo { name, bundle_id }));
            }
            _ => {}
        }
    }

    if let Some(mut child) = slot.lock().expect("lock").take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

const WATCH_JS: &str = r#"
function run() {
  ObjC.import('AppKit');
  var out = $.NSFileHandle.fileHandleWithStandardOutput;
  function write(line) {
    var s = $.NSString.alloc.initWithUTF8String(line + '\n');
    out.writeData(s.dataUsingEncoding($.NSUTF8StringEncoding));
  }
  function emit(app) {
    if (!app) return;
    var bid = ObjC.unwrap(app.bundleIdentifier) || '';
    var nm = ObjC.unwrap(app.localizedName) || '';
    if (!bid) return;
    write('app\t' + bid + '\t' + nm);
  }
  var ws = $.NSWorkspace.sharedWorkspace;
  ws.notificationCenter.addObserverForNameObjectQueueUsingBlock(
    'NSWorkspaceDidActivateApplicationNotification',
    $(),
    $.NSOperationQueue.mainQueue,
    function (note) {
      emit(note.userInfo.objectForKey('NSWorkspaceApplicationKey'));
    }
  );
  var dnc = $.NSDistributedNotificationCenter.defaultCenter;
  ['com.spotify.client.PlaybackStateChanged', 'com.apple.iTunes.playerInfo'].forEach(
    function (name) {
      dnc.addObserverForNameObjectQueueUsingBlock(
        name,
        $(),
        $.NSOperationQueue.mainQueue,
        function () {
          write('media');
        }
      );
    }
  );
  emit(ws.frontmostApplication);
  $.NSRunLoop.currentRunLoop.run;
}
"#;
