use crate::protocol::PRODUCT_NAME;
use crate::state::AppState;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

#[cfg(debug_assertions)]
use tauri::{WebviewUrl, WebviewWindowBuilder};

pub struct AutostartItem(pub CheckMenuItem<tauri::Wry>);

pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn toggle_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            show_main(app);
        }
    }
}

pub fn attach(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();
    let show = MenuItem::with_id(handle, "show", "Show pairing", true, None::<&str>)?;
    let enabled = handle.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(
        handle,
        "autostart",
        "Launch at Login",
        true,
        enabled,
        None::<&str>,
    )?;
    let uninstall = MenuItem::with_id(handle, "uninstall", "Uninstall", true, None::<&str>)?;
    let quit = MenuItem::with_id(
        handle,
        "quit",
        format!("Quit {PRODUCT_NAME}"),
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(handle)?;
    #[cfg(debug_assertions)]
    let menu = {
        let logs = MenuItem::with_id(handle, "logs", "Show logs", true, None::<&str>)?;
        let devtools = MenuItem::with_id(handle, "devtools", "Open DevTools", true, None::<&str>)?;
        Menu::with_items(
            handle,
            &[&show, &logs, &devtools, &autostart, &sep, &uninstall, &quit],
        )?
    };
    #[cfg(not(debug_assertions))]
    let menu = Menu::with_items(handle, &[&show, &autostart, &sep, &uninstall, &quit])?;
    handle.manage(AutostartItem(autostart));
    let tray = handle.tray_by_id("caddie").ok_or("tray icon missing")?;
    tray.set_menu(Some(menu))?;
    tray.set_show_menu_on_left_click(false)?;
    Ok(())
}

pub fn on_menu(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "show" => show_main(app),
        "logs" => {
            #[cfg(debug_assertions)]
            show_logs(app);
        }
        "devtools" => {
            #[cfg(debug_assertions)]
            open_devtools(app);
        }
        "autostart" => toggle_autostart(app),
        "uninstall" => prompt_uninstall(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn on_tray(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_main(app);
    }
}

fn prompt_uninstall(app: &AppHandle) {
    app.state::<AppState>().set_uninstall_prompt(true);
    show_main(app);
    let _ = app.emit("uninstall-prompt", ());
}

#[cfg(debug_assertions)]
fn show_logs(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("logs") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "logs", WebviewUrl::App("index.html".into()))
        .title(format!("{PRODUCT_NAME} logs"))
        .inner_size(560.0, 700.0)
        .resizable(true)
        .skip_taskbar(true)
        .initialization_script("window.__CADDIE_VIEW__='logs';")
        .build();
}

#[cfg(debug_assertions)]
fn open_devtools(app: &AppHandle) {
    let focused = app
        .webview_windows()
        .into_values()
        .find(|win| win.is_focused().unwrap_or(false));
    let win = focused.or_else(|| app.get_webview_window("main"));
    if let Some(win) = win {
        win.open_devtools();
    }
}

fn toggle_autostart(app: &AppHandle) {
    let launch = app.autolaunch();
    let on = launch.is_enabled().unwrap_or(false);
    let next = if on {
        launch.disable().is_ok() && false
    } else {
        launch.enable().is_ok()
    };
    if let Some(item) = app.try_state::<AutostartItem>() {
        let _ = item.0.set_checked(next);
    }
}
