use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use codewig_core::music::{
    execute_line, key_to_midi, parse_music_line, resolve_track_at, run_chain, MusicLine,
    MusicSession,
};
use codewig_core::{parse_name_eq_value, parse_note_spec, Client, NoteSpec};
use serde_json::Value;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "codewig-cli",
    version,
    about = "Codewig — control Bitwig Studio from the shell",
    long_about = "Talks to the Codewig Bitwig extension over localhost TCP+JSON.\n\
                  Bitwig must be running with Controllers → Codewig enabled."
)]
struct Cli {
    /// Extension host (default 127.0.0.1)
    #[arg(long, env = "CODEWIG_HOST", global = true, default_value = "127.0.0.1")]
    host: String,

    /// Extension port (default 9470)
    #[arg(long, env = "CODEWIG_PORT", global = true, default_value_t = 9470)]
    port: u16,

    /// Connect/read timeout in milliseconds
    #[arg(long, global = true, default_value_t = 2000)]
    timeout_ms: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Health check
    Ping,
    /// Transport + connection status
    Status,
    /// Start playback (alias: start)
    #[command(visible_alias = "start")]
    Play,
    /// Stop playback
    Stop,
    /// Set a global value
    Set {
        #[command(subcommand)]
        target: SetTarget,
    },
    /// Generate shell completion script
    ///
    /// Example: `codewig-cli completions powershell | Out-String | Invoke-Expression`
    Completions {
        /// Shell: bash, zsh, fish, powershell, elvish
        shell: Shell,
    },
    /// Tracks
    Track {
        #[command(subcommand)]
        action: TrackCmd,
    },
    /// Devices on the selected track
    Device {
        #[command(subcommand)]
        action: DeviceCmd,
    },
    /// Direct parameters on the selected device
    Param {
        #[command(subcommand)]
        action: ParamCmd,
    },
    /// Clip Launcher (live/performance)
    Clip {
        #[command(subcommand)]
        action: ClipCmd,
    },
    /// Scenes (launcher rows)
    Scene {
        #[command(subcommand)]
        action: SceneCmd,
    },
    /// One line → track + device chain
    ///
    /// Example: `codewig-cli chain --name bass Polymer Delay+`
    Chain {
        /// instrument | audio | effect (default: instrument)
        #[arg(default_value = "instrument")]
        kind: String,
        /// Track name
        #[arg(long, short)]
        name: Option<String>,
        /// Insert index (0 = top). Default -1 = end
        #[arg(long, default_value = "-1", allow_hyphen_values = true)]
        at: i32,
        /// Devices in order (e.g. Polymer Delay+)
        #[arg(required = true)]
        devices: Vec<String>,
    },
    /// Run one **WIGSCRIPT** line (same language as codewig-live UI).
    ///
    /// Examples:
    ///   codewig-cli eval "mute(kick)"
    ///   codewig-cli eval "s(1).start"
    ///   codewig-cli eval "new track(bass).device(Polymer).add(Delay+)"
    ///   codewig-cli eval "bass: n \"c e g\""
    #[command(visible_alias = "music")]
    Eval {
        /// WIGSCRIPT source (quote it in the shell)
        line: String,
    },
    /// Run lines from a file or stdin. Each line: WIGSCRIPT first, else legacy clap form.
    /// Stops at the first error. `#` starts a comment.
    ///
    /// Example: `codewig-cli batch session.wig`
    Batch {
        /// Command file (default: stdin)
        file: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum SetTarget {
    /// Project tempo in BPM
    Tempo {
        /// Tempo in beats per minute
        bpm: f64,
    },
}

#[derive(Subcommand, Debug)]
enum TrackCmd {
    /// Create a track (default type: instrument)
    New {
        /// Track type: instrument | audio | effect (default: instrument)
        #[arg(default_value = "instrument")]
        kind: String,
        /// Track name
        #[arg(long, short)]
        name: Option<String>,
        /// Insert index (0 = top, -1 = end)
        #[arg(long, default_value = "-1", allow_hyphen_values = true)]
        at: i32,
    },
    List,
    /// Select a track
    Select {
        /// Track name or index
        r#ref: String,
    },
    /// Delete a track
    Delete {
        /// Track name or index
        r#ref: String,
    },
    /// Rename a track
    Rename {
        /// Track name or index
        r#ref: String,
        /// New track name
        name: String,
    },
    /// Move a track
    Move {
        /// Track name or index
        r#ref: String,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        after: Option<String>,
        #[arg(long)]
        to: Option<i32>,
    },
    /// Mute tracks: `track mute 1 3 6` or `track mute bass lead`
    /// Fluent later: track.mute(1,3,6)
    Mute {
        /// Track indices and/or names
        #[arg(required = true)]
        refs: Vec<String>,
        /// Unmute instead
        #[arg(long)]
        off: bool,
    },
    /// Solo tracks: `track solo 1 3 6`
    Solo {
        /// Track indices and/or names
        #[arg(required = true)]
        refs: Vec<String>,
        /// Unsolo instead
        #[arg(long)]
        off: bool,
    },
    /// Volume 0..1 on one track
    Volume {
        /// Track name or index
        r#ref: String,
        /// Volume 0..1
        value: f64,
    },
}

#[derive(Subcommand, Debug)]
enum DeviceCmd {
    /// Add a device to the selected track
    Add {
        /// Device name (e.g. Polymer, Delay+)
        name: String,
    },
    List,
    /// Select a device by index
    Select {
        /// Device index
        index: i32,
    },
    /// Delete a device by index
    Delete {
        /// Device index
        index: i32,
    },
    /// Enable a device by index
    On {
        /// Device index
        index: i32,
    },
    /// Disable a device by index
    Off {
        /// Device index
        index: i32,
    },
    /// Move a device to a chain position
    Move {
        /// Device index
        index: i32,
        /// Target position (0-based)
        to: i32,
    },
}

#[derive(Subcommand, Debug)]
enum SceneCmd {
    /// List scenes (launcher rows)
    List,
    /// Claim / name a scene row (idempotent if name exists)
    New {
        /// Scene name
        name: Option<String>,
    },
    /// Launch a scene
    Launch {
        /// Scene index or name
        r#ref: String,
    },
    /// Stop clip launcher playback (all tracks)
    Stop {
        /// Scene index or name
        r#ref: String,
    },
    /// Rename a scene
    Rename {
        /// Scene index or current name
        r#ref: String,
        /// New scene name
        name: String,
    },
    /// Delete a scene incl. its clips
    Delete {
        /// Scene index or name
        r#ref: String,
    },
}

#[derive(Subcommand, Debug)]
enum ParamCmd {
    /// List parameters (`--source remote` = Remote Controls only)
    List {
        /// direct | remote | all  (default: direct)
        #[arg(long, default_value = "direct")]
        source: String,
    },
    /// Set one or more direct parameters
    Set {
        /// Parameter name
        #[arg(long, short = 'n')]
        name: Option<String>,
        /// Parameter id
        #[arg(long)]
        id: Option<String>,
        /// Value 0..1
        #[arg(long, short = 'v')]
        value: Option<f64>,
        /// Set multiple params as name=value
        #[arg(long = "set", value_name = "NAME=VALUE")]
        sets: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ClipCmd {
    /// Create empty launcher clip (first empty slot if --slot omitted)
    New {
        /// Track name or index
        track: String,
        /// Slot 0..15 (default: first empty)
        #[arg(long, short = 's')]
        slot: Option<i32>,
        /// Length in beats (default 4)
        #[arg(long, short = 'b', default_value_t = 4)]
        beats: i32,
        /// Clip name
        #[arg(long, short = 'n')]
        name: Option<String>,
    },
    /// List clip slots on a track
    List {
        /// Track name or index
        track: String,
    },
    /// Launch clip slot (switch clips live)
    Launch {
        /// Track name or index
        track: String,
        /// Slot 0..15
        slot: i32,
    },
    /// Stop clip launcher on track
    Stop {
        /// Track name or index
        track: String,
    },
    /// Rename a clip slot
    Rename {
        /// Track name or index
        track: String,
        /// Slot 0..15
        slot: i32,
        /// New clip name
        name: String,
    },
    /// Delete the clip in a slot
    Delete {
        /// Track name or index
        track: String,
        /// Slot 0..15
        slot: i32,
    },
    /// Write notes into a clip: step:key[:vel[:dur]] (key = MIDI or name, C3 = 60)
    ///
    /// Example: `codewig-cli clip note bass 0 0:C3:100:1 4:E3 8:G3`
    Note {
        /// Track name or index
        track: String,
        /// Slot 0..15
        slot: i32,
        /// Notes as step:key[:vel[:dur]] (vel 1..127 = 100, dur in 16th steps = 1)
        #[arg(required = true)]
        notes: Vec<String>,
    },
    /// Clear notes: whole clip, or one cell with --step + --key
    #[command(name = "clear-notes")]
    ClearNotes {
        /// Track name or index
        track: String,
        /// Slot 0..15
        slot: i32,
        /// Step to clear (requires --key)
        #[arg(long)]
        step: Option<i32>,
        /// MIDI number or note name (C3 = 60)
        #[arg(long)]
        key: Option<String>,
    },
}

fn main() -> ExitCode {
    // Create + seed the per-user layout once at startup (devices/*.yaml home).
    if let Err(e) = codewig_core::ensure_user_layout() {
        eprintln!("user layout: {e}");
    }
    let cli = Cli::parse();
    match run_with(&cli.host, cli.port, cli.timeout_ms, cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_with(
    host: &str,
    port: u16,
    timeout_ms: u64,
    command: Commands,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Commands::Completions { shell } = command {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "codewig-cli", &mut std::io::stdout());
        return Ok(());
    }

    if let Commands::Batch { file } = command {
        return run_batch(host, port, timeout_ms, file);
    }

    let client = Client::new(host, port, timeout_ms);
    let mut session = MusicSession::default();

    let result: Option<Value> = match command {
        Commands::Eval { line } => run_one_line(&client, &mut session, &line)?,
        Commands::Chain {
            kind,
            name,
            at,
            devices,
        } => Some(
            run_chain(&client, &kind, name.as_deref(), at, &devices)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
        ),
        other => dispatch(&client, other)?,
    };

    match result {
        Some(value) => println!("{}", serde_json::to_string_pretty(&value)?),
        None => println!("ok"),
    }
    Ok(())
}

/// Send a single command over the shared client via typed `Client` methods and
/// return the extension's result value, or `None` when the command only returns `{ok:true}`.
fn dispatch(client: &Client, command: Commands) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let result = match command {
        Commands::Chain { .. }
        | Commands::Batch { .. }
        | Commands::Eval { .. }
        | Commands::Completions { .. } => {
            unreachable!("dispatch does not handle chain, batch, eval or completions")
        }
        Commands::Ping => client.ping()?,
        Commands::Status => client.status()?,
        Commands::Play => client.play()?,
        Commands::Stop => client.stop()?,
        Commands::Set {
            target: SetTarget::Tempo { bpm },
        } => client.set_tempo(bpm)?,
        Commands::Track { action } => match action {
            TrackCmd::New { kind, name, at } => {
                let at = resolve_track_at(client, at)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                client.track_new(&kind, at, name.as_deref())?
            }
            TrackCmd::List => client.track_list()?,
            TrackCmd::Select { r#ref } => client.track_select(&r#ref)?,
            TrackCmd::Delete { r#ref } => client.track_delete(&r#ref)?,
            TrackCmd::Rename { r#ref, name } => client.track_rename(&r#ref, &name)?,
            TrackCmd::Move {
                r#ref,
                before,
                after,
                to,
            } => client.track_move(&r#ref, before.as_deref(), after.as_deref(), to)?,
            TrackCmd::Mute { refs, off } => client.track_mute(&refs, !off)?,
            TrackCmd::Solo { refs, off } => client.track_solo(&refs, !off)?,
            TrackCmd::Volume { r#ref, value } => client.track_volume(&r#ref, value)?,
        },
        Commands::Device { action } => match action {
            DeviceCmd::Add { name } => client.device_add(&name)?,
            DeviceCmd::List => client.device_list()?,
            DeviceCmd::Select { index } => client.device_select(index)?,
            DeviceCmd::Delete { index } => client.device_delete(index)?,
            DeviceCmd::On { index } => client.device_enable(index, true)?,
            DeviceCmd::Off { index } => client.device_enable(index, false)?,
            DeviceCmd::Move { index, to } => client.device_move(index, to)?,
        },
        Commands::Param { action } => match action {
            ParamCmd::List { source } => client.param_list_source(&source)?,
            ParamCmd::Set {
                name,
                id,
                value,
                sets,
            } => {
                if !sets.is_empty() {
                    let parsed: Result<Vec<(String, f64)>, _> =
                        sets.iter().map(|s| parse_name_eq_value(s)).collect();
                    client.param_set_multi(&parsed?)?
                } else {
                    let v = value.ok_or("param set needs --value or --set name=value")?;
                    if let Some(n) = name {
                        client.param_set_name_value(&n, v)?
                    } else if let Some(i) = id {
                        client.param_set_id_value(&i, v)?
                    } else {
                        return Err("param set needs --name or --id (or --set pairs)".into());
                    }
                }
            }
        },
        Commands::Clip { action } => match action {
            ClipCmd::New {
                track,
                slot,
                beats,
                name,
            } => client.clip_new(&track, slot, beats, name.as_deref())?,
            ClipCmd::List { track } => client.clip_list(&track)?,
            ClipCmd::Launch { track, slot } => client.clip_launch(&track, slot)?,
            ClipCmd::Stop { track } => client.clip_stop(&track)?,
            ClipCmd::Rename {
                track,
                slot,
                name,
            } => client.clip_rename(&track, slot, &name)?,
            ClipCmd::Delete { track, slot } => client.clip_delete(&track, slot)?,
            ClipCmd::Note {
                track,
                slot,
                notes,
            } => {
                // replace = clear + write one RPC (live pattern rewrite)
                let parsed: Result<Vec<NoteSpec>, _> =
                    notes.iter().map(|s| parse_note_spec(s)).collect();
                client.clip_replace_notes(&track, slot, &parsed?)?
            }
            ClipCmd::ClearNotes {
                track,
                slot,
                step,
                key,
            } => {
                let key = key.map(|k| key_to_midi(&k)).transpose()?;
                client.clip_clear_notes(&track, slot, step, key)?
            }
        },
        Commands::Scene { action } => match action {
            SceneCmd::List => client.scene_list()?,
            SceneCmd::New { name } => client.scene_new(name.as_deref())?,
            SceneCmd::Launch { r#ref } => client.scene_launch(&r#ref)?,
            SceneCmd::Stop { r#ref } => client.scene_stop(&r#ref)?,
            SceneCmd::Rename { r#ref, name } => client.scene_rename(&r#ref, &name)?,
            SceneCmd::Delete { r#ref } => client.scene_delete(&r#ref)?,
        },
    };
    Ok(result)
}

/// One line: WIGSCRIPT if it parses, else legacy `codewig-cli …` clap form (without binary name).
fn run_one_line(
    client: &Client,
    session: &mut MusicSession,
    trimmed: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match parse_music_line(trimmed) {
        Ok(MusicLine::Empty) => Ok(None),
        Ok(MusicLine::PassThrough(cmd)) => {
            // `> track list` → legacy clap line
            run_legacy_line(client, &cmd)
        }
        Ok(line) => execute_line(client, session, line).map_err(|e| e.into()),
        Err(_) => run_legacy_line(client, trimmed),
    }
}

fn run_legacy_line(
    client: &Client,
    trimmed: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let words = shlex::split(trimmed).ok_or("unmatched quote")?;
    let args = std::iter::once("codewig-cli".to_string()).chain(words);
    let inner = Cli::try_parse_from(args)?;
    if matches!(
        inner.command,
        Commands::Batch { .. } | Commands::Eval { .. } | Commands::Completions { .. }
    ) {
        return Err("nested batch/eval/completions not allowed here".into());
    }
    if let Commands::Chain {
        kind,
        name,
        at,
        devices,
    } = inner.command
    {
        Ok(Some(
            run_chain(client, &kind, name.as_deref(), at, &devices)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
        ))
    } else {
        Ok(dispatch(client, inner.command)?)
    }
}

fn run_batch(
    host: &str,
    port: u16,
    timeout_ms: u64,
    file: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{BufRead, BufReader};

    let client = Client::new(host, port, timeout_ms);
    let mut session = MusicSession::default();
    let input: Box<dyn BufRead> = match file {
        Some(path) => Box::new(BufReader::new(std::fs::File::open(path)?)),
        None => Box::new(BufReader::new(std::io::stdin())),
    };

    for (line_no, line) in input.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let result = run_one_line(&client, &mut session, trimmed)
            .map_err(|e| format!("line {}: {e}", line_no + 1))?;

        match result {
            Some(value) => println!("{}", serde_json::to_string(&value)?),
            None => println!("ok"),
        }
    }
    Ok(())
}
