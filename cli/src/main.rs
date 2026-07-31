use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use cliwig_core::music::{execute_line, parse_music_line, MusicLine, MusicSession};
use cliwig_core::Client;
use serde_json::{json, Map, Value};
use std::process::ExitCode;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "cliwig",
    version,
    about = "CLIwig — control Bitwig Studio from the shell",
    long_about = "Talks to the CLIwig Bitwig extension over localhost TCP+JSON.\n\
                  Bitwig must be running with Controllers → CLIwig enabled."
)]
struct Cli {
    /// Extension host (default 127.0.0.1)
    #[arg(long, env = "CLIWIG_HOST", global = true, default_value = "127.0.0.1")]
    host: String,

    /// Extension port (default 9470)
    #[arg(long, env = "CLIWIG_PORT", global = true, default_value_t = 9470)]
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
    /// Example: `cliwig completions powershell | Out-String | Invoke-Expression`
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
    /// One line → track + device chain
    ///
    /// Example: `cliwig chain --name bass Polymer Delay+`
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
    ///   cliwig eval "mute(kick)"
    ///   cliwig eval "s(1).start"
    ///   cliwig eval "new track(bass).device(Polymer).add(Delay+)"
    ///   cliwig eval "bass: n \"c e g\""
    #[command(visible_alias = "music")]
    Eval {
        /// WIGSCRIPT source (quote it in the shell)
        line: String,
    },
    /// Run lines from a file or stdin. Each line: WIGSCRIPT first, else legacy clap form.
    /// Stops at the first error. `#` starts a comment.
    ///
    /// Example: `cliwig batch session.wig`
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
}

#[derive(Subcommand, Debug)]
enum ParamCmd {
    List,
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
    /// Write notes into a clip: step:key[:vel[:dur]] (key = MIDI or name, C3 = 60)
    ///
    /// Example: `cliwig clip note bass 0 0:C3:100:1 4:E3 8:G3`
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
        clap_complete::generate(shell, &mut cmd, "cliwig", &mut std::io::stdout());
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
        } => Some(run_chain(&client, kind, name, at, devices)?),
        other => dispatch(&client, other)?,
    };

    match result {
        Some(value) => println!("{}", serde_json::to_string_pretty(&value)?),
        None => println!("ok"),
    }
    Ok(())
}

/// Resolve the insert index for a new track. When no explicit index was given
/// (`at == -1`), query `track.list` and count only instrument/audio tracks so
/// the new track lands on the next free slot. Effect and master tracks are
/// ignored for the count, matching what Bitwig users consider "real" tracks.
fn resolve_track_at(client: &Client, at: i32) -> Result<i32, Box<dyn std::error::Error>> {
    if at >= 0 {
        return Ok(at);
    }

    let list = client
        .track_list()?
        .ok_or("track.list returned no data")?;
    let tracks = list
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or("track.list response missing 'tracks' array")?;

    let count = tracks
        .iter()
        .filter(|t| {
            t.get("type")
                .and_then(Value::as_str)
                .map(|ty| {
                    let ty = ty.to_lowercase();
                    ty == "instrument" || ty == "audio"
                })
                .unwrap_or(false)
        })
        .count();

    Ok(count as i32)
}

