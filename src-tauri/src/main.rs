#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod data;
mod export;
mod model;
mod settings;
mod transcript;

use capture::RecordingState;
use data::Database;
use std::sync::Mutex;
use tauri::{
    Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
};

fn main() {
    dotenvy::dotenv().ok();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init());
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .manage(Mutex::new(RecordingState::default()))
        .setup(|app| {
            let data_dir = if cfg!(feature = "e2e") {
                std::env::var_os("SCREENBOX_TEST_DATA_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or(app.path().app_data_dir()?)
            } else {
                app.path().app_data_dir()?
            };
            std::fs::create_dir_all(&data_dir)?;
            app.manage(Database::open(data_dir)?);
            let stop = MenuItemBuilder::with_id("stop", "Stop recording").build(app)?;
            let show = MenuItemBuilder::with_id("show", "Show Screenbox").build(app)?;
            let menu = MenuBuilder::new(app).item(&show).item(&stop).build()?;
            let tray = TrayIconBuilder::with_id("screenbox")
                .icon(app.default_window_icon().ok_or("App icon missing")?.clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "stop" => {
                        let _ = capture::stop_recording_internal(app);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            app.manage(tray);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::list_sources,
            capture::start_recording,
            capture::stop_recording,
            capture::recording_status,
            capture::create_fixture_project,
            data::list_projects,
            data::save_edit,
            data::cursor_track,
            data::get_preferences,
            settings::save_preferences,
            settings::set_tray_language,
            export::export_project,
            export::reveal_in_folder,
            export::copy_file_path,
        ])
        .run(tauri::generate_context!())
        .expect("Screenbox failed to start");
}
