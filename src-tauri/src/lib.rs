pub mod network;
pub mod audio;
pub mod vpn;
pub mod storage;
pub mod security;

use tauri::{App, Manager};
use network::{start_node, dial_peer, connect_via_code, get_my_void_code, NetworkState};
use storage::vault::{encrypt_file, decrypt_file, list_vault_files};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub fn setup_mini_window(app: &mut App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Logic to spawn a second window ("mini") handled in frontend or here if needed
    // But user asked for logic to spawn it.
    // We can define it in tauri.conf.json or create it dynamically.
    // For "Picture-in-Picture" logic:
    let handle = app.handle();
    let _main_window = handle.get_webview_window("main").unwrap();
    
    // Listen for minimize event to show mini window
    // Note: Tauri v2 window events are async.
    // This part is tricky to do purely in setup without a loop.
    // Better to handle "minimize" event in frontend and call a command to show mini window.
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(NetworkState::new())
        .setup(|app| {
             // We can create the mini window here but keep it hidden
             #[cfg(desktop)]
             {
                 let _mini = tauri::WebviewWindowBuilder::new(
                     app,
                     "mini",
                     tauri::WebviewUrl::App("/mini".into())
                 )
                 .title("VOID Mini")
                 .inner_size(320.0, 180.0)
                 .decorations(false)
                 .transparent(true)
                 .always_on_top(true)
                 .visible(false) // Start hidden
                 .build()?;
             }
             Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, start_node, dial_peer, connect_via_code, get_my_void_code, network::send_signal, encrypt_file, decrypt_file, list_vault_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