/// Send a single command over the shared client and return the
/// extension's result value, or `None` when the command only returns `{ok:true}`.
fn dispatch(client: &Client, command: Commands) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let (c, fields): CmdSpec = match command {
        Commands::Chain { .. }
        | Commands::Batch { .. }
        | Commands::Eval { .. }
        | Commands::Completions { .. } => {
            unreachable!("dispatch does not handle chain, batch, eval or completions")
        }
        Commands::Ping => ("ping", Map::new()),
        Commands::Status => ("status", Map::new()),
        Commands::Play => ("play", Map::new()),
        Commands::Stop => ("stop", Map::new()),
        Commands::Set {
            target: SetTarget::Tempo { bpm },
        } => {
            let mut m = Map::new();
            m.insert("k".into(), "tempo".into());
            m.insert("v".into(), bpm.into());
            ("set", m)
        }
        Commands::Track { action } => match action {
            TrackCmd::New { kind, name, at } => {
                let at = resolve_track_at(client, at)?;
                let mut m = Map::new();
                m.insert("type".into(), kind.into());
                m.insert("at".into(), at.into());
                if let Some(n) = name {
                    m.insert("name".into(), n.into());
                }
                ("track.new", m)
            }
            TrackCmd::List => ("track.list", Map::new()),
            TrackCmd::Select { r#ref } => {
                let mut m = Map::new();
                m.insert("ref".into(), r#ref.into());
                ("track.select", m)
            }
            TrackCmd::Delete { r#ref } => {
                let mut m = Map::new();
                m.insert("ref".into(), r#ref.into());
                ("track.delete", m)
            }
            TrackCmd::Rename { r#ref, name } => {
                let mut m = Map::new();
                m.insert("ref".into(), r#ref.into());
                m.insert("name".into(), name.into());
                ("track.rename", m)
            }
            TrackCmd::Move {
                r#ref,
                before,
                after,
                to,
            } => {
                let mut m = Map::new();
                m.insert("ref".into(), r#ref.into());
                if let Some(b) = before {
                    m.insert("before".into(), b.into());
                }
                if let Some(a) = after {
                    m.insert("after".into(), a.into());
                }
                if let Some(t) = to {
                    m.insert("to".into(), t.into());
                }
                ("track.move", m)
            }
            TrackCmd::Mute { refs, off } => ("track.mute", refs_fields(refs, !off)),
            TrackCmd::Solo { refs, off } => ("track.solo", refs_fields(refs, !off)),
            TrackCmd::Volume { r#ref, value } => {
                let mut m = Map::new();
                m.insert("ref".into(), r#ref.into());
                m.insert("v".into(), value.into());
                ("track.volume", m)
            }
        },
        Commands::Device { action } => match action {
            DeviceCmd::Add { name } => {
                let mut m = Map::new();
                m.insert("name".into(), name.into());
                ("device.add", m)
            }
            DeviceCmd::List => ("device.list", Map::new()),
            DeviceCmd::Select { index } => {
                let mut m = Map::new();
                m.insert("index".into(), index.into());
                ("device.select", m)
            }
            DeviceCmd::Delete { index } => {
                let mut m = Map::new();
                m.insert("index".into(), index.into());
                ("device.delete", m)
            }
        },
        Commands::Param { action } => match action {
            ParamCmd::List => ("param.list", Map::new()),
            ParamCmd::Set {
                name,
                id,
                value,
                sets,
            } => build_param_set(name, id, value, sets)?,
        },
        Commands::Clip { action } => match action {
            ClipCmd::New {
                track,
                slot,
                beats,
                name,
            } => {
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                m.insert("beats".into(), beats.into());
                m.insert("slot".into(), slot.unwrap_or(-1).into());
                if let Some(n) = name {
                    m.insert("name".into(), n.into());
                }
                ("clip.new", m)
            }
            ClipCmd::List { track } => {
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                ("clip.list", m)
            }
            ClipCmd::Launch { track, slot } => {
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                m.insert("slot".into(), slot.into());
                ("clip.launch", m)
            }
            ClipCmd::Stop { track } => {
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                ("clip.stop", m)
            }
            ClipCmd::Note {
                track,
                slot,
                notes,
            } => {
                let parsed: Result<Vec<Value>, _> = notes.iter().map(|s| parse_note(s)).collect();
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                m.insert("slot".into(), slot.into());
                m.insert("notes".into(), Value::Array(parsed?));
                ("clip.set-notes", m)
            }
            ClipCmd::ClearNotes {
                track,
                slot,
                step,
                key,
            } => {
                let mut m = Map::new();
                m.insert("track".into(), track.into());
                m.insert("slot".into(), slot.into());
                if let (Some(st), Some(k)) = (step, key) {
                    m.insert("step".into(), st.into());
                    m.insert("key".into(), parse_key(&k)?.into());
                }
                ("clip.clear-notes", m)
            }
        },
    };

    Ok(client.send_raw(c, fields)?)
}

/// One line: WIGSCRIPT if it parses, else legacy `cliwig …` clap form (without binary name).
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
    let args = std::iter::once("cliwig".to_string()).chain(words);
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
        Ok(Some(run_chain(client, kind, name, at, devices)?))
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

fn refs_fields(refs: Vec<String>, on: bool) -> Map<String, Value> {
    let arr: Vec<Value> = refs.into_iter().map(Value::String).collect();
    let mut m = Map::new();
    m.insert("refs".into(), Value::Array(arr));
    m.insert("on".into(), Value::Bool(on));
    m
}

fn run_chain(
    client: &Client,
    kind: String,
    name: Option<String>,
    at: i32,
    devices: Vec<String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    if devices.is_empty() {
        return Err("chain needs at least one device (e.g. Polymer Delay+)".into());
    }

    let at = resolve_track_at(client, at)?;
    let created = client.track_new(&kind, at, name.as_deref())?.unwrap_or(Value::Bool(true));
    eprintln!("track: {}", serde_json::to_string(&created)?);

    std::thread::sleep(Duration::from_millis(120));

    if let Some(ref n) = name {
        let sel = client.track_select(n)?.unwrap_or(Value::Bool(true));
        eprintln!("select: {}", serde_json::to_string(&sel)?);
        std::thread::sleep(Duration::from_millis(40));
    }

    let mut added = Vec::new();
    for dev in &devices {
        let r = client.device_add(dev)?.unwrap_or(Value::Bool(true));
        added.push(json!({ "device": dev, "result": r }));
        std::thread::sleep(Duration::from_millis(40));
    }

    // Optional first empty clip for live switching
    if let Some(ref n) = name {
        match client.clip_new(n, None, 4, Some("A")) {
            Ok(Some(clip)) => eprintln!("clip: {}", serde_json::to_string(&clip)?),
            Ok(None) => eprintln!("clip: ok"),
            Err(e) => eprintln!("clip note: {e} (create manually with: cliwig clip new {n})"),
        }
    }

    let summary = json!({
        "chain": {
            "track_type": kind,
            "name": name,
            "at": at,
            "devices": devices,
        },
        "added": added,
        "next": [
            "clip new <track> --name B   # more slots for live switch",
            "clip launch <track> 0",
            "param list / param set",
            "track mute 1 3 6 / track solo 0 2",
        ]
    });
    Ok(summary)
}

type CmdSpec = (&'static str, Map<String, Value>);

fn build_param_set(
    name: Option<String>,
    id: Option<String>,
    value: Option<f64>,
    sets: Vec<String>,
) -> Result<CmdSpec, Box<dyn std::error::Error>> {
    if !sets.is_empty() {
        let mut arr = Vec::new();
        for s in sets {
            let (n, v) = parse_name_eq_value(&s)?;
            arr.push(json!({ "name": n, "v": v }));
        }
        let mut m = Map::new();
        m.insert("sets".into(), Value::Array(arr));
        return Ok(("param.set", m));
    }

    let v = value.ok_or("param set needs --value or --set name=value")?;
    let mut m = Map::new();
    m.insert("v".into(), v.into());
    if let Some(n) = name {
        m.insert("name".into(), n.into());
    } else if let Some(i) = id {
        m.insert("id".into(), i.into());
    } else {
        return Err("param set needs --name or --id (or --set pairs)".into());
    }
    Ok(("param.set", m))
}

fn parse_name_eq_value(s: &str) -> Result<(String, f64), Box<dyn std::error::Error>> {
    let (n, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected name=value, got '{s}'"))?;
    let val: f64 = v.parse().map_err(|_| format!("bad value in '{s}'"))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!("value must be 0..1, got {val}").into());
    }
    Ok((n.trim().to_string(), val))
}

/// Parse `step:key[:vel[:dur]]` into a note JSON object for `clip.set-notes`.
/// key: MIDI number or note name (C3 = 60). vel default 100, dur default 1.0 (16th steps).
fn parse_note(s: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return Err(format!("expected step:key[:vel[:dur]], got '{s}'").into());
    }
    let step: i32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| format!("bad step in '{s}'"))?;
    if step < 0 {
        return Err(format!("step must be >= 0, got {step}").into());
    }
    let key = parse_key(parts[1])?;
    let vel: i32 = match parts.get(2) {
        Some(v) => {
            let v: i32 = v.trim().parse().map_err(|_| format!("bad vel in '{s}'"))?;
            if !(1..=127).contains(&v) {
                return Err(format!("vel must be 1..127, got {v}").into());
            }
            v
        }
        None => 100,
    };
    let dur: f64 = match parts.get(3) {
        Some(d) => {
            let d: f64 = d.trim().parse().map_err(|_| format!("bad dur in '{s}'"))?;
            if d <= 0.0 {
                return Err(format!("dur must be > 0, got {d}").into());
            }
            d
        }
        None => 1.0,
    };
    Ok(json!({ "step": step, "key": key, "vel": vel, "dur": dur }))
}

/// MIDI number (0..127) or note name with Bitwig convention (C3 = 60).
fn parse_key(s: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i32>() {
        if !(0..=127).contains(&n) {
            return Err(format!("key must be 0..127, got {n}").into());
        }
        return Ok(n);
    }
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err("empty key".into());
    }
    let base = match bytes[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return Err(format!("bad note name '{s}'").into()),
    };
    let mut idx = 1;
    let mut semitone = base;
    if idx < bytes.len() && bytes[idx] == b'#' {
        semitone += 1;
        idx += 1;
    } else if idx < bytes.len() && (bytes[idx] == b'b' || bytes[idx] == b'B') {
        semitone -= 1;
        idx += 1;
    }
    let octave_str = &s[idx..];
    if octave_str.is_empty() {
        return Err(format!("missing octave in '{s}'").into());
    }
    let octave: i32 = octave_str
        .parse()
        .map_err(|_| format!("bad octave in '{s}'"))?;
    let midi = (octave + 2) * 12 + semitone;
    if !(0..=127).contains(&midi) {
        return Err(format!("note '{s}' out of MIDI range ({midi})").into());
    }
    Ok(midi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_numbers() {
        assert_eq!(parse_key("60").unwrap(), 60);
        assert_eq!(parse_key("0").unwrap(), 0);
        assert_eq!(parse_key("127").unwrap(), 127);
        assert!(parse_key("128").is_err());
        assert!(parse_key("-1").is_err());
    }

    #[test]
    fn parse_key_names() {
        assert_eq!(parse_key("C3").unwrap(), 60); // Bitwig middle C
        assert_eq!(parse_key("C4").unwrap(), 72);
        assert_eq!(parse_key("A3").unwrap(), 69);
        assert_eq!(parse_key("F#3").unwrap(), 66);
        assert_eq!(parse_key("Bb2").unwrap(), 58);
        assert_eq!(parse_key("c3").unwrap(), 60); // case-insensitive
        assert_eq!(parse_key("C-2").unwrap(), 0);
        assert_eq!(parse_key("G8").unwrap(), 127);
        assert!(parse_key("C").is_err());
        assert!(parse_key("H3").is_err());
        assert!(parse_key("G#8").is_err());
    }

    #[test]
    fn parse_note_full() {
        let n = parse_note("0:C3:100:1").unwrap();
        assert_eq!(n, json!({"step":0,"key":60,"vel":100,"dur":1.0}));
    }

    #[test]
    fn parse_note_defaults() {
        let n = parse_note("4:E3").unwrap();
        assert_eq!(n, json!({"step":4,"key":64,"vel":100,"dur":1.0}));
    }

    #[test]
    fn parse_note_errors() {
        assert!(parse_note("0").is_err());
        assert!(parse_note("-1:C3").is_err());
        assert!(parse_note("0:C3:0").is_err()); // vel 0
        assert!(parse_note("0:C3:100:0").is_err()); // dur 0
        assert!(parse_note("0:C3:100:1:extra").is_err());
    }
}
