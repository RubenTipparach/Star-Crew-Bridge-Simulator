//! A server and two bot clients over loopback WebRTC play the drill to its end (openspec/changes/coop-drill
//! task 3.4): the whole network path (signalling, three channels, the codec, the seat checks, automation's
//! commands from a client) with nothing mocked. The server ticks four times faster than real time to keep the test
//! short; the bots decide from snapshots exactly as on two machines.

use sc_core::combat::data::DrillData;
use sc_core::combat::{Outcome, Station};
use sc_net::bot::Bot;
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::{DrillServer, ServerConfig};
use std::time::{Duration, Instant};

fn play(impair: Impair, seed: u64) -> (DrillServer, Vec<Session>) {
    let lo = "127.0.0.1".parse().unwrap();
    let cfg = ServerConfig { listen: "127.0.0.1:0".parse().unwrap(), host_ip: lo, impair, seed, bridge: None };
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

#[test]
fn a_client_that_goes_silent_loses_its_seat_to_automation_within_six_seconds() {
    use sc_core::combat::Operator;
    let lo = "127.0.0.1".parse().unwrap();
    let cfg = ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        host_ip: lo,
        impair: Impair::default(),
        seed: 3,
        bridge: None,
    };
    let mut server = DrillServer::start(DrillData::shipped(), &cfg).unwrap();
    let mut c = Session::connect(server.addr(), "Gone", false, Some(Station::Helm), Impair::default(), 3).unwrap();
    let joined = Instant::now() + Duration::from_secs(10);
    while Instant::now() < joined && server.drill.operator(Station::Helm) == Operator::Auto {
        server.poll();
        server.tick();
        c.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_ne!(server.drill.operator(Station::Helm), Operator::Auto, "the client took the helm");
    // The client stops updating: no pings, no sticks. Its socket stays open, as a hung process's would.
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(8) && server.drill.operator(Station::Helm) != Operator::Auto {
        server.poll();
        server.tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    let took = t0.elapsed().as_secs_f64();
    assert_eq!(server.drill.operator(Station::Helm), Operator::Auto, "the seat went back to automation");
    assert!(took < 6.5, "within the 5 s silence limit and a little, took {took:.1} s");
    drop(c);
}

/// The Tern's bridge seats and muster point (layout.json), straight paths: the test needs no compiled deck.
fn bridge(data: &DrillData) -> sc_core::combat::bodies::Bridge {
    let m = &data.mission;
    sc_core::combat::bodies::Bridge::straight(
        vec![(Station::Helm, [1.8, 3.5, 28.2], 0.0), (Station::Tactical, [-1.8, 3.5, 28.2], 0.0)],
        [m.muster_m[0] as f32, m.muster_m[1] as f32, m.muster_m[2] as f32],
        m.muster_spacing_m as f32,
        2.4,
        data.stations.seats.stand_s as f32,
        data.stations.seats.sit_s as f32,
    )
}

#[test]
fn on_foot_a_lone_tactical_bot_walks_to_helm_and_a_second_bot_takes_tactical() {
    use sc_core::combat::Operator;
    let lo = "127.0.0.1".parse().unwrap();
    let data = DrillData::shipped();
    let cfg = ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        host_ip: lo,
        impair: Impair::default(),
        seed: 5,
        bridge: Some(bridge(&data)),
    };
    let mut server = DrillServer::start(data.clone(), &cfg).unwrap();
    let mut clients: Vec<(Session, Bot)> = Vec::new();
    let addr = server.addr();
    let join = |s: Station| {
        let mut bot = Bot::new(&data, s);
        bot.read_s = 0.2;
        (Session::connect(addr, s.name(), true, Some(s), Impair::default(), 5).unwrap(), bot)
    };
    clients.push(join(Station::Tactical));
    let mut seen = Vec::new();
    let mut second = false;
    let t0 = Instant::now();
    let mut next_tick = Instant::now();
    let mut ticks = 0u32;
    while t0.elapsed() < Duration::from_secs(60) {
        server.poll();
        if Instant::now() >= next_tick {
            server.tick();
            ticks += 1;
            next_tick += Duration::from_secs_f64(1.0 / 120.0);
            let op = (server.drill.operator(Station::Helm), server.drill.operator(Station::Tactical));
            if seen.last() != Some(&op) {
                seen.push(op);
            }
        }
        // Once the first bot holds Helm, a second joins wanting Helm, and takes Tactical instead.
        if !second && matches!(server.drill.operator(Station::Helm), Operator::Player(_)) {
            second = true;
            let c = join(Station::Helm);
            clients.push(c);
        }
        for (c, bot) in &mut clients {
            c.update();
            bot.drive(c, &data);
        }
        if second && matches!(server.drill.operator(Station::Tactical), Operator::Player(_)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let first = clients[0].0.slot.expect("the first bot joined");
    assert!(seen.contains(&(Operator::Auto, Operator::Player(first))), "the bot sat at Tactical first: {seen:?}");
    assert!(
        seen.contains(&(Operator::Player(first), Operator::Auto)),
        "then got up and walked to Helm, leaving Tactical to automation: {seen:?}"
    );
    let second_slot = clients.get(1).and_then(|c| c.0.slot).expect("a second bot joined");
    assert_eq!(server.drill.operator(Station::Tactical), Operator::Player(second_slot), "{seen:?}");
    let s = f64::from(ticks) / 30.0;
    assert!(s < 40.0, "all of it inside 40 s of drill time: {s:.1}");
}
