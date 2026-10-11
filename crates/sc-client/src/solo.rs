//! Single player (openspec/changes/single-player design 1-4): the drill hosted inside the client, and an NPC at every
//! other station of the mission's roster.
//!
//! It lives in the client because the client is where a single-player game runs; it adds no rule. The server is
//! `sc-server`'s own `DrillServer` on loopback, ticked from the client's frame loop on the drill's fixed clock; each
//! NPC is a bot session (`sc-net::bot`) joined over loopback like any player, so a solo game is a multiplayer game
//! with one person in it (CLAUDE.md 6.3). The only thing kept here is the player's choice for each station, AUTO or
//! MANUAL, and on MANUAL the player's commands for that station go to the server through its NPC's session.

use std::net::SocketAddr;
use std::path::Path;

use sc_core::clock::FixedClock;
use sc_core::combat::bodies::Bridge;
use sc_core::combat::data::DrillData;
use sc_core::combat::{Command, Station, TICK_HZ};
use sc_core::names::NameData;
use sc_core::rng::Rng;
use sc_net::bot::Bot;
use sc_net::session::Session;
use sc_net::transport::Impair;
use sc_server::{DrillServer, ServerConfig};

/// An NPC crew member at one station.
pub struct Npc {
    /// The station they hold.
    pub station: Station,
    /// "Lt. Dal Varen".
    pub name: String,
    /// Their client session, over loopback.
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
}

/// The drill server and the NPC crew of a single-player game.
pub struct SoloHost {
    server: DrillServer,
    clock: FixedClock,
    /// The player's own station.
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
    /// Host the drill on loopback with a crew for every station of the roster but `seat`, named from `names` and the
    /// session's `seed`.
    pub fn start(
        data: &DrillData,
        seat: Station,
        names: &NameData,
        seed: u64,
        bridge: Option<Bridge>,
    ) -> Result<Self, String> {
        let lo = "127.0.0.1".parse().map_err(|e| format!("{e}"))?;
        let listen: SocketAddr = "127.0.0.1:0".parse().map_err(|e| format!("{e}"))?;
        let cfg = ServerConfig { listen, host_ip: lo, impair: Impair::default(), seed, bridge };
        let server = DrillServer::start(data.clone(), &cfg).map_err(|e| format!("single player: {e}"))?;
        let rank = names.ranks.iter().find(|r| r.name == "Lieutenant").or(names.ranks.first()).map_or("", |r| &r.short);
        let (stations, _) = roster(data, seat);
        let mut crew = Vec::new();
        for (i, st) in stations.into_iter().filter(|s| *s != seat).enumerate() {
            let name = format!("{rank} {}", names.generate(&mut Rng::for_purpose(seed, i as u64, "npc")));
            let session = Session::connect(server.addr(), &name, true, Some(st), Impair::default(), seed ^ i as u64)?;
            crew.push(Npc { station: st, name, session, bot: Bot::new(data, st), auto: true });
        }
        println!(
            "sc-client: single player on {}: the player at {}, {} NPCs ({})",
            server.addr(),
            seat.name(),
            crew.len(),
            crew.iter().map(|n| format!("{} {}", n.station.name(), n.name)).collect::<Vec<_>>().join(", ")
        );
        Ok(Self { server, clock: FixedClock::new(TICK_HZ, 8), seat, crew })
    }

    /// Where the player's session connects.
    pub fn addr(&self) -> SocketAddr {
        self.server.addr()
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
            n.bot.manual = !n.auto;
            n.bot.drive(&mut n.session, data);
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
        let mut me = Session::connect(host.addr(), "Lt. Player", false, Some(seat), Impair::default(), 7).unwrap();
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
}
