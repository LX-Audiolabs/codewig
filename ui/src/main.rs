use codewig_core::music::param_catalog::{catalog, DeviceHostKind};
use codewig_core::music::MusicSession;
use codewig_core::Client;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

mod commands;

slint::include_modules!();

/// Status probe only — short so offline Bitwig does not freeze the UI (commands keep 2s).
const STATUS_TIMEOUT_MS: u64 = 250;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One client for whole UI lifetime — persistent TCP inside Client.
    let client = Rc::new(Client::default());
    // WIGSCRIPT session (key/scale) shared across sends — not transport.
    let session = Rc::new(RefCell::new(MusicSession::default()));

    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Load param catalog (devices with a YAML definition) into the Devices tab.
    let catalog = catalog();
    let devices: Vec<DeviceEntry> = catalog
        .devices()
        .iter()
        .map(|d| {
            let aliases = if d.aliases.is_empty() {
                String::new()
            } else {
                d.aliases.join(", ")
            };
            let mut detail = format!(
                "{}\nkind: {}\nsource: {}\n\nParameters:\n",
                d.bitwig_name,
                match d.kind {
                    DeviceHostKind::Bitwig => "bitwig",
                    DeviceHostKind::Clap => "clap",
                },
                d.source
            );
            if d.params.is_empty() {
                detail.push_str("  (none documented yet)\n");
            } else {
                for p in &d.params {
                    let aliases = if p.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({}", p.aliases.join(", "))
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
            DeviceEntry {
                name: d.bitwig_name.clone().into(),
                aliases: aliases.into(),
                syntax: format!(".device({})", d.bitwig_name).into(),
                detail: detail.into(),
            }
        })
        .collect();
    ui.set_devices(slint::ModelRc::new(slint::VecModel::from(devices)));

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

    // Sidebar filter: substring match (Slint has no string.contains)
    ui.on_text_matches(|haystack, needle| {
        let n = needle.trim().to_lowercase();
        if n.is_empty() {
            return true;
        }
        haystack.to_lowercase().contains(&n)
    });

    let reconnect_weak = ui.as_weak();
    let client_reconnect = client.clone();
    ui.on_reconnect(move || {
        let ui = reconnect_weak.upgrade().unwrap();
        client_reconnect.reset();
        // Short probe on UI thread — max ~STATUS_TIMEOUT_MS, not 2s
        ui.set_status(connection_status().into());
    });

    let insert_weak = ui.as_weak();
    ui.on_insert_command(move |cmd| {
        let ui = insert_weak.upgrade().unwrap();
        ui.set_command(cmd);
    });

    let help_weak = ui.as_weak();
    ui.on_show_help(move |help| {
        let ui = help_weak.upgrade().unwrap();
        ui.set_help_text(help);
    });

    let session_send = session.clone();
    let client_send = client.clone();
    ui.on_send_command(move || {
        let ui = ui_weak.upgrade().unwrap();
        let cmd = ui.get_command().to_string();
        let mut sess = session_send.borrow_mut();

        let result = commands::run(&client_send, &mut sess, &cmd);

        let mut output = ui.get_output().to_string();
        output.push_str(&format!("♫ {}\n", cmd));
        match &result {
            Ok(Some(v)) => {
                output.push_str(&format!("{}\n", serde_json::to_string_pretty(v).unwrap()));
                // RPC succeeded → socket live; skip extra ping RTT
                ui.set_status("connected".into());
            }
            Ok(None) => {
                output.push_str("ok\n");
                ui.set_status("connected".into());
            }
            Err(e) => {
                output.push_str(&format!("error: {}\n", e));
                // Re-check only on failure (short probe — do not block UI 2s)
                ui.set_status(connection_status().into());
            }
        }
        ui.set_output(output.into());
        ui.set_command("".into());
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
