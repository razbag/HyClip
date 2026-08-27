// Prevents additional console window on Windows in release, unused on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::{ManagerExt as AutostartManagerExt, MacosLauncher};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;

const DEFAULT_MAX_HISTORY: usize = 50;
const MIN_MAX_HISTORY: usize = 1;
const ABSOLUTE_MAX_HISTORY: usize = 250;
const POPUP_LABEL: &str = "popup";
const PREFERENCES_LABEL: &str = "preferences";
const ABOUT_LABEL: &str = "about";
const UPDATE_LABEL: &str = "update";
const TRAY_ID: &str = "hyclip-tray";

#[derive(Serialize, Deserialize, Clone)]
struct HistoryEntry {
    text: String,
    copied_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Retention {
    Week,
    Month,
    Forever,
}

impl Retention {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "week" => Some(Retention::Week),
            "month" => Some(Retention::Month),
            "forever" => Some(Retention::Forever),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Retention::Week => "week",
            Retention::Month => "month",
            Retention::Forever => "forever",
        }
    }

    fn max_age_secs(&self) -> Option<i64> {
        match self {
            Retention::Week => Some(7 * 24 * 3600),
            Retention::Month => Some(30 * 24 * 3600),
            Retention::Forever => None,
        }
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

struct ClipboardState {
    history: VecDeque<HistoryEntry>,
    last_seen: String,
    max_history: usize,
    retention: Retention,
}

impl ClipboardState {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            last_seen: String::new(),
            max_history: DEFAULT_MAX_HISTORY,
            retention: Retention::Forever,
        }
    }

    fn remember(&mut self, text: String) {
        self.last_seen = text.clone();
        self.history.retain(|existing| existing.text != text);
        self.history.push_front(HistoryEntry {
            text,
            copied_at: now_secs(),
        });
        self.prune_expired();
        self.trim();
    }

    fn trim(&mut self) {
        while self.history.len() > self.max_history {
            self.history.pop_back();
        }
    }

    fn prune_expired(&mut self) {
        if let Some(max_age) = self.retention.max_age_secs() {
            let cutoff = now_secs() - max_age;
            self.history.retain(|entry| entry.copied_at >= cutoff);
        }
    }
}

type SharedState = Mutex<ClipboardState>;

#[derive(Serialize, Deserialize, Clone)]
struct PersistedPrefs {
    max_history: usize,
    retention: Retention,
}

fn app_data_dir(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().data_dir().ok()?.join("HyClip");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn history_file(app: &AppHandle) -> Option<PathBuf> {
    app_data_dir(app).map(|d| d.join("history.json"))
}

fn prefs_file(app: &AppHandle) -> Option<PathBuf> {
    app_data_dir(app).map(|d| d.join("preferences.json"))
}

fn save_history(app: &AppHandle, state: &ClipboardState) {
    let Some(path) = history_file(app) else { return };
    let items: Vec<&HistoryEntry> = state.history.iter().collect();
    if let Ok(json) = serde_json::to_string_pretty(&items) {
        let _ = std::fs::write(path, json);
    }
}

fn save_prefs(app: &AppHandle, state: &ClipboardState) {
    let Some(path) = prefs_file(app) else { return };
    let prefs = PersistedPrefs {
        max_history: state.max_history,
        retention: state.retention,
    };
    if let Ok(json) = serde_json::to_string_pretty(&prefs) {
        let _ = std::fs::write(path, json);
    }
}

/// Loads persisted history/preferences from disk (if any) into the already-
/// managed state. Runs in `setup()`, since resolving the app data dir needs
/// an `AppHandle` that doesn't exist yet when `.manage()` is first called.
fn load_persisted_state(app: &AppHandle) {
    let state = app.state::<SharedState>();
    let mut guard = state.lock().unwrap();

    if let Some(path) = prefs_file(app) {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<PersistedPrefs>(&json) {
                guard.max_history = prefs.max_history.clamp(MIN_MAX_HISTORY, ABSOLUTE_MAX_HISTORY);
                guard.retention = prefs.retention;
            }
        }
    }

    if let Some(path) = history_file(app) {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&json) {
                guard.history = entries.into();
            }
        }
    }

    guard.prune_expired();
    guard.trim();
    save_history(app, &guard);
}

#[derive(Serialize, Clone)]
struct HistoryPayload {
    items: Vec<String>,
}

fn emit_history(app: &AppHandle, state: &ClipboardState) {
    let payload = HistoryPayload {
        items: state.history.iter().map(|e| e.text.clone()).collect(),
    };
    let _ = app.emit("history-updated", payload);
}

#[tauri::command]
fn get_history(state: State<SharedState>) -> Vec<String> {
    state.lock().unwrap().history.iter().map(|e| e.text.clone()).collect()
}

