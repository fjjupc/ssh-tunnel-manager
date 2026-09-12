mod commands;
mod config;
mod models;
mod tunnel;

use config::{config_path, load_config};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tunnel::SharedState;

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(true);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir)?;
            let cfg_path = config_path(&data_dir);
            let cfg = load_config(&cfg_path).unwrap_or_default();
            CLOSE_TO_TRAY.store(cfg.close_to_tray, Ordering::SeqCst);
            let state = SharedState::new(cfg.clone(), cfg_path);
            app.manage(state.clone());

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                state.set_app_handle(handle).await;
                let _ = state.restore_enabled().await;
            });

            #[cfg(desktop)]
            {
                use tauri_plugin_autostart::ManagerExt;
                let launcher = app.autolaunch();
                let _ = if cfg.auto_start {
                    launcher.enable()
                } else {
                    launcher.disable()
                };
            }

            let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing default window icon for tray"))?;

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("SSH Tunnel Manager")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        ALLOW_EXIT.store(true, Ordering::SeqCst);
                        let state = app.state::<SharedState>().inner().clone();
                        let app2 = app.clone();
                        tauri::async_runtime::spawn(async move {
                            state.shutdown_all().await;
                            app2.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if ALLOW_EXIT.load(Ordering::SeqCst) {
                    return;
                }
                if CLOSE_TO_TRAY.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    ALLOW_EXIT.store(true, Ordering::SeqCst);
                    let state = window.app_handle().state::<SharedState>().inner().clone();
                    let app = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        state.shutdown_all().await;
                        app.exit(0);
                    });
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_device_statuses,
            commands::add_device,
            commands::remove_device,
            commands::update_device,
            commands::set_device_enabled,
            commands::add_mapping,
            commands::remove_mapping,
            commands::set_mapping_enabled,
            commands::update_mapping,
            commands::get_default_key_path,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = &event {
            if !ALLOW_EXIT.load(Ordering::SeqCst) && CLOSE_TO_TRAY.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
        if let RunEvent::Exit = event {
            let state = app_handle.state::<SharedState>().inner().clone();
            tauri::async_runtime::block_on(async {
                state.shutdown_all().await;
            });
        }
    });
}

pub fn set_close_to_tray_flag(value: bool) {
    CLOSE_TO_TRAY.store(value, Ordering::SeqCst);
}

pub fn allow_exit() {
    ALLOW_EXIT.store(true, Ordering::SeqCst);
}
