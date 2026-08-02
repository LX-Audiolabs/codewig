use codewig_core::music::param_catalog::{catalog, reload_catalog, DeviceHostKind};
use codewig_core::music::MusicSession;
use codewig_core::Client;
use std::sync::mpsc;
use std::thread;

mod commands;

slint::include_modules!();

/// Status probe only — short so offline Bitwig does not freeze the UI (commands keep 2s).
const STATUS_TIMEOUT_MS: u64 = 250;

/// Light list only — full param text via [`device_detail_for`] on click (keeps UI fast with big catalogs).
fn device_entries_from_catalog() -> Vec<DeviceEntry> {
    catalog()
        .devices()
        .iter()
        .map(|d| {
            let aliases = if d.aliases.is_empty() {
                String::new()
            } else {
                d.aliases.join(", ")
            };
            let kind = match d.kind {
                DeviceHostKind::Bitwig => "bitwig",
                DeviceHostKind::Clap => "clap",
            };
            let n = d.params.len();
            let summary = if n == 0 {
                format!("{kind} · help only (raw wire OK)")
            } else {
                format!("{kind} · {n} params")
            };
            DeviceEntry {
                name: d.bitwig_name.clone().into(),
                aliases: aliases.into(),
                syntax: format!(".device({})", d.bitwig_name).into(),
                summary: summary.into(),
            }
        })
        .collect()
}

/// Full parameter dump for one device (called when user selects it in the Devices tab).
fn device_detail_for(name: &str) -> String {
    let cat = catalog();
    let Some(d) = cat.resolve(name) else {
        return format!("Unknown device: {name}\n\nDrop a devices/*.yaml and hit ↻.");
    };
    let kind = match d.kind {
        DeviceHostKind::Bitwig => "bitwig",
        DeviceHostKind::Clap => "clap",
    };
    let mut detail = format!(
        "{}\nid: {}\nkind: {}\nsource: {}\n\nParameters ({}):\n",
        d.bitwig_name,
        d.id,
        kind,
        d.source,
        d.params.len()
    );
    if d.params.is_empty() {
        detail.push_str(
            "  (no param help yet)\n\
             Raw WIGSCRIPT still works: track&device: Name(0.5)  // wire 0..1\n",
        );
    } else {
        for p in &d.params {
            let aliases = if p.aliases.is_empty() {
                String::new()
            } else {
                format!(" ({})", p.aliases.join(", "))
            };
            let unit = if p.unit.is_empty() {
                String::new()
            } else {
                format!(" {}", p.unit)
            };
            detail.push_str(&format!(
                "  {}{}: {}..{}{}\n",
                p.name, aliases, p.display.0, p.display.1, unit
            ));
        }
    }
    detail.push_str(&format!(
        "\nWIGSCRIPT:\n  track&{}: param(50)\n",
        d.id
    ));
    detail
}

