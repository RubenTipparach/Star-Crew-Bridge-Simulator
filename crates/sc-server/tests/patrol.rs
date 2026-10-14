//! The patrol through the real server and bot sessions on the local link (openspec/changes/patrol-mission design 9):
//! a crew of bots starts the drive, undocks, sweeps and jumps from what their snapshots show, and each says what it
//! does in the crew chat through the lines a player's would use.

use sc_core::combat::chat::{Kind, Line};
use sc_core::combat::data::DrillData;
use sc_core::combat::patrol::Stage;
use sc_core::combat::{Phase, Station, TICK_HZ};
use sc_net::bot::Bot;
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::{DrillServer, Listen, ServerConfig};
use std::time::{Duration, Instant};

#[test]
fn a_crew_of_bots_undocks_jumps_and_sweeps_and_says_so() {
    let data = DrillData::shipped_patrol();
    let cfg = ServerConfig {
        listen: Listen::Local,
        host_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        impair: Impair::default(),
        seed: 5,
        bridge: None,
    };
    let mut server = DrillServer::start(data.clone(), &cfg).unwrap();
    let crew = [Station::Helm, Station::Engineering, Station::Science, Station::Captain];
    let mut sessions: Vec<Session> = crew
        .iter()
        .map(|s| Session::connect(server.dial(), &format!("{} bot", s.name()), true, Some(*s), Impair::default(), 1))
        .collect::<Result<_, _>>()
        .unwrap();
    let mut bots: Vec<Bot> = crew
        .iter()
        .map(|s| {
            let mut b = Bot::new(&data, *s);
            b.read_s = 0.2;
            b
        })
        .collect();
    let mut heard: Vec<Line> = Vec::new();
    let mut jumped_at = None;
    // The bots decide on the clients' real clocks: the server runs at 6x real time, as the LAN drill's test does.
    let mut tick = 0usize;
    let mut next_tick = Instant::now();
    while tick < (400.0 * TICK_HZ) as usize {
        server.poll();
        if Instant::now() >= next_tick {
            server.tick();
            tick += 1;
            next_tick += Duration::from_secs_f64(1.0 / (6.0 * TICK_HZ));
        }
        for (s, b) in sessions.iter_mut().zip(&mut bots) {
            s.update();
            b.drive(s, &data);
        }
        std::thread::sleep(Duration::from_millis(1));
        heard.extend(sessions[0].drain_chat());
        let p = sessions[0].latest().and_then(|x| x.patrol.clone());
        if p.as_ref().is_some_and(|p| p.jumps >= 1 && p.swept.get(1) == Some(&true)) {
            jumped_at = Some(tick as f64 / TICK_HZ);
            break;
        }
    }
    let t = jumped_at.unwrap_or_else(|| {
        let x = sessions[0].latest();
        panic!(
            "the bots did not reach and sweep the Pell beacon within 400 s: phase {:?}, patrol {:?}, heard {:?}",
            x.map(|x| x.phase),
            x.and_then(|x| x.patrol.clone()),
            heard.iter().map(|l| &l.text).collect::<Vec<_>>()
        )
    });
    let snap = sessions[0].latest().unwrap();
    assert_eq!(snap.phase, Phase::Patrol);
    assert_eq!(snap.patrol.as_ref().unwrap().stage, Stage::Underway);
    let said = |text: &str| heard.iter().any(|l| l.npc && l.text == text);
    for (text, kind) in [
        ("Off shore power", Kind::Intent),
        ("Helm, take us out", Kind::Order),
        ("Releasing the clamps", Kind::Intent),
        ("Spooling the jump drive", Kind::Intent),
        ("Sweeping", Kind::Intent),
    ] {
        assert!(said(text), "a bot said {text:?} (heard {:?})", heard.iter().map(|l| &l.text).collect::<Vec<_>>());
        assert!(heard.iter().any(|l| l.text == text && l.kind == kind), "{text:?} is a {kind:?}");
    }
    println!("bots: at the Pell beacon and swept after {t:.0} s");
}
