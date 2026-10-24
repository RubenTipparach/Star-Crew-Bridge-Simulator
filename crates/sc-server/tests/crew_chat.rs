//! Crew chat through the real server and sessions on the local link (openspec/changes/crew-chat design 1, 3 and 4):
//! a typed line reaches every crew member stamped with who said it, a late joiner gets the last lines, a player typing
//! too fast is held back, and a bot says what it does through the same message a player types in.

use sc_core::combat::chat::{Kind, Line};
use sc_core::combat::data::DrillData;
use sc_core::combat::{Station, TICK_HZ};
use sc_net::bot::Bot;
use sc_net::session::{Session, Stage};
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

fn join(server: &DrillServer, name: &str, station: Station) -> Session {
    Session::connect(server.dial(), name, false, Some(station), Impair::default(), 1).unwrap()
}

/// Run the server `ticks` ticks, the clients updating between them; returns each client's chat lines, in order.
fn run(server: &mut DrillServer, clients: &mut [&mut Session], ticks: usize) -> Vec<Vec<Line>> {
    let mut heard = vec![Vec::new(); clients.len()];
    for _ in 0..ticks {
        server.poll();
        server.tick();
        for (c, h) in clients.iter_mut().zip(&mut heard) {
            c.update();
            h.extend(c.drain_chat());
        }
    }
    heard
}

fn texts(lines: &[Line]) -> Vec<&str> {
    lines.iter().filter(|l| l.kind == Kind::Say).map(|l| l.text.as_str()).collect()
}

#[test]
fn a_typed_line_reaches_every_crew_member_with_its_sender() {
    let mut server = local_server();
    let (mut a, mut b) =
        (join(&server, "Ens. Dara Holt", Station::Tactical), join(&server, "Lt. Player", Station::Helm));
    run(&mut server, &mut [&mut a, &mut b], 30);
    assert_eq!((a.stage, b.stage), (Stage::Joined, Stage::Joined));
    a.say(Kind::Say, "  Ready at Tactical ");
    let heard = run(&mut server, &mut [&mut a, &mut b], 10);
    for h in &heard {
        let l = h.iter().find(|l| l.kind == Kind::Say).expect("the line arrived");
        assert_eq!(
            (l.slot, l.name.as_str(), l.station, l.npc, l.text.as_str()),
            (a.slot, "Ens. Dara Holt", Some(Station::Tactical), false, "Ready at Tactical"),
            "the sender too, cleaned and stamped by the server"
        );
        assert!(l.at_s > 0.0, "stamped with the drill's clock");
    }
}

#[test]
fn a_late_joiner_gets_the_last_lines() {
    let mut server = local_server();
    let mut a = join(&server, "Lt. Player", Station::Captain);
    run(&mut server, &mut [&mut a], 30);
    for t in ["Evening all", "Fighters first", "Then the Hound"] {
        a.say(Kind::Say, t);
        // More than a speaker's gap of the drill's clock between lines.
        run(&mut server, &mut [&mut a], (1.2 * TICK_HZ) as usize);
    }
    let mut late = join(&server, "Ens. Dara Holt", Station::Tactical);
    let heard = run(&mut server, &mut [&mut a, &mut late], 30);
    assert_eq!(texts(&heard[1]), ["Evening all", "Fighters first", "Then the Hound"], "oldest first");
}

#[test]
fn typing_faster_than_a_speakers_gap_is_dropped_by_the_server() {
    let mut server = local_server();
    let (mut a, mut b) =
        (join(&server, "Lt. Player", Station::Captain), join(&server, "Ens. Dara Holt", Station::Helm));
    run(&mut server, &mut [&mut a, &mut b], 30);
    a.say(Kind::Say, "One");
    a.say(Kind::Say, "Two");
    let heard = run(&mut server, &mut [&mut a, &mut b], 10);
    assert_eq!(texts(&heard[1]), ["One"], "the second came inside the gap");
}

