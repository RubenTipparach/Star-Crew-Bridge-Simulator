//! START MISSION through the server (openspec/changes/start-mission design 2 and 6): the captain's message begins the
//! countdown with nobody ready, and the same message from another station is refused with the captain's call.

use sc_core::combat::data::DrillData;
use sc_core::combat::{Operator, Phase, Refusal, Station};
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::{DrillServer, Listen, ServerConfig};

fn local_server() -> DrillServer {
    let cfg = ServerConfig {
        listen: Listen::Local,
        host_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        impair: Impair::default(),
        seed: 5,
        bridge: None,
    };
    DrillServer::start(DrillData::shipped(), &cfg).unwrap()
}

/// Poll and tick the server and update the clients until `done` holds (one process, no thread, no sleep).
fn run(server: &mut DrillServer, clients: &mut [&mut Session], what: &str, done: impl Fn(&DrillServer) -> bool) {
    for _ in 0..300 {
        server.poll();
        server.tick();
        for c in clients.iter_mut() {
            c.update();
        }
        if done(server) {
            return;
        }
    }
    panic!("never happened: {what}");
}

#[test]
fn the_captains_start_mission_begins_the_countdown_and_another_stations_is_refused() {
    let mut server = local_server();
    let mut cap =
        Session::connect(server.dial(), "Capt. A", false, Some(Station::Captain), Impair::default(), 1).unwrap();
    let mut helm = Session::connect(server.dial(), "Lt. B", false, Some(Station::Helm), Impair::default(), 2).unwrap();
    run(&mut server, &mut [&mut cap, &mut helm], "both seated", |s| {
        s.drill.operator(Station::Captain) != Operator::Auto && s.drill.operator(Station::Helm) != Operator::Auto
    });
    assert_eq!(server.drill.phase, Phase::Muster, "nobody has said READY");

    helm.start_mission();
    for _ in 0..20 {
        server.poll();
        server.tick();
        helm.update();
        cap.update();
    }
    assert_eq!(helm.refused.map(|r| r.0), Some(Refusal::CaptainsCall), "only the captain starts the mission");
    assert_eq!(server.drill.phase, Phase::Muster, "a refused start changes nothing");

    cap.start_mission();
    run(&mut server, &mut [&mut cap, &mut helm], "the countdown", |s| s.drill.phase == Phase::Countdown);
    assert_eq!(cap.refused, None, "the captain's call is taken");
}
