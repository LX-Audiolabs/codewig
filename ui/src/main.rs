use cliwig_core::music::MusicSession;
use cliwig_core::Client;
use std::cell::RefCell;
use std::rc::Rc;

mod commands;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One client for whole UI lifetime — persistent TCP inside Client.
    let client = Rc::new(Client::default());
    // WIGSCRIPT session (key/scale) shared across sends — not transport.
    let session = Rc::new(RefCell::new(MusicSession::default()));

    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    ui.set_status(connection_status(&client).into());

    let reconnect_weak = ui.as_weak();
    let client_reconnect = client.clone();
    ui.on_reconnect(move || {
        let ui = reconnect_weak.upgrade().unwrap();
        client_reconnect.reset();
        ui.set_status(connection_status(&client_reconnect).into());
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
                output.push_str(&format!(
                    "{}\n",
                    serde_json::to_string_pretty(v).unwrap()
                ));
                // RPC succeeded → socket live; skip extra ping RTT
                ui.set_status("connected".into());
            }
            Ok(None) => {
                output.push_str("ok\n");
                ui.set_status("connected".into());
            }
            Err(e) => {
                output.push_str(&format!("error: {}\n", e));
                // Re-check only on failure (may be disconnect)
                ui.set_status(connection_status(&client_send).into());
            }
        }
        ui.set_output(output.into());
        ui.set_command("".into());
    });

    ui.run()?;
    Ok(())
}

fn connection_status(client: &Client) -> String {
    match client.ping() {
        Ok(_) => "connected".to_string(),
        Err(e) => format!("disconnected ({})", e),
    }
}
