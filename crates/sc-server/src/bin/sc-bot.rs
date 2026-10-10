//! `sc-bot`: a headless bot crew member (openspec/changes/coop-drill design 3 and 6). It joins a drill server on
//! the LAN, takes a station and plays it with `sc-core`'s automation at the bot profile, printing a line for every
//! phase and a summary each second it is engaged. The game client's `--bot` is the same bot with a screen.
//!
//! A lost connection is retried every `RETRY_S` seconds, as the game client does, so a bot left running rides out a
//! server restart or the Wi-Fi dropping (coop-drill log, 2026-10-10: Pi 1's Wi-Fi went for two minutes mid-drill).
//!
//! Usage: sc-bot --connect HOST[:7700] --station helm|tactical|engineering|science|captain [--name N] [--root .] [--seconds S]

use sc_core::combat::data::{DrillData, DRILL_FILES};
use sc_core::combat::Station;
use sc_net::bot::Bot;
use sc_net::session::{Session, Stage};
use sc_net::transport::Impair;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Seconds between tries to reach the server, and the longest a try may take to join before it is given up.
const RETRY_S: f64 = 3.0;
const JOIN_S: f64 = 15.0;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let Some(host) = value("--connect") else {
        eprintln!("usage: sc-bot --connect HOST[:7700] --station helm|tactical|engineering|science|captain [--name N] [--root .] [--seconds S]");
        return ExitCode::from(2);
    };
    let host = if host.contains(':') { host } else { format!("{host}:7700") };
    let Some(addr) = host.to_socket_addrs().ok().and_then(|mut a| a.find(SocketAddr::is_ipv4)) else {
        eprintln!("sc-bot: cannot resolve {host}");
        return ExitCode::from(2);
    };
    let Some(station) = value("--station").and_then(|s| Station::from_id(&s)) else {
        eprintln!("sc-bot: --station helm, tactical, engineering, science or captain");
        return ExitCode::from(2);
    };
    let name = value("--name").unwrap_or_else(|| format!("{} bot", station.name()));
    let root = PathBuf::from(value("--root").unwrap_or_else(|| ".".into()));
    let seconds = value("--seconds").and_then(|v| v.parse::<f64>().ok());
    let read = |rel: &str| std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"));
    let data = (|| {
        let t: Vec<String> = DRILL_FILES.iter().map(|f| read(f)).collect::<Result<_, _>>()?;
        let arr: [&str; 6] = std::array::from_fn(|i| t[i].as_str());
        let m = "data/missions/drill-hound.json";
        DrillData::parse(arr, m, &read(m)?).map_err(|e| e.to_string())
    })();
    let data = match data {
        Ok(d) => d,
        Err(e) => {
            eprintln!("sc-bot: {e}");
            return ExitCode::FAILURE;
        }
    };
    let start = Instant::now();
    let connect = || -> Option<Session> {
        match Session::connect(addr, &name, true, Some(station), Impair::default(), start.elapsed().as_nanos() as u64) {
            Ok(s) => {
                println!(
                    "[{:7.1} s] sc-bot: {name} connecting to {addr} for {}",
                    start.elapsed().as_secs_f64(),
                    station.name()
                );
                Some(s)
            }
            Err(e) => {
                eprintln!("[{:7.1} s] sc-bot: {e}; trying again in {RETRY_S:.0} s", start.elapsed().as_secs_f64());
                None
            }
        }
    };
    let mut session = connect();
    let mut tried = start.elapsed().as_secs_f64();
    let mut bot = Bot::new(&data, station);
    let (mut last_phase, mut last_print, mut last_stage) = (None, 0.0, None);
    loop {
        let now = start.elapsed().as_secs_f64();
        if seconds.is_some_and(|x| now >= x) {
            return ExitCode::SUCCESS;
        }
        // No connection, a closed one, or one that never joined: try again, as a fresh crew member.
        let lost = match &session {
            None => true,
            Some(s) => {
                s.stage == Stage::Closed
                    || (s.stage != Stage::Joined && now - tried >= JOIN_S)
                    || (s.stage == Stage::Joined && s.silent_s() >= sc_server::SILENT_S)
            }
        };
        if lost {
            if now - tried >= RETRY_S || session.is_some() {
                if session.take().is_some() {
                    println!("[{now:7.1} s] lost the server; trying again every {RETRY_S:.0} s");
                    tried = now;
                    last_stage = None;
                    continue;
                }
                session = connect();
                tried = now;
                bot = Bot::new(&data, station);
                last_phase = None;
            }
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let Some(s) = session.as_mut() else { continue };
        s.update();
        bot.drive(s, &data);
        if Some(s.stage) != last_stage {
            last_stage = Some(s.stage);
            println!("[{:7.1} s] {:?}", start.elapsed().as_secs_f64(), s.stage);
        }
        if let Some(snap) = s.latest() {
            if last_phase != Some(snap.phase) {
                last_phase = Some(snap.phase);
                println!(
                    "[{:7.1} s] round {} {:?} {:?}",
                    start.elapsed().as_secs_f64(),
                    snap.round,
                    snap.phase,
                    snap.outcome
                );
            }
            let t = start.elapsed().as_secs_f64();
            if t - last_print >= 5.0 {
                last_print = t;
                let (tern, enemy) = (&snap.ships[0], &snap.ships[1]);
                println!(
                    "[{t:7.1} s] {:?} {:.0} s: range {:.0} m, Tern hull {:.0}, Hound hull {:.0}; rtt {} ms, snapshot loss {:.1} %",
                    snap.phase,
                    snap.phase_s,
                    (enemy.pos - tern.pos).length(),
                    tern.hull,
                    enemy.hull,
                    s.rtt_ms.map(|r| format!("{r:.1}")).unwrap_or_else(|| "?".into()),
                    s.snapshot_loss() * 100.0
                );
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
