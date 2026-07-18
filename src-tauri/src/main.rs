// Prevents additional console window on Windows in release, unused on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const DEFAULT_MAX_HISTORY: usize = 50;
const MIN_MAX_HISTORY: usize = 1;
const ABSOLUTE_MAX_HISTORY: usize = 250;
const POPUP_LABEL: &str = "popup";
const PREFERENCES_LABEL: &str = "preferences";
const TRAY_ID: &str = "hayclip-tray";

struct ClipboardState {
    history: VecDeque<String>,
    last_seen: String,
    max_history: usize,
}

impl ClipboardState {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            last_seen: String::new(),
            max_history: DEFAULT_MAX_HISTORY,
        }
    }

    fn remember(&mut self, text: String) {
        self.last_seen = text.clone();
        self.history.retain(|existing| existing != &text);
        self.history.push_front(text);
        self.trim();
    }

    fn trim(&mut self) {
        while self.history.len() > self.max_history {
            self.history.pop_back();
        }
    }
}

type SharedState = Mutex<ClipboardState>;

#[derive(Serialize, Clone)]
struct HistoryPayload {
    items: Vec<String>,
}

fn emit_history(app: &AppHandle, state: &ClipboardState) {
    let payload = HistoryPayload {
        items: state.history.iter().cloned().collect(),
    };
    let _ = app.emit("history-updated", payload);
}

#[tauri::command]
fn get_history(state: State<SharedState>) -> Vec<String> {
    state.lock().unwrap().history.iter().cloned().collect()
}

#[tauri::command]
fn copy_and_hide(app: AppHandle, state: State<SharedState>, text: String) -> Result<(), String> {
    {
        let mut guard = state.lock().unwrap();
        guard.remember(text.clone());
        emit_history(&app, &guard);
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
}

#[tauri::command]
fn clear_history(app: AppHandle) {
    clear_history_internal(&app);
}

#[tauri::command]
fn hide_popup_cmd(app: AppHandle) {
    hide_popup(&app);
}

#[derive(Serialize, Clone)]
struct PreferencesPayload {
    max_history: usize,
    min: usize,
    max: usize,
}

#[tauri::command]
fn get_preferences(state: State<SharedState>) -> PreferencesPayload {
    PreferencesPayload {
        max_history: state.lock().unwrap().max_history,
        min: MIN_MAX_HISTORY,
        max: ABSOLUTE_MAX_HISTORY,
    }
}

#[tauri::command]
fn set_max_history(app: AppHandle, state: State<SharedState>, value: usize) -> usize {
    let clamped = value.clamp(MIN_MAX_HISTORY, ABSOLUTE_MAX_HISTORY);
    let mut guard = state.lock().unwrap();
    guard.max_history = clamped;
    guard.trim();
    emit_history(&app, &guard);
    clamped
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
    let version = app.package_info().version.to_string();
    app.dialog()
        .message(format!(
            "HayClip v{version}\nA minimal clipboard manager for your menu bar.\n\nBy Razmik Baghdasaryan\nMIT License"
        ))
        .title("About HayClip")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
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
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
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
            set_max_history
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyV);
            app.global_shortcut().register(shortcut)?;

            let about_item = MenuItemBuilder::with_id("about", "About HayClip").build(app)?;
            let show_item = MenuItemBuilder::with_id("show", "Show HayClip").build(app)?;
            let clear_item = MenuItemBuilder::with_id("clear", "Clear History").build(app)?;
            let preferences_item =
                MenuItemBuilder::with_id("preferences", "Preferences…").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit HayClip").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&about_item])
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

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window.hide().ok();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running HayClip");
}
