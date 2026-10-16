//! Single player (openspec/changes/single-player design 1-4): the drill hosted inside the client, and an NPC at every
//! other station of the mission's roster.
//!
//! It lives in the client because the client is where a single-player game runs; it adds no rule. The server is
//! `sc-server`'s own `DrillServer` on the local link (browser-single-player design 2: no socket and no thread, so a
//! browser page hosts it as a desktop does), ticked from the client's frame loop on the drill's fixed clock; each
//! NPC is a bot session (`sc-net::bot`) joined over the same link like any player, so a solo game is a multiplayer
//! game with one person in it (CLAUDE.md 6.3). The only thing kept here is the player's choice for each station,
//! AUTO or MANUAL, and on MANUAL the player's commands for that station go to the server through its NPC's session.

use std::path::Path;

use sc_core::clock::FixedClock;
use sc_core::combat::bodies::Bridge;
use sc_core::combat::data::DrillData;
use sc_core::combat::{Command, Station, TICK_HZ};
use sc_core::names::NameData;
use sc_core::rng::Rng;
use sc_net::bot::Bot;
use sc_net::session::Session;
use sc_net::transport::{Dial, Impair};
use sc_server::{DrillServer, Listen, ServerConfig};

/// An NPC crew member at one station.
pub struct Npc {
    /// The station they hold, or walk to: where they joined at first, then wherever the seats take them (a person
    /// relieves them, and they swap into the seat the person left: relieve-bots 3 and 5).
    pub station: Station,
    /// "Lt. Dal Varen".
    pub name: String,
    /// Their client session, over the local link.
    pub session: Session,
    /// What plays the station on AUTO.
    pub bot: Bot,
    /// AUTO: the NPC plays it. MANUAL: it does what the player does on its console.
    pub auto: bool,
}

impl Npc {
    /// Seated at their station: only then does the station work (design 4).
    pub fn seated(&self) -> bool {
        self.session.station() == Some(self.station)
    }

    /// Their station follows their seat: the one they hold, else the seat their body walks to.
    fn follow(&mut self) {
        let slot = self.session.slot;
        let going = self
            .session
            .latest()
            .and_then(|snap| snap.bodies.iter().find(|b| Some(b.slot) == slot))
            .and_then(|b| b.going);
        if let Some(st) = self.session.station().or(going) {
            self.station = st;
        }
    }
}

/// The drill server and the NPC crew of a single-player game.
pub struct SoloHost {
    server: DrillServer,
    clock: FixedClock,
    /// The player's own station: where they joined at first, then the station they last sat at (relieve-bots 5).
    pub seat: Station,
    /// One NPC per other station of the roster, in the roster's order.
    pub crew: Vec<Npc>,
}

/// The mission's stations in its roster's order, and the player's seat among them (Helm when the one asked for is not
/// on the roster).
pub fn roster(data: &DrillData, want: Station) -> (Vec<Station>, Station) {
    let stations: Vec<Station> = data.mission.stations.iter().filter_map(|s| Station::from_id(s)).collect();
    let seat = if stations.contains(&want) { want } else { stations.first().copied().unwrap_or(Station::Helm) };
    (stations, seat)
}

/// The walking bridge from the compiled deck, or straight paths without it; none when the seats cannot be read (the
/// crew then sit at once, as `--seated`).
pub fn bridge(data: &DrillData) -> Option<Bridge> {
    match sc_server::bridge::load(data, Path::new("."), Some(Path::new("compiled/tern.deck"))) {
        Ok((b, how)) => {
            println!("sc-client: single player: the crew on foot, {how}");
            Some(b)
        }
        Err(e) => {
            eprintln!("sc-client: single player: no bridge to walk ({e}); the crew sit at once");
            None
        }
    }
}

