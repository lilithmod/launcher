// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod constants;
mod download;
mod launch;
mod utils;

use log::info;
use slint::{SharedString, StyledText, winit_030::WinitWindowAccessor};
use std::{error::Error, sync::Arc};
use tokio::runtime::Runtime;

use crate::{
    launch::PROCESS,
    utils::config::{LauncherConfig, init_config, save_config},
};
use crate::{
    launch::launch,
    utils::{files, log::Logger},
};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    Logger::init();
    files::init_lilith_dir();

    let config_handle = init_config();

    let rt = Arc::new(Runtime::new().expect("couldnt start up tokio runtime"));

    let ui = AppWindow::new()?;
    let logs_window = LogsWindow::new()?;

    ui.global::<slint_generatedAppWindow::Config>()
        .set_alpha(config_handle.blocking_read().alpha);
    ui.global::<slint_generatedAppWindow::Config>()
        .set_debug(config_handle.blocking_read().debug);

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });
    ui.on_start_drag({
        let ui_handle = ui.as_weak();
        move || {
            let handle = ui_handle.clone();
            slint::spawn_local(async move {
                let window = handle.unwrap();
                let winit_window = window.window().winit_window().await.unwrap();
                let _ = winit_window.drag_window();
            })
            .unwrap();
        }
    });

    ui.on_big_button_clicked({
        let rt_handle = (&rt).clone();
        let ui_handle = ui.as_weak();
        let logs_handle = logs_window.as_weak();
        let config_h = config_handle.clone();
        move || {
            let proxy_state = ui_handle.unwrap().get_proxy_state();
            match proxy_state {
                ProxyState::Idle => {
                    ui_handle.unwrap().set_proxy_state(ProxyState::Starting);
                    let handle = ui_handle.clone();
                    let logs_handle_thatt_is_moved_again = logs_handle.clone();
                    let c = config_h.clone();
                    rt_handle.spawn(async move {
                        for attempt in 1..=3 {
                            info!(target:"launch_init", "attempt {attempt}/4");
                            let res = launch(
                                handle.clone(),
                                c.clone(),
                                logs_handle_thatt_is_moved_again.clone(),
                            )
                            .await;
                            match res {
                                Ok(success) => {
                                    if !success {
                                        continue;
                                    };
                                    break;
                                }
                                Err((title, body)) => {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        let ui_strong = handle.unwrap();
                                        ui_strong.set_proxy_state(ProxyState::Idle);
                                        ui_strong.set_error_popup_title(SharedString::from(title));
                                        ui_strong.set_error_popup_body(
                                            StyledText::from_plain_text(&body),
                                        );
                                        ui_strong.invoke_toggle_error_popup();
                                    });
                                    break;
                                }
                            };
                        }
                    });
                }

                ProxyState::Updating => {
                    info!(target:"ui::big_button_clicked", "updating state: staying idle");
                }
                ProxyState::Starting => {
                    info!(target:"ui::big_button_clicked", "starting state: staying idle");
                }
                ProxyState::Running => {
                    let mut process = PROCESS.lock().unwrap();
                    let a = process.take();
                    if let Some(mut child) = a {
                        let _ = child.kill();
                        ui_handle.unwrap().set_proxy_state(ProxyState::Idle);
                    }
                }
            }
        }
    });

    ui.on_show_logs_window({
        let logs_handle = logs_window.as_weak();
        move || {
            let _ = logs_handle.unwrap().show();
        }
    });

    ui.on_config_popup_closed({
        let ui_handle = ui.as_weak();
        let config = config_handle.clone();

        move || {
            let ui_strong = ui_handle.unwrap();
            let global_slint_config = ui_strong.global::<slint_generatedAppWindow::Config>();
            let alpha = global_slint_config.get_alpha();
            let debug = global_slint_config.get_debug();
            let c = config.clone();

            rt.spawn(async move {
                let mut writable = c.write().await;
                writable.alpha = alpha.clone();
                writable.debug = debug.clone();
                drop(writable);
                save_config(LauncherConfig {
                    alpha: alpha,
                    debug: debug,
                })
                .await
            });
        }
    });

    ui.on_minimize_app({
        let ui_handle = ui.as_weak();
        move || ui_handle.unwrap().window().set_minimized(true)
    });

    ui.on_close_app({
        move || {
            let _ = slint::quit_event_loop();
        }
    });

    ui.run()?;
    // loop exit
    info!(target:"main", "main loop was exited, closing lilith");
    let mut process = PROCESS.lock().unwrap();
    let a = process.take();
    if let Some(mut child) = a {
        let _ = child.kill();
    }
    Ok(())
}