#[test]
fn a_bot_says_it_is_at_its_station_through_the_chat() {
    let data = DrillData::shipped();
    let mut server = local_server();
    let mut me = join(&server, "Lt. Player", Station::Captain);
    let mut bot_s =
        Session::connect(server.dial(), "Lt. Sil Saren", true, Some(Station::Helm), Impair::default(), 2).unwrap();
    let mut bot = Bot::new(&data, Station::Helm);
    let mut heard = Vec::new();
    for _ in 0..60 {
        server.poll();
        server.tick();
        me.update();
        bot_s.update();
        bot.drive(&mut bot_s, &data);
        heard.extend(me.drain_chat());
    }
    let l = heard.iter().find(|l| l.text == "At Helm").expect("the bot said it sat down");
    assert_eq!((l.name.as_str(), l.station, l.npc, l.kind), ("Lt. Sil Saren", Some(Station::Helm), true, Kind::Report));
}

#[test]
fn a_captains_order_from_the_console_is_said_in_chat_as_theirs() {
    let data = DrillData::shipped();
    let mut server = local_server();
    let (mut cap, mut tac) =
        (join(&server, "Lt. Player", Station::Captain), join(&server, "Ens. Dara Holt", Station::Tactical));
    run(&mut server, &mut [&mut cap, &mut tac], 30);
    // Into Engage: orders are applied only in a fight.
    cap.set_ready(true);
    tac.set_ready(true);
    // The round waits for START MISSION (start-mission 1): the captain's call.
    cap.start_mission();
    let mut ticks = 0;
    while server.drill.phase != sc_core::combat::Phase::Engage && ticks < 2000 {
        run(&mut server, &mut [&mut cap, &mut tac], 1);
        ticks += 1;
    }
    let engage = data.order_verbs(Station::Tactical).iter().position(|v| v[0] == "ENGAGE").unwrap() as u8;
    cap.command(sc_core::combat::Command::Order(Station::Tactical, engage));
    let heard = run(&mut server, &mut [&mut cap, &mut tac], 10);
    let l = heard[1].iter().find(|l| l.kind == Kind::Order && l.slot == cap.slot).expect("the order in chat");
    assert_eq!((l.name.as_str(), l.text.as_str()), ("Lt. Player", "Tactical, engage the Hound corvette"));
}

#[test]
fn a_bot_that_takes_the_helm_in_the_fight_holds_the_course_it_found() {
    // playtest-seats-and-retreat design 3a: leaving a station changes who operates it, never what it holds. A bot
    // that sits at the helm mid-fight neither hunts the hostile nor acknowledges an order to close on it.
    let data = DrillData::shipped();
    let mut server = local_server();
    let mut cap = join(&server, "Lt. Player", Station::Captain);
    run(&mut server, &mut [&mut cap], 30);
    cap.set_ready(true);
    cap.start_mission();
    let mut ticks = 0;
    while server.drill.phase != sc_core::combat::Phase::Engage && ticks < 2000 {
        run(&mut server, &mut [&mut cap], 1);
        ticks += 1;
    }
    assert_eq!(server.drill.phase, sc_core::combat::Phase::Engage);
    let mut bot_s =
        Session::connect(server.dial(), "Lt. Sil Saren", true, Some(Station::Helm), Impair::default(), 2).unwrap();
    let mut bot = Bot::new(&data, Station::Helm);
    let chase = data.order_verbs(Station::Helm).iter().position(|v| v[0] == "CHASE").unwrap() as u8;
    let mut heard = Vec::new();
    for t in 0..(8.0 * TICK_HZ) as usize {
        if t == (4.0 * TICK_HZ) as usize {
            assert_eq!(bot_s.station(), Some(Station::Helm), "seated before the order comes");
            cap.command(sc_core::combat::Command::Order(Station::Helm, chase));
        }
        server.poll();
        server.tick();
        cap.update();
        bot_s.update();
        bot.drive(&mut bot_s, &data);
        heard.extend(cap.drain_chat());
    }
    assert_eq!(bot_s.station(), Some(Station::Helm), "the bot took the helm");
    let said: Vec<&str> = heard.iter().filter(|l| l.npc).map(|l| l.text.as_str()).collect();
    assert!(!said.iter().any(|t| t.starts_with("Bringing her about") || t.starts_with("Aye, closing")), "{said:?}");
    assert!(
        server.drill.bridge.orders.iter().any(|o| o.to == Station::Helm && !o.done),
        "the order to close stands unacknowledged: nobody is flying it ({:?})",
        server.drill.bridge.orders
    );
}