impl SoloHost {
    /// Host the drill on the local link with a crew for every station of the roster but `seat`, named from `names` and
    /// the session's `seed`.
    pub fn start(
        data: &DrillData,
        seat: Station,
        names: &NameData,
        seed: u64,
        bridge: Option<Bridge>,
    ) -> Result<Self, String> {
        let lo = std::net::Ipv4Addr::LOCALHOST.into();
        let cfg = ServerConfig { listen: Listen::Local, host_ip: lo, impair: Impair::default(), seed, bridge };
        let server = DrillServer::start(data.clone(), &cfg).map_err(|e| format!("single player: {e}"))?;
        let rank = names.ranks.iter().find(|r| r.name == "Lieutenant").or(names.ranks.first()).map_or("", |r| &r.short);
        let (stations, _) = roster(data, seat);
        let mut crew = Vec::new();
        for (i, st) in stations.into_iter().filter(|s| *s != seat).enumerate() {
            let name = format!("{rank} {}", names.generate(&mut Rng::for_purpose(seed, i as u64, "npc")));
            let session = Session::connect(server.dial(), &name, true, Some(st), Impair::default(), seed ^ i as u64)?;
            crew.push(Npc { station: st, name, session, bot: Bot::new(data, st), auto: true });
        }
        println!(
            "sc-client: single player on {}: the player at {}, {} NPCs ({})",
            server.dial(),
            seat.name(),
            crew.len(),
            crew.iter().map(|n| format!("{} {}", n.station.name(), n.name)).collect::<Vec<_>>().join(", ")
        );
        Ok(Self { server, clock: FixedClock::new(TICK_HZ, 8), seat, crew })
    }

    /// Where the player's session connects: the local link's door.
    pub fn dial(&self) -> Dial {
        self.server.dial()
    }

    /// The server, for checks.
    #[cfg(test)]
    pub fn server(&self) -> &DrillServer {
        &self.server
    }

    /// `dt` seconds of the game: the server's network and its ticks, then each NPC's session and decisions.
    pub fn step(&mut self, dt: f64, data: &DrillData) {
        for line in self.server.poll() {
            println!("sc-client: server: {line}");
        }
        for _ in 0..self.clock.advance(dt) {
            for line in self.server.tick() {
                println!("sc-client: server: {line}");
            }
        }
        for n in &mut self.crew {
            n.session.update();
            n.follow();
            n.bot.manual = !n.auto;
            n.bot.drive(&mut n.session, data);
        }
    }

    /// The player now holds `st` (or none, on foot): their own chip and Tab's first console follow the seat they sit
    /// at (relieve-bots 5).
    pub fn follow(&mut self, st: Option<Station>) {
        if let Some(st) = st {
            self.seat = st;
        }
    }

    /// The NPC at `st`.
    pub fn npc(&self, st: Station) -> Option<&Npc> {
        self.crew.iter().find(|n| n.station == st)
    }

    /// The NPC at `st`, to change.
    pub fn npc_mut(&mut self, st: Station) -> Option<&mut Npc> {
        self.crew.iter_mut().find(|n| n.station == st)
    }

    /// Every NPC is in their seat.
    pub fn all_seated(&self) -> bool {
        self.crew.iter().all(Npc::seated)
    }