#[tauri::command]
fn copy_and_hide(app: AppHandle, state: State<SharedState>, text: String) -> Result<(), String> {
    {
        let mut guard = state.lock().unwrap();
        guard.remember(text.clone());
        emit_history(&app, &guard);
        save_history(&app, &guard);
    }
    app.clipboard().write_text(text).map_err(|e| e.to_string())?;
    hide_popup(&app);
    Ok(())
}

fn clear_history_internal(app: &AppHandle) {
    let state = app.state::<SharedState>();
    let mut guard = state.lock().unwrap();
    guard.history.clear();
    guard.last_seen.clear();
    emit_history(app, &guard);
    save_history(app, &guard);
}

#[tauri::command]
fn clear_history(app: AppHandle) {
    clear_history_internal(&app);
}

#[tauri::command]
fn hide_popup_cmd(app: AppHandle) {
    hide_popup(&app);
}

#[tauri::command]
fn hide_update_window_cmd(app: AppHandle) {
    if let Some(window) = app.get_webview_window(UPDATE_LABEL) {
        let _ = window.hide();
    }
}

#[derive(Serialize, Clone)]
struct AboutInfo {
    name: String,
    version: String,
}

#[tauri::command]
fn get_about_info(app: AppHandle) -> AboutInfo {
    let info = app.package_info();
    AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
    }
}

#[derive(Serialize, Clone)]
struct UpdateInfo {
    resource_id: tauri::ResourceId,
    version: String,
    current_version: String,
    body: Option<String>,
}

async fn run_update_check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let info = UpdateInfo {
        resource_id: 0,
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        body: update.body.clone(),
    };
    let rid = app.resources_table().add(update);
    Ok(Some(UpdateInfo {
        resource_id: rid,
        ..info
    }))
}

#[tauri::command]
async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    run_update_check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle, resource_id: tauri::ResourceId) -> Result<(), String> {
    let update = app
        .resources_table()
        .take::<tauri_plugin_updater::Update>(resource_id)
        .map_err(|e| e.to_string())?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    relaunch_and_exit();
}

// `AppHandle::restart()` spawns the new process while this one is still
// alive, and macOS's LaunchServices sometimes kills the new instance as a
// "duplicate" of a menu-bar/accessory app before it finishes registering.
// Relaunching via `open -n` after this process has fully exited avoids the
// overlap.
fn relaunch_and_exit() -> ! {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bundle) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
            let _ = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(format!("sleep 1 && open -n {:?}", bundle))
                .spawn();
        }
    }
    std::process::exit(0);
}

const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 3600);

async fn check_and_notify_if_available(app: &AppHandle) {
    if let Ok(Some(info)) = run_update_check(app).await {
        if let Some(window) = app.get_webview_window(UPDATE_LABEL) {
            let _ = window.center();
            let _ = window.show();
            let _ = window.set_focus();
            let _ = app.emit("update-available", info);
        }
    }
}

/// Checks for an update shortly after launch, then again every 24 hours for
/// as long as the app keeps running — HyClip is a menu bar app that can stay
/// open for weeks, so a launch-only check would leave long sessions stale.
/// Silent unless an update is actually found; the tray menu's "Check for
/// Updates…" item covers on-demand checks (and surfaces errors) separately.
fn start_periodic_update_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        thread::sleep(Duration::from_secs(3));
        loop {
            check_and_notify_if_available(&app).await;
            thread::sleep(UPDATE_CHECK_INTERVAL);
        }
    });
}

#[derive(Serialize, Clone)]
struct PreferencesPayload {
    max_history: usize,
    min: usize,
    max: usize,
    launch_at_login: bool,
    retention: &'static str,
}

#[tauri::command]
fn get_preferences(app: AppHandle, state: State<SharedState>) -> PreferencesPayload {
    let guard = state.lock().unwrap();
    PreferencesPayload {
        max_history: guard.max_history,
        min: MIN_MAX_HISTORY,
        max: ABSOLUTE_MAX_HISTORY,
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        retention: guard.retention.as_str(),
    }
}

#[tauri::command]
fn set_max_history(app: AppHandle, state: State<SharedState>, value: usize) -> usize {
    let clamped = value.clamp(MIN_MAX_HISTORY, ABSOLUTE_MAX_HISTORY);
    let mut guard = state.lock().unwrap();
    guard.max_history = clamped;
    guard.trim();
    emit_history(&app, &guard);
    save_history(&app, &guard);
    save_prefs(&app, &guard);
    clamped
}

