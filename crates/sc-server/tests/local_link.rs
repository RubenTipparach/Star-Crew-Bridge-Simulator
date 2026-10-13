//! The drill server on the local link (openspec/changes/browser-single-player design 4 and 8): a peer is silent only
//! on both the server's ticks and the wall clock, so a paused game (a hidden browser tab runs no frames) drops nobody,
//! and a peer that stays silent while the server runs is dropped as before.

use std::time::Duration;

use sc_core::combat::data::DrillData;
use sc_core::combat::{Operator, Station, TICK_HZ};
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::{DrillServer, Listen, ServerConfig, SILENT_S};

fn local_server() -> DrillServer {
    let cfg = ServerConfig {
        listen: Listen::Local,
        host_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        impair: Impair::default(),
        seed: 11,
        bridge: None,
    };
    DrillServer::start(DrillData::shipped(), &cfg).unwrap()
}

/// Poll and tick the server and update the client until the client holds Helm (no thread, no sleep: one process).
fn join_helm(server: &mut DrillServer, c: &mut Session) {
    for _ in 0..200 {
        server.poll();
        server.tick();
        c.update();
        if server.drill.operator(Station::Helm) != Operator::Auto {
            return;
        }
    }
    panic!("the client never took the helm over the local link");
}

#[test]
fn a_server_stepped_faster_than_real_time_keeps_a_client_that_pings_each_second() {
    let mut server = local_server();
    let mut c = Session::connect(server.dial(), "Fast", false, Some(Station::Helm), Impair::default(), 3).unwrap();
    join_helm(&mut server, &mut c);
    // Ten times the silence limit in ticks, in well under a second of wall clock, with the client quiet throughout.
    for _ in 0..(10.0 * SILENT_S * TICK_HZ) as usize {
        server.poll();
        server.tick();
    }
    c.update();
    server.poll();
    assert_ne!(server.drill.operator(Station::Helm), Operator::Auto, "a fast server is not a silent client");
}

#[test]
fn a_game_paused_longer_than_the_silence_limit_drops_no_one() {
    let mut server = local_server();
    let mut c = Session::connect(server.dial(), "Paused", false, Some(Station::Helm), Impair::default(), 1).unwrap();
    join_helm(&mut server, &mut c);
    // The whole game stops, as a hidden tab stops it: nothing ticks, nothing is sent, for longer than SILENT_S.
    std::thread::sleep(Duration::from_secs_f64(SILENT_S + 1.0));
    server.poll();
    server.tick();
    c.update();
    assert_ne!(
        server.drill.operator(Station::Helm),
        Operator::Auto,
        "a pause of the whole game is not a silent peer: the player keeps the helm"
    );
}

#[test]
fn a_peer_silent_while_the_server_runs_loses_its_seat() {
    let mut server = local_server();
    let mut c = Session::connect(server.dial(), "Gone", false, Some(Station::Helm), Impair::default(), 2).unwrap();
    join_helm(&mut server, &mut c);
    let limit = (SILENT_S * TICK_HZ).ceil() as usize;
    // The server takes in what the client last sent, then runs on while the client sends nothing: for the limit on
    // the wall clock and then in ticks.
    server.poll();
    std::thread::sleep(Duration::from_secs_f64(SILENT_S + 0.1));
    for _ in 0..limit {
        server.poll();
        server.tick();
    }
    assert_ne!(server.drill.operator(Station::Helm), Operator::Auto, "not yet: exactly the limit has passed");
    for _ in 0..2 {
        server.poll();
        server.tick();
    }
    assert_eq!(
        server.drill.operator(Station::Helm),
        Operator::Auto,
        "silent past {SILENT_S} s of ticks: dropped, and the seat went back to automation"
    );
}