/// Commands from the UI thread to the worker. `Client` and `MusicSession` are
/// `!Send` (Rc/RefCell inside) — they live **in the worker thread**, only plain
/// strings cross the channel.
enum WorkerCmd {
    Run(String),
    Reconnect,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create + seed the per-user layout once at startup (devices/*.yaml home).
    if let Err(e) = codewig_core::ensure_user_layout() {
        eprintln!("user layout: {e}");
    }

    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Devices tab = scan of devices/*.yaml (not compiled-in).
    ui.set_devices(slint::ModelRc::new(slint::VecModel::from(
        device_entries_from_catalog(),
    )));

    // Non-blocking start: never wait on TCP before first frame.
    ui.set_status("checking…".into());
    let status_weak = ui.as_weak();
    thread::spawn(move || {
        let status = connection_status();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = status_weak.upgrade() {
                ui.set_status(status.into());
            }
        });
    });

    // Worker thread owns Client + MusicSession for the whole UI lifetime.
    // RPC (2s timeout) and the 250ms status probe never run on the UI thread;
    // results come back via invoke_from_event_loop.
    let (tx, rx) = mpsc::channel::<WorkerCmd>();
    let worker_weak = ui.as_weak();
    thread::spawn(move || {
        let client = Client::default();
        let mut session = MusicSession::default();
        for cmd in rx {
            match cmd {
                WorkerCmd::Run(line) => {
                    let result = commands::run(&client, &mut session, &line);
                    // RPC succeeded → socket live; re-check only on failure (short probe)
                    let status = match &result {
                        Ok(_) => "connected".to_string(),
                        Err(_) => connection_status(),
                    };
                    let weak = worker_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = weak.upgrade() else { return };
                        let mut output = ui.get_output().to_string();
                        match &result {
                            Ok(Some(v)) => {
                                output.push_str(&format!(
                                    "{}\n",
                                    serde_json::to_string_pretty(v).unwrap()
                                ));
                            }
                            Ok(None) => output.push_str("ok\n"),
                            Err(e) => output.push_str(&format!("error: {e}\n")),
                        }
                        ui.set_output(output.into());
                        ui.set_status(status.into());
                    });
                }
                WorkerCmd::Reconnect => {
                    client.reset();
                    let status = connection_status();
                    let weak = worker_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = weak.upgrade() else { return };
                        ui.set_status(status.into());
                    });
                }
            }
        }
    });

    // Sidebar filter: substring match (Slint has no string.contains)
    ui.on_text_matches(|haystack, needle| {
        let n = needle.trim().to_lowercase();
        if n.is_empty() {
            return true;
        }
        haystack.to_lowercase().contains(&n)
    });

    // Lazy device params — only when user clicks a device row
    ui.on_device_detail(|name| device_detail_for(name.as_str()).into());

    let reconnect_weak = ui.as_weak();
    let tx_reconnect = tx.clone();
    ui.on_reconnect(move || {
        let Some(ui) = reconnect_weak.upgrade() else { return };
        ui.set_status("checking…".into());
        let _ = tx_reconnect.send(WorkerCmd::Reconnect);
    });

    let reload_weak = ui.as_weak();
    ui.on_reload_devices(move || {
        let n = reload_catalog();
        let load_errors: Vec<String> = catalog().load_errors().to_vec();
        let Some(ui) = reload_weak.upgrade() else { return };
        ui.set_devices(slint::ModelRc::new(slint::VecModel::from(
            device_entries_from_catalog(),
        )));
        ui.set_status(format!("devices: reloaded {n}").into());
        let user_dir = codewig_core::user_devices_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(unknown)".into());
        let errors = if load_errors.is_empty() {
            String::new()
        } else {
            format!("\nYAML errors ({}):\n  {}", load_errors.len(), load_errors.join("\n  "))
        };
        ui.set_help_text(
            format!(
                "Rescanned devices — {n} device(s).\n\
                 User folder: {user_dir}\n\
                 Drop a new YAML (bitwig|clap), click ↻ again.\n\
                 Env: CODEWIG_HOME / CODEWIG_DEVICES_DIR{errors}"
            )
            .into(),
        );
    });

    let insert_weak = ui.as_weak();
    ui.on_insert_command(move |cmd| {
        let Some(ui) = insert_weak.upgrade() else { return };
        ui.set_command(cmd);
    });

    let help_weak = ui.as_weak();
    ui.on_show_help(move |help| {
        let Some(ui) = help_weak.upgrade() else { return };
        ui.set_help_text(help);
    });

    let tx_send = tx.clone();
    ui.on_send_command(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let cmd = ui.get_command().to_string();
        // Echo immediately (UI thread); result arrives via the worker callback.
        let mut output = ui.get_output().to_string();
        output.push_str(&format!("♫ {cmd}\n"));
        ui.set_output(output.into());
        ui.set_command("".into());
        let _ = tx_send.send(WorkerCmd::Run(cmd));
    });

    ui.run()?;
    Ok(())
}

/// Fresh short-timeout client so status never shares the live socket or 2s command timeout.
fn connection_status() -> String {
    let probe = Client::new("127.0.0.1", 9470, STATUS_TIMEOUT_MS);
    match probe.ping() {
        Ok(_) => "connected".to_string(),
        Err(e) => format!("disconnected ({e})"),
    }
}
