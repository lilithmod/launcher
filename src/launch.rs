use std::{
    fs::remove_file,
    io::{BufRead, BufReader},
    process::{Child, Command},
    ptr::null_mut,
    sync::{Arc, LazyLock},
    thread,
};

use log::{error, info};
use slint::{SharedString, Weak};
use strip_ansi_escapes::strip_str;
use tokio::sync::RwLock;

pub static PROCESS: LazyLock<std::sync::Mutex<Option<Child>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

use crate::{
    AppWindow, LogsWindow,
    download::{compute_file_hash, download_artifact, fetch_release, get_artifact_from_release},
    utils::{config::LauncherConfig, files::bin_dir},
};
/**
 * `Ok(true)` -> success
 * `Ok(false)` -> non-fatal error: retry
 * `Err((title, body))` -> fatal error requiring intervention: human readable string describing the error
 */
pub async fn launch(
    app_handle: Weak<AppWindow>,
    config_handle: Arc<RwLock<LauncherConfig>>,
    logs_handle: Weak<LogsWindow>,
) -> Result<bool, (String, String)> {
    let config = config_handle.read().await;

    info!(target:"launch", "loaded config: {config:?}");

    let possible_child = {
        let mut process = PROCESS.lock().unwrap();
        process.take()
    };
    if let Some(mut child) = possible_child {
        info!(target:"launch", "for some reasons a process was already running!");
        let _ = child.kill();
        let _ = child.wait();
    }

    match fetch_release(config.alpha).await {
        Ok(release) => {
            if let Some(artifact) = get_artifact_from_release(release) {
                let expected_path = bin_dir().join(artifact.name);

                if let Ok(true) = tokio::fs::try_exists(&expected_path).await {
                    let artifact_hash =
                        hex::decode(artifact.digest.get(7..).unwrap_or("67")).unwrap();
                    let file_hash = compute_file_hash(&expected_path).unwrap();

                    if file_hash.as_slice() != artifact_hash.as_slice() {
                        info!(target:"launch","file hashes did not match, removing file");

                        let _ = remove_file(expected_path);
                        Ok(false)
                    } else {
                        run(app_handle, &expected_path, logs_handle, config.debug)
                    }
                } else {
                    {
                        let handle = app_handle.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            handle.unwrap().set_proxy_state(crate::ProxyState::Updating);
                        });
                    }

                    match download_artifact(
                        app_handle.clone(),
                        artifact.url,
                        &expected_path,
                        artifact.size,
                    )
                    .await
                    {
                        Ok(_) => run(app_handle, &expected_path, logs_handle, config.debug),
                        Err(e) => Err(("Download Error:".to_string(), e.to_string())),
                    }
                }
            } else {
                Err(("Error:".to_string(), "Platform not supported!".to_string()))
            }
        }

        Err(e) => Err(("Error: Release".to_string(), e.to_string())),
    }
}

pub fn run(
    ui_handle: Weak<AppWindow>,
    exec_path: &std::path::Path,
    logs_handle: Weak<LogsWindow>,
    debug: bool,
) -> Result<bool, (String, String)> {
    info!(target:"run", "running lilith living at {}", exec_path.as_os_str().display());

    #[cfg(target_os = "macos")]
    {
        info!(target:"run", "removing quarantine flags");
        let _ = Command::new("xattr")
            .args(["-cr", exec_path.to_str().unwrap()])
            .status();
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        info!(target:"run", "making lilith an executable");
        let _ = Command::new("chmod")
            .args(["+x", exec_path.to_str().unwrap()])
            .status();
    }

    let (out, writer) = std::io::pipe().map_err(|e| {
        error!(target:"launch::pipe", "couldn't create pipes: {e}");
        ("Launch Error:".to_string(), e.to_string())
    })?;
    let cmd = Command::new(exec_path)
        .args(["--iknowwhatimdoing"])
        .args(debug.then_some("--debug"))
        .stdout(writer.try_clone().map_err(|e| {
            error!(target:"launch::pipe", "couldn't duplicate pipe: {e}");
            ("Launch Error:".to_string(), e.to_string())
        })?)
        .stderr(writer)
        .spawn()
        .map_err(|e| {
            error!(target:"run", "got a {} error while starting lilith: {e}", e.kind());
            ("Runtime Error:".to_string(), e.to_string())
        })?;

    waitpid(cmd.id(), ui_handle.clone());

    PROCESS.lock().unwrap().replace(cmd);

    let _ = slint::invoke_from_event_loop(move || {
        ui_handle
            .unwrap()
            .set_proxy_state(crate::ProxyState::Running);
    });

    let reader = BufReader::new(out);
    for line in reader.lines() {
        let line_content = line.map_or_else(
            |_| "failed to get log line :(".to_string(),
            |text| strip_str(text),
        );
        info!(target:"proxy","{}", line_content);

        let logs_ui = logs_handle.clone();
        let _ = slint::invoke_from_event_loop(move || {
            logs_ui
                .unwrap()
                .invoke_update_logs(SharedString::from(line_content));
        });
    }
    Ok(true)
}

fn waitpid(pid: u32, ui_handle_for_wait: Weak<AppWindow>) {
    thread::spawn(move || {
        #[cfg(not(target_os = "windows"))]
        {
            let status: *mut i32 = null_mut();
            unsafe {
                info!(target: "launch::wait", "starting to wait");
                libc::waitpid(pid as i32, status, 0);
            }
        }
        #[cfg(target_os = "windows")]
        {
            use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
            let s = System::new_with_specifics(
                RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
            );
            if let Some(running) = s.process(Pid::from_u32(pid)) {
                info!(target: "launch::wait", "starting to wait");
                let _ = running.wait();
            }
        }
        let _ = slint::invoke_from_event_loop(move || {
            ui_handle_for_wait
                .unwrap()
                .set_proxy_state(crate::ProxyState::Idle);
        });
    });
}
