#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::Emitter;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

// ✅ ADD: CLI plugin import (otherwise `tauri_plugin_cli::init()` may not resolve)
use tauri_plugin_cli;

// Stores redirect URL + timestamp (TTL)
struct PendingRedirect(Mutex<Option<(String, Instant)>>);

// Tracks whether the main window is currently focused
struct FocusState(Mutex<bool>);

#[tauri::command]
fn notify_from_xano(app: AppHandle, title: String, body: String, redirect_url: Option<String>) {
  // Always show notification
  let _ = app.notification().builder().title(title).body(body).show();

  // If no redirect -> done
  let Some(url) = redirect_url else {
    return;
  };
  let url = url.trim().to_string();
  if url.is_empty() {
    return;
  }

  // Store pending ONLY if app is NOT focused right now
  let focused_now = {
    let fs = app.state::<FocusState>();
    let v = fs.0.lock().unwrap();
    *v
  };

  if focused_now {
    // user is already in the app => do not store
    return;
  }

  let pending = app.state::<PendingRedirect>();
  let mut guard = pending.0.lock().unwrap();
  *guard = Some((url, Instant::now()));
}

#[tauri::command]
fn consume_pending_redirect(app: AppHandle) -> Option<String> {
  let pending = app.state::<PendingRedirect>();

  let maybe = {
    let guard = pending.0.lock().unwrap();
    guard.clone()
  };

  let Some((url, ts)) = maybe else {
    return None;
  };

  // TTL 10 minutes
  if ts.elapsed() > Duration::from_secs(10 * 60) {
    let mut guard = pending.0.lock().unwrap();
    *guard = None;
    return None;
  }

  // Consume now
  let mut guard = pending.0.lock().unwrap();
  *guard = None;
  Some(url)
}

fn emit_redirect_event_if_pending(app: &AppHandle) {
  // If something is pending, emit a “click semantics” event.
  // Frontend will call consume_pending_redirect().
  let pending = app.state::<PendingRedirect>();
  let has_pending = {
    let guard = pending.0.lock().unwrap();
    guard.is_some()
  };

  if has_pending {
    let _ = app.emit("korner://redirect", ());
  }
}

fn main() {
  tauri::Builder::default()
    // ✅ CLI plugin enabled
    .plugin(tauri_plugin_cli::init())
    .plugin(tauri_plugin_notification::init())
    .manage(PendingRedirect(Mutex::new(None)))
    .manage(FocusState(Mutex::new(false)))
    .invoke_handler(tauri::generate_handler![notify_from_xano, consume_pending_redirect])
    .on_window_event(|window, event| {
      if let tauri::WindowEvent::Focused(focused) = event {
        let app = window.app_handle();
        let focused = *focused; // ✅ on transforme &bool -> bool
      
        // update focus state
        {
          let fs = app.state::<FocusState>();
          let mut v = fs.0.lock().unwrap();
          *v = focused;
        }
      
        // When app becomes focused (typical after notif click), emit event
        if focused {
          emit_redirect_event_if_pending(&app);
        }
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}