#[tauri::command]
fn set_retention(app: AppHandle, state: State<SharedState>, value: String) -> &'static str {
    let retention = Retention::parse(&value).unwrap_or(Retention::Forever);
    let mut guard = state.lock().unwrap();
    guard.retention = retention;
    guard.prune_expired();
    emit_history(&app, &guard);
    save_history(&app, &guard);
    save_prefs(&app, &guard);
    retention.as_str()
}

#[tauri::command]
fn set_launch_at_login(app: AppHandle, enabled: bool) -> bool {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(err) = result {
        eprintln!("[hyclip] failed to update launch-at-login: {err}");
    }
    autolaunch.is_enabled().unwrap_or(false)
}

fn hide_popup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        let _ = window.hide();
    }
}

fn position_popup_near_tray(app: &AppHandle, window: &tauri::WebviewWindow) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        let _ = window.center();
        return;
    };
    let Ok(Some(tray_rect)) = tray.rect() else {
        let _ = window.center();
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let tray_pos = tray_rect.position.to_physical::<i32>(scale);
    let tray_size = tray_rect.size.to_physical::<u32>(scale);
    let window_size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(320, 400));

    let gap = 6;
    let mut x = tray_pos.x + tray_size.width as i32 / 2 - window_size.width as i32 / 2;
    let y = tray_pos.y + tray_size.height as i32 + gap;

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let min_x = m_pos.x + 4;
        let max_x = m_pos.x + m_size.width as i32 - window_size.width as i32 - 4;
        x = x.clamp(min_x, max_x.max(min_x));
    }

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
}

fn show_popup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        position_popup_near_tray(app, &window);
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("popup-shown", ());
    }
}

fn show_about(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(ABOUT_LABEL) {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_update_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(UPDATE_LABEL) {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("check-updates", ());
    }
}

fn show_preferences(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PREFERENCES_LABEL) {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("preferences-shown", ());
    }
}

fn toggle_popup(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            show_popup(app);
        }
    }
}

/// Enables "launch at login" the very first time HyClip ever runs, then
/// leaves the user's choice (on or off) alone on every later launch. A tiny
/// marker file in the app's data dir is the only thing that needs to persist
/// for this — the login-item state itself is already persisted by macOS.
fn apply_first_run_defaults(app: &AppHandle) {
    let Some(data_dir) = app_data_dir(app) else {
        return;
    };
    let marker = data_dir.join(".defaults-applied");
    if marker.exists() {
        return;
    }
    if let Err(err) = app.autolaunch().enable() {
        eprintln!("[hyclip] failed to enable default launch-at-login: {err}");
    }
    let _ = std::fs::write(&marker, b"");
}

fn start_clipboard_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(400));
        let text = match app.clipboard().read_text() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if text.trim().is_empty() {
            continue;
        }
        let state = app.state::<SharedState>();
        let mut guard = state.lock().unwrap();
        if text == guard.last_seen {
            continue;
        }
        guard.remember(text);
        emit_history(&app, &guard);
        save_history(&app, &guard);
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let toggle_shortcut =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyV);
                    if shortcut == &toggle_shortcut && event.state() == ShortcutState::Pressed {
                        toggle_popup(app);
                    }
                })
                .build(),
        )
        .manage(Mutex::new(ClipboardState::new()))
        .invoke_handler(tauri::generate_handler![
            get_history,
            copy_and_hide,
            clear_history,
            hide_popup_cmd,
            get_preferences,
            set_max_history,
            set_retention,
            set_launch_at_login,
            get_about_info,
            check_for_update,
            install_update,
            hide_update_window_cmd
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyV);
            app.global_shortcut().register(shortcut)?;

            load_persisted_state(app.handle());
            apply_first_run_defaults(app.handle());

            let about_item = MenuItemBuilder::with_id("about", "About HyClip").build(app)?;
            let show_item = MenuItemBuilder::with_id("show", "Show HyClip").build(app)?;
            let clear_item = MenuItemBuilder::with_id("clear", "Clear History").build(app)?;
            let preferences_item =
                MenuItemBuilder::with_id("preferences", "Preferences…").build(app)?;
            let check_updates_item =
                MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit HyClip").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&about_item, &check_updates_item])
                .separator()
                .items(&[&show_item, &preferences_item, &clear_item])
                .separator()
                .items(&[&quit_item])
                .build()?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-icon.png"
            ))?;

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "about" => show_about(app),
                    "show" => show_popup(app),
                    "clear" => clear_history_internal(app),
                    "preferences" => show_preferences(app),
                    "check-updates" => show_update_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_popup(tray.app_handle());
                    }
                })
                .build(app)?;

            if let Some(popup) = app.get_webview_window(POPUP_LABEL) {
                let handle = app.handle().clone();
                popup.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        hide_popup(&handle);
                    }
                });
            }

            start_clipboard_watcher(app.handle().clone());
            start_periodic_update_checks(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().ok();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running HyClip");
}
