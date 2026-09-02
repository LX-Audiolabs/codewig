use codewig_core::Client;
use codewig_core::music::MusicSession;
use codewig_core::music::{
    DeviceHostKind,
    param_catalog::{catalog, reload_catalog},
};
use std::sync::mpsc;
use std::thread;

mod commands;

slint::include_modules!();

/// Status probe only — short so offline Bitwig does not freeze the UI (commands keep 2s).
const STATUS_TIMEOUT_MS: u64 = 250;

/// Devices tab entries from the alias catalog (`devices/aliases.yml`).
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
            let summary = if aliases.is_empty() {
                format!("{kind} · no aliases")
            } else {
                format!("{kind} · {}", aliases)
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

/// Command reference entries from `commands.yaml` (loaded at runtime).
#[derive(Debug, serde::Deserialize)]
struct CommandRefYaml {
    name: String,
    summary: String,
    syntax: String,
    detail: String,
    tag: String,
}

#[derive(Debug, serde::Deserialize)]
struct CommandsYaml {
    commands: Vec<CommandRefYaml>,
}

fn commands_yaml_path() -> Option<std::path::PathBuf> {
    // 1. Next to the executable (packaged / installed builds).
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let p = dir.join("commands.yaml");
        if p.exists() {
            return Some(p);
        }
    }
    // 2. Current working directory.
    let p = std::path::PathBuf::from("commands.yaml");
    if p.exists() {
        return Some(p);
    }
    // 3. Cargo manifest dir (dev builds from crate root).
    let p = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/commands.yaml"));
    if p.exists() {
        return Some(p);
    }
    None
}

fn load_command_refs() -> Result<Vec<RefEntry>, String> {
    let path = commands_yaml_path().ok_or("commands.yaml not found")?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("commands.yaml: {e}"))?;
    let parsed: CommandsYaml =
        serde_norway::from_str(&text).map_err(|e| format!("commands.yaml: {e}"))?;
    Ok(parsed
        .commands
        .into_iter()
        .map(|c| RefEntry {
            name: c.name.into(),
            summary: c.summary.into(),
            syntax: c.syntax.into(),
            detail: c.detail.into(),
            tag: c.tag.into(),
        })
        .collect())
}

/// Alias + page-model cheat sheet for the Devices tab detail box.
fn device_detail_for(name: &str) -> String {
    let cat = catalog();
    let Some(d) = cat.resolve(name) else {
        return format!("Unknown device: {name}\n\nAdd it to devices/aliases.yml and hit ↻.");
    };
    let kind = match d.kind {
        DeviceHostKind::Bitwig => "bitwig",
        DeviceHostKind::Clap => "clap",
    };
    let aliases = if d.aliases.is_empty() {
        String::from("(none)")
    } else {
        d.aliases.join(", ")
    };
    format!(
        "{}\n\
         id: {}\n\
         kind: {}\n\
         aliases: {}\n\n\
         Insert:\n\
           .device({})\n\
           .device({})\n\n\
         Device page (8 Remote Control slots):\n\
           t(mytrack).device({}).page(list)\n\
           t(mytrack).device({}).page(cutoff=0.3)\n\n\
         Inline on a note line (same track, named device):\n\
           mytrack: n \"c e g\" +{}.cutoff:0.3\n\n\
         Track Perform page (no device):\n\
           t(mytrack).perform(list)\n\
           mytrack: n \"c e g\" +cutoff:0.3\n\n\
         Use list first to see the slot names Bitwig exposes.",
        d.bitwig_name,
        d.id,
        kind,
        aliases,
        d.bitwig_name,
        d.id,
        d.bitwig_name,
        d.bitwig_name,
        d.bitwig_name,
    )
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

    // Devices tab = scan of devices/aliases.yml (not compiled-in).
    ui.set_devices(slint::ModelRc::new(slint::VecModel::from(
        device_entries_from_catalog(),
    )));

    // Commands tab = load command reference from commands.yaml at runtime.
    match load_command_refs() {
        Ok(refs) => {
            ui.set_refs(slint::ModelRc::new(slint::VecModel::from(refs)));
        }
        Err(e) => {
            eprintln!("{e}");
            ui.set_status(format!("error: {e}").into());
        }
    }

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
        let Some(ui) = reconnect_weak.upgrade() else {
            return;
        };
        ui.set_status("checking…".into());
        let _ = tx_reconnect.send(WorkerCmd::Reconnect);
    });

    let reload_weak = ui.as_weak();
    ui.on_reload_devices(move || {
        let n = reload_catalog();
        let load_errors: Vec<String> = catalog().load_errors().to_vec();
        let Some(ui) = reload_weak.upgrade() else {
            return;
        };
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
            format!(
                "\nYAML errors ({}):\n  {}",
                load_errors.len(),
                load_errors.join("\n  ")
            )
        };
        ui.set_help_text(
            format!(
                "Reloaded alias catalog — {n} device(s).\n\
                 User folder: {user_dir}\n\
                 Edit devices/aliases.yml (bitwig|clap) and click ↻ again.\n\
                 Per-device YAMLs are no longer used; only aliases.yml is loaded.\n\
                 Env: CODEWIG_HOME / CODEWIG_DEVICES_DIR{errors}"
            )
            .into(),
        );
    });

    let insert_weak = ui.as_weak();
    ui.on_insert_command(move |cmd| {
        let Some(ui) = insert_weak.upgrade() else {
            return;
        };
        ui.set_command(cmd);
    });

    let help_weak = ui.as_weak();
    ui.on_show_help(move |help| {
        let Some(ui) = help_weak.upgrade() else {
            return;
        };
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