    /// Send `c` as station `st`'s, through its NPC's session (MANUAL: the player's hands on its console).
    pub fn command_as(&mut self, st: Station, c: Command) {
        if let Some(n) = self.npc_mut(st) {
            n.session.command(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::combat::{Operator, Phase};
    use std::time::{Duration, Instant};

    fn names() -> NameData {
        sc_core::data::parse("data/crew/names.json", include_str!("../../../data/crew/names.json")).unwrap()
    }

    /// Step the host and the player until `done`, ticking four times faster than real time; false on timeout.
    fn run(
        host: &mut SoloHost,
        me: &mut Session,
        data: &DrillData,
        secs: f64,
        mut done: impl FnMut(&SoloHost, &Session) -> bool,
    ) -> bool {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        while Instant::now() < end {
            host.step(4.0 / TICK_HZ, data);
            me.update();
            if done(host, me) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        false
    }

    #[test]
    fn a_single_player_game_seats_an_npc_at_every_other_station_and_manual_plays_through_it() {
        let data = DrillData::shipped();
        let (stations, seat) = roster(&data, Station::Helm);
        let mut host = SoloHost::start(&data, seat, &names(), 7, None).unwrap();
        for n in &mut host.crew {
            n.bot.read_s = 0.2;
        }
        assert_eq!(host.crew.len(), stations.len() - 1, "an NPC at every station of the roster but the player's");
        let mut me = Session::connect(host.dial(), "Lt. Player", false, Some(seat), Impair::default(), 7).unwrap();
        let seated = run(&mut host, &mut me, &data, 20.0, |h, me| h.all_seated() && me.station() == Some(seat));
        assert!(seated, "every NPC and the player sat down");
        for st in &stations {
            assert_ne!(host.server().drill.operator(*st), Operator::Auto, "{} is held by someone", st.name());
        }
        // Science on MANUAL: its NPC does not scan, then the player's scan goes through as Science's.
        host.npc_mut(Station::Science).unwrap().auto = false;
        me.set_ready(true);
        let engaged = run(&mut host, &mut me, &data, 30.0, |h, _| h.server().drill.phase == Phase::Engage);
        assert!(engaged, "the round started with the crew ready");
        let quiet = Instant::now() + Duration::from_secs(2);
        run(&mut host, &mut me, &data, 3.0, |_, _| Instant::now() >= quiet);
        let b = &host.server().drill.bridge;
        assert!(!b.scanning && b.scan == 0.0, "Science on MANUAL sent nothing of its own in 2 s of the round");
        host.command_as(Station::Science, Command::Scan(true));
        let scanned = run(&mut host, &mut me, &data, 5.0, |h, _| h.server().drill.bridge.scanning);
        assert!(scanned, "the player's scan reached the server as Science's");
    }

    /// Walk the player's body to `to` as the client does: steps of at most a metre, each sent, while the game runs.
    fn walk_to(host: &mut SoloHost, me: &mut Session, data: &DrillData, to: [f32; 3]) {
        for _ in 0..400 {
            let Some(at) = me.latest().and_then(|s| s.bodies.iter().find(|b| Some(b.slot) == me.slot).map(|b| b.pos))
            else {
                run(host, me, data, 0.05, |_, _| false);
                continue;
            };
            let v = [to[0] - at[0], to[1] - at[1], to[2] - at[2]];
            let len = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if len < 0.05 {
                return;
            }
            let k = (1.0 / len).min(1.0);
            me.walked([at[0] + v[0] * k, at[1] + v[1] * k, at[2] + v[2] * k], 0.0);
            run(host, me, data, 0.05, |_, _| false);
        }
        panic!("the walk never arrived at {to:?}");
    }

    #[test]
    fn the_player_takes_the_captains_chair_in_a_fight_and_the_tabs_follow_the_swap() {
        // The owner, 2026-10-15: "for example I'm helmsman, I should be able to sit in captains seat whenever"
        // (relieve-bots 5): after the swap the player's own chip is the captain's and the captain's NPC holds the helm.
        let data = DrillData::shipped();
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let (bridge, _) = sc_server::bridge::load(&data, root, None).expect("the bridge's seats");
        let (helm, captain) = (bridge.stand_point(Station::Helm), bridge.stand_point(Station::Captain));
        let mut host = SoloHost::start(&data, Station::Helm, &names(), 7, Some(bridge)).unwrap();
        for n in &mut host.crew {
            n.bot.read_s = 0.2;
        }
        let mut me =
            Session::connect(host.dial(), "Capt. Raggra of House Da", false, None, Impair::default(), 7).unwrap();
        assert!(run(&mut host, &mut me, &data, 30.0, |_, me| me.slot.is_some()), "joined");
        walk_to(&mut host, &mut me, &data, helm);
        me.claim(Station::Helm);
        assert!(run(&mut host, &mut me, &data, 30.0, |h, me| h.all_seated() && me.station() == Some(Station::Helm)));
        me.set_ready(true);
        assert!(run(&mut host, &mut me, &data, 60.0, |h, _| h.server().drill.phase == Phase::Engage), "the fight");
        let skipper = host.npc(Station::Captain).map(|n| n.name.clone()).expect("an NPC captain");
        me.stand();
        assert!(run(&mut host, &mut me, &data, 10.0, |_, me| me.station().is_none()), "up from the helm");
        walk_to(&mut host, &mut me, &data, captain);
        me.claim(Station::Captain);
        let swapped = run(&mut host, &mut me, &data, 30.0, |h, me| {
            me.station() == Some(Station::Captain) && h.npc(Station::Helm).is_some_and(|n| n.seated())
        });
        assert!(swapped, "the player sat in the captain's chair and an NPC took the helm");
        host.follow(me.station());
        assert_eq!(host.seat, Station::Captain, "the player's own chip is the captain's now");
        assert_eq!(
            host.npc(Station::Helm).map(|n| n.name.clone()),
            Some(skipper),
            "the captain's NPC swapped to the helm"
        );
        assert!(host.npc(Station::Captain).is_none(), "no NPC tab for the player's own seat");
        assert!(host.all_seated(), "READY waits for nobody");
        assert_eq!(host.server().drill.phase, Phase::Engage, "all in the fight");
    }
}
