//! `sc-server`: the authoritative loop (engine-stack design sections 3 and 6), headless on a 4 GB Pi 5 or any
//! machine. It renders nothing and plays nothing.
//!
//! It runs the co-op drill (openspec/changes/coop-drill) on a LAN: signalling and the status page on TCP (7700 by
//! default), one WebRTC peer per client, the fixed 30 Hz clock from `sc-core`, a log line for every join, leave,
//! phase and round. The drill repeats until the process is stopped.
//!
//! Usage: sc-server [--root .] [--mission drill-hound] [--listen 0.0.0.0:7700] [--host-ip IP] [--seed N]
//!                  [--loss 0.1] [--delay-ms 50] [--rounds N] [--seconds S] [--seated] [--deck compiled/tern.deck]
//!                  [--list NAME | --unlisted] [--matchmaker http://host:port]
//!
//! `--list NAME` puts the session in the matchmaker's public list (the server browser, matchmaker design 10);
//! `--unlisted` registers it to be joined by its code only. The matchmaker is `--matchmaker`, else the one in
//! `data/matchmaker/client.json`. The host key that holds the listing is made on the first run and kept in
//! `settings/host-key`.
//!
//! The crew are on foot (design 9) unless `--seated`: a claim is a walk across the bridge, on paths from the
//! compiled deck's walk grid when it is there.

use sc_core::clock::FixedClock;
use sc_core::combat::{data::DrillData, data::DRILL_FILES, TICK_HZ};
use sc_net::transport::{local_ip_toward, Impair};
use sc_server::{DrillServer, ServerConfig};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

fn stamp(start: Instant) -> String {
    format!("[{:8.1} s]", start.elapsed().as_secs_f64())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let root = PathBuf::from(value("--root").unwrap_or_else(|| ".".into()));
    let mission = value("--mission").unwrap_or_else(|| "drill-hound".into());
    let listen: SocketAddr = match value("--listen").unwrap_or_else(|| "0.0.0.0:7700".into()).parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sc-server: --listen: {e}");
            return ExitCode::from(2);
        }
    };
    let host_ip: IpAddr = match value("--host-ip") {
        Some(ip) => match ip.parse() {
            Ok(ip) => ip,
            Err(e) => {
                eprintln!("sc-server: --host-ip: {e}");
                return ExitCode::from(2);
            }
        },
        // The address this machine reaches the LAN by (no packet is sent).
        None => local_ip_toward("192.168.0.1".parse().expect("an address"))
            .unwrap_or_else(|_| "127.0.0.1".parse().expect("an address")),
    };
    let num = |name: &str, default: f64| value(name).and_then(|v| v.parse::<f64>().ok()).unwrap_or(default);
    let impair = Impair { loss: num("--loss", 0.0).clamp(0.0, 1.0), delay_ms: num("--delay-ms", 0.0).max(0.0) as u64 };
    let seed = num("--seed", 1.0) as u64;
    let rounds = value("--rounds").and_then(|v| v.parse::<usize>().ok());
    let seconds = value("--seconds").and_then(|v| v.parse::<f64>().ok());

    // Data: a file that fails to parse stops startup with its path and field (CLAUDE.md 6.5).
    let read = |rel: &str| std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: cannot be read: {e}"));
    let mission_rel = format!("data/missions/{mission}.json");
    let texts: Result<Vec<String>, String> = DRILL_FILES.iter().map(|f| read(f)).collect();
    let data = texts.and_then(|t| {
        let arr: [&str; sc_core::combat::data::DRILL_FILE_COUNT] = std::array::from_fn(|i| t[i].as_str());
        let m = read(&mission_rel)?;
        DrillData::parse(arr, &mission_rel, &m).map_err(|e| e.to_string())
    });
    let data = match data {
        Ok(d) => d,
        Err(e) => {
            eprintln!("sc-server: {e}");
            return ExitCode::FAILURE;
        }
    };
    let bridge = if args.iter().any(|a| a == "--seated") {
        None
    } else {
        let deck = root.join(value("--deck").unwrap_or_else(|| "compiled/tern.deck".into()));
        match sc_server::bridge::load(&data, &root, Some(&deck)) {
            Ok((b, how)) => {
                println!("[     0.0 s] sc-server: the crew on foot: {how}");
                Some(b)
            }
            Err(e) => {
                eprintln!("sc-server: the bridge: {e}");
                return ExitCode::FAILURE;
            }
        }
    };
    let cfg = ServerConfig { listen: listen.into(), host_ip, impair, seed, bridge };
    let mut server = match DrillServer::start(data, &cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sc-server: cannot listen on {listen}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let start = Instant::now();
    println!(
        "{} sc-server: {} on {} (peers on {host_ip}), seed {seed}, impairment {:.0} % loss and {} ms delay; status at http://{}:{}/status",
        stamp(start),
        server.drill.data.mission.title,
        server.addr(),
        impair.loss * 100.0,
        impair.delay_ms,
        host_ip,
        server.addr().port()
    );
    // The server browser (matchmaker design 10.3): listed, or by code only, when asked.
    let list_name = value("--list");
    let unlisted = args.iter().any(|a| a == "--unlisted");
    let listed = if list_name.is_some() || unlisted {
        let mm = sc_net::matchmaker::Matchmaker::configured(&root, value("--matchmaker").as_deref());
        let key = host_key(&root);
        let name = list_name.clone().unwrap_or_else(|| server.drill.data.mission.title.clone());
        match mm.and_then(|m| key.and_then(|k| server.list(m.clone(), k, &name, list_name.is_some()).map(|l| (l, m)))) {
            Ok((l, m)) => {
                println!(
                    "{} sc-server: {} at {} as \"{name}\", join code {}",
                    stamp(start),
                    if list_name.is_some() { "listed" } else { "registered unlisted" },
                    m.url,
                    l.code()
                );
                Some(l)
            }
            Err(e) => {
                eprintln!("sc-server: the matchmaker: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    let mut clock = FixedClock::new(TICK_HZ, 8);
    let mut last = Instant::now();
    loop {
        for line in server.poll() {
            println!("{} {line}", stamp(start));
        }
        let now = Instant::now();
        let n = clock.advance((now - last).as_secs_f64());
        last = now;
        for _ in 0..n {
            for line in server.tick() {
                println!("{} {line}", stamp(start));
            }
        }
        if let Some(l) = &listed {
            l.set(server.heartbeat());
        }
        if rounds.is_some_and(|r| server.results.len() >= r)
            || seconds.is_some_and(|s| start.elapsed().as_secs_f64() >= s)
        {
            println!("{} sc-server: done; {}", stamp(start), server.status_json());
            return ExitCode::SUCCESS;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The host key that holds this server's listing (matchmaker design 7): made from the operating system's random
/// source on the first run and kept in `settings/host-key`.
fn host_key(root: &std::path::Path) -> Result<String, String> {
    let path = root.join("settings/host-key");
    if let Ok(k) = std::fs::read_to_string(&path) {
        let k = k.trim().to_owned();
        if k.len() >= 16 {
            return Ok(k);
        }
    }
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("no random source for the host key: {e}"))?;
    let key: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(root.join("settings"))
        .and_then(|()| std::fs::write(&path, format!("{key}\n")))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(key)
}
