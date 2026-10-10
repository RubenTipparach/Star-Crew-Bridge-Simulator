//! A server and two bot clients over loopback WebRTC play the drill to its end (openspec/changes/coop-drill
//! task 3.4): the whole network path (signalling, three channels, the codec, the seat checks, automation's
//! commands from a client) with nothing mocked. The server ticks four times faster than real time to keep the test
//! short; the bots decide from snapshots exactly as on two machines.

use sc_core::combat::data::DrillData;
use sc_core::combat::{Outcome, Station};
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::bot::Bot;
use sc_server::{DrillServer, ServerConfig};
use std::time::{Duration, Instant};

fn play(impair: Impair, seed: u64) -> (DrillServer, Vec<Session>) {
    let lo = "127.0.0.1".parse().unwrap();
    let cfg = ServerConfig { listen: "127.0.0.1:0".parse().unwrap(), host_ip: lo, impair, seed };
    let data = DrillData::shipped();
    let mut server = DrillServer::start(data.clone(), &cfg).unwrap();
    let mut clients: Vec<(Session, Bot)> = [Station::Helm, Station::Tactical]
        .into_iter()
        .map(|s| {
            let mut bot = Bot::new(&data, s);
            bot.read_s = 0.2;
            (Session::connect(server.addr(), s.name(), true, Some(s), Impair::default(), seed).unwrap(), bot)
        })
        .collect();
    let end = Instant::now() + Duration::from_secs(120);
    let mut next_tick = Instant::now();
    while Instant::now() < end && server.results.is_empty() {
        server.poll();
        if Instant::now() >= next_tick {
            server.tick();
            next_tick += Duration::from_secs_f64(1.0 / 120.0);
        }
        for (c, bot) in &mut clients {
            c.update();
            bot.drive(c, &data);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    (server, clients.into_iter().map(|c| c.0).collect())
}

#[test]
fn two_bot_clients_win_the_drill_over_loopback_webrtc() {
    let (server, clients) = play(Impair::default(), 11);
    assert_eq!(server.results.len(), 1, "a round finished inside the time limit; status: {}", server.status_json());
    assert_eq!(server.results[0].outcome, format!("{:?}", Outcome::Victory), "{:?}", server.results[0]);
    assert!(
        server.results[0].crew.contains("Helm Helm (bot)")
            && server.results[0].crew.contains("Tactical Tactical (bot)"),
        "both seats were the clients': {}",
        server.results[0].crew
    );
    for c in &clients {
        assert!(c.rtt_ms.is_some(), "each client measured a round trip");
        assert_eq!(c.dropped, 0, "nothing failed to decode");
    }
}

#[test]
fn the_drill_survives_ten_percent_loss_and_fifty_ms_of_delay() {
    let (server, clients) = play(Impair { loss: 0.1, delay_ms: 50 }, 12);
    assert_eq!(server.results.len(), 1, "a round finished; status: {}", server.status_json());
    assert_eq!(server.results[0].outcome, format!("{:?}", Outcome::Victory), "{:?}", server.results[0]);
    for c in &clients {
        let loss = c.snapshot_loss();
        assert!((0.03..0.2).contains(&loss), "the client saw the impairment's loss on snapshots: {loss:.3}");
    }
}
