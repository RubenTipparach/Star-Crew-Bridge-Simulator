//! The drill's messages and their binary codec (openspec/changes/coop-drill design 4.1; netcode-and-sessions
//! section 2).
//!
//! One implementation for both ends, so the server and the client cannot disagree on a layout (CLAUDE.md 6.6:
//! one schema, validated at the edge). Little-endian. Every float is checked finite and in range, every count
//! against its cap and every string against its length on decode; a message that fails is an error, which the
//! caller drops and counts, never applies.

use glam::{DQuat, DVec3};
use sc_core::combat::bodies::Posture;
use sc_core::combat::{self, Command, HelmMode, Outcome, Phase, Refusal, Station, TubeState};

/// The protocol's version; a Hello with another is refused.
pub const PROTOCOL: u16 = 2;
/// The longest unreliable message, so it is one SCTP chunk in one datagram (netcode-and-sessions section 2).
pub const MAX_UNRELIABLE: usize = 1200;
/// The longest string field, in bytes.
pub const MAX_STRING_BYTES: usize = 96;

/// Why a message was refused on decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Builds a message.
#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f64) {
        self.0.extend_from_slice(&(v as f32).to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn v3_64(&mut self, v: DVec3) {
        self.f64(v.x);
        self.f64(v.y);
        self.f64(v.z);
    }
    fn v3_32(&mut self, v: DVec3) {
        self.f32(v.x);
        self.f32(v.y);
        self.f32(v.z);
    }
    fn quat(&mut self, q: DQuat) {
        for c in [q.x, q.y, q.z, q.w] {
            self.f32(c);
        }
    }
    fn str(&mut self, s: &str) {
        let mut end = s.len().min(MAX_STRING_BYTES);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        self.u8(end as u8);
        self.0.extend_from_slice(&s.as_bytes()[..end]);
    }
}

/// Reads a message, checking as it goes.
pub struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// A reader over `b`.
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.at.checked_add(n).ok_or(DecodeError("truncated"))?;
        let s = self.b.get(self.at..end).ok_or(DecodeError("truncated"))?;
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two bytes")))
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("eight bytes")))
    }
    fn f32(&mut self, lo: f64, hi: f64) -> Result<f64, DecodeError> {
        let v = f64::from(f32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")));
        if !v.is_finite() {
            return Err(DecodeError("a number is not finite"));
        }
        if v < lo || v > hi {
            return Err(DecodeError("a number is out of range"));
        }
        Ok(v)
    }
    fn f64(&mut self, lo: f64, hi: f64) -> Result<f64, DecodeError> {
        let v = f64::from_le_bytes(self.take(8)?.try_into().expect("eight bytes"));
        if !v.is_finite() {
            return Err(DecodeError("a number is not finite"));
        }
        if v < lo || v > hi {
            return Err(DecodeError("a number is out of range"));
        }
        Ok(v)
    }
    fn v3_64(&mut self, lim: f64) -> Result<DVec3, DecodeError> {
        Ok(DVec3::new(self.f64(-lim, lim)?, self.f64(-lim, lim)?, self.f64(-lim, lim)?))
    }
    fn v3_32(&mut self, lim: f64) -> Result<DVec3, DecodeError> {
        Ok(DVec3::new(self.f32(-lim, lim)?, self.f32(-lim, lim)?, self.f32(-lim, lim)?))
    }
    fn quat(&mut self) -> Result<DQuat, DecodeError> {
        let q = DQuat::from_xyzw(
            self.f32(-1.01, 1.01)?,
            self.f32(-1.01, 1.01)?,
            self.f32(-1.01, 1.01)?,
            self.f32(-1.01, 1.01)?,
        );
        // An attitude must be a rotation (flight-and-navigation 6a rejects a length off by more than 0.001).
        if (q.length() - 1.0).abs() > 0.001 {
            return Err(DecodeError("an attitude is not a unit quaternion"));
        }
        Ok(q.normalize())
    }
    fn bool(&mut self) -> Result<bool, DecodeError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError("a flag is not 0 or 1")),
        }
    }
    fn str(&mut self, max_chars: usize) -> Result<String, DecodeError> {
        let n = usize::from(self.u8()?);
        if n > MAX_STRING_BYTES {
            return Err(DecodeError("a string is too long"));
        }
        let s = std::str::from_utf8(self.take(n)?).map_err(|_| DecodeError("a string is not UTF-8"))?;
        if s.chars().count() > max_chars || s.chars().any(char::is_control) {
            return Err(DecodeError("a string is too long or has control characters"));
        }
        Ok(s.to_owned())
    }
    fn count(&mut self, cap: usize) -> Result<usize, DecodeError> {
        let n = usize::from(self.u8()?);
        if n > cap {
            return Err(DecodeError("a count is over its cap"));
        }
        Ok(n)
    }
    /// Every byte was read.
    fn done(&self) -> Result<(), DecodeError> {
        if self.at == self.b.len() {
            Ok(())
        } else {
            Err(DecodeError("trailing bytes"))
        }
    }
}

/// The largest distance a position may hold, in metres (the drill's volume, with margin).
const POS_LIM: f64 = 1.0e6;
/// The largest speed, in m/s.
const VEL_LIM: f64 = 1.0e5;

/// The header on the unreliable channels (netcode-and-sessions section 2): our sequence, the newest sequence we
/// have from the other end, and 32 bits saying which of the 32 before that arrived.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Header {
    /// This message's sequence.
    pub seq: u16,
    /// The newest sequence received from the other end.
    pub ack: u16,
    /// Bit `i` set: `ack - 1 - i` arrived too.
    pub ack_bits: u32,
}

impl Header {
    fn write(&self, w: &mut Writer) {
        w.u16(self.seq);
        w.u16(self.ack);
        w.u32(self.ack_bits);
    }
    fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(Self { seq: r.u16()?, ack: r.u16()?, ack_bits: r.u32()? })
    }
}

/// A player as the crew list shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct CrewEntry {
    /// Slot.
    pub slot: u8,
    /// Name.
    pub name: String,
    /// The station held.
    pub station: Option<Station>,
    /// Ready.
    pub ready: bool,
    /// A bot.
    pub bot: bool,
}

/// A client's message on the command channel.
#[derive(Clone, Debug, PartialEq)]
pub enum ClientMsg {
    /// The first message: the protocol, the officer, whether a bot plays, and the station wanted.
    Hello {
        /// [`PROTOCOL`].
        protocol: u16,
        /// The officer's name.
        name: String,
        /// A bot client.
        bot: bool,
        /// The station to claim at once.
        station: Option<Station>,
    },
    /// Take a station.
    Claim(Station),
    /// Ready, or not.
    Ready(bool),
    /// A console command.
    Command(Command),
    /// A round-trip probe; the server echoes it.
    Ping(u32),
}

/// A client's message on the input channel: the helm's stick, newest wins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StickMsg {
    /// The header.
    pub header: Header,
    /// The command it carries (always a [`Command::Stick`]).
    pub stick: Command,
}

/// A ship as a snapshot carries it.
#[derive(Clone, Debug, PartialEq)]
pub struct ShipSnap {
    /// Id.
    pub id: u16,
    /// In the fight.
    pub active: bool,
    /// Not destroyed.
    pub alive: bool,
    /// Position, system frame.
    pub pos: DVec3,
    /// Attitude.
    pub rot: DQuat,
    /// Velocity.
    pub vel: DVec3,
    /// Body rates, rad/s.
    pub rates: DVec3,
    /// Speed set point, m/s.
    pub speed_set: f64,
    /// Helm mode.
    pub helm_mode: HelmMode,
    /// Shield faces, MJ.
    pub faces: [f64; 6],
    /// Preset.
    pub preset: u8,
    /// Hull, MJ.
    pub hull: f64,
    /// Turrets: aim (unit, system frame), bearing, hit chance (0-1), capacitor share (0-1).
    pub turrets: Vec<TurretSnap>,
    /// Tubes: state and seconds left.
    pub tubes: Vec<(TubeState, f64)>,
    /// Magazine.
    pub magazine: u8,
    /// Designated target.
    pub lock_target: Option<u16>,
    /// The lock's progress (0-1).
    pub lock_frac: f64,
    /// Weapons free.
    pub weapons_free: bool,
}

/// A turret as a snapshot carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurretSnap {
    /// Where it points, a unit vector, system frame.
    pub aim: DVec3,
    /// Its arc holds the target.
    pub bearing: bool,
    /// Hit chance now (0-1).
    pub hit_chance: f64,
    /// Capacitor charge as a share of full (0-1).
    pub charge: f64,
}

/// A missile in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissileSnap {
    /// Id.
    pub id: u16,
    /// Position.
    pub pos: DVec3,
    /// Velocity.
    pub vel: DVec3,
}

/// The drill's numbers, for the debrief.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatsSnap {
    /// Bolts the Tern fired.
    pub tern_shots: u32,
    /// Hits.
    pub tern_hits: u32,
    /// The enemy's bolts.
    pub enemy_shots: u32,
    /// Hits.
    pub enemy_hits: u32,
    /// Missiles fired.
    pub missiles_fired: u32,
    /// Missiles that did damage.
    pub missile_hits: u32,
    /// Damage dealt, MJ.
    pub damage_dealt_mj: f64,
    /// Damage taken, MJ.
    pub damage_taken_mj: f64,
    /// Seconds of Engage.
    pub engage_s: f64,
}

/// The server's state at a tick (design 4.1): full, not a delta.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// The header.
    pub header: Header,
    /// The server's tick.
    pub tick: u64,
    /// The round.
    pub round: u32,
    /// The phase.
    pub phase: Phase,
    /// Seconds in the phase.
    pub phase_s: f64,
    /// The outcome.
    pub outcome: Outcome,
    /// The Tern, then the enemy.
    pub ships: Vec<ShipSnap>,
    /// Missiles.
    pub missiles: Vec<MissileSnap>,
    /// This round's numbers.
    pub stats: StatsSnap,
    /// The crew's bodies on the bridge (coop-drill design 9); empty in a drill with no bridge.
    pub bodies: Vec<BodySnap>,
}

/// A crew member's body in a snapshot: 10 bytes (coop-drill design 9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySnap {
    /// The player's slot.
    pub slot: u8,
    /// Where, ship frame, metres (sent in centimetres).
    pub pos: [f32; 3],
    /// The way it faces, radians about +Y (sent as 256 a turn).
    pub yaw: f32,
    /// What it is doing.
    pub posture: Posture,
    /// The station it is walking to.
    pub going: Option<Station>,
}

/// The server's messages on the command channel.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerMsg {
    /// The answer to Hello: the protocol, the slot given, and the mission's id.
    Welcome {
        /// [`PROTOCOL`].
        protocol: u16,
        /// The player's slot.
        slot: u8,
        /// The mission's id.
        mission: String,
    },
    /// A request refused, with the reason.
    Refused(Refusal),
    /// The crew list, sent whenever it changes.
    Crew(Vec<CrewEntry>),
    /// A bolt left a turret.
    Fired {
        /// Bolt id.
        bolt: u32,
        /// The ship.
        owner: u16,
        /// Where.
        pos: DVec3,
        /// Velocity.
        vel: DVec3,
    },
    /// A bolt struck.
    Hit {
        /// Bolt id.
        bolt: u32,
        /// The ship struck.
        target: u16,
        /// The face.
        face: u8,
        /// Damage, MJ.
        damage_mj: f64,
    },
    /// A missile left a tube.
    Launched {
        /// Id.
        missile: u16,
    },
    /// A missile exploded.
    Detonated {
        /// Id.
        missile: u16,
        /// Where.
        pos: DVec3,
        /// Damage, MJ.
        damage_mj: f64,
    },
    /// A ship was destroyed.
    Destroyed(u16),
    /// The phase changed.
    Phase(Phase, Outcome),
    /// The echo of a Ping.
    Pong(u32),
}

const MAX_SHIPS: usize = 2;
const MAX_TURRETS: usize = 16;
const MAX_TUBES: usize = 8;

fn station_byte(s: Option<Station>) -> u8 {
    s.map(|s| s as u8).unwrap_or(255)
}

fn read_station(r: &mut Reader) -> Result<Option<Station>, DecodeError> {
    match r.u8()? {
        255 => Ok(None),
        v => Station::from_u8(v).map(Some).ok_or(DecodeError("no such station")),
    }
}

fn write_command(w: &mut Writer, c: &Command) {
    match *c {
        Command::Stick { speed_set_mps, yaw, pitch, roll } => {
            w.u8(1);
            w.f32(speed_set_mps);
            w.f32(yaw);
            w.f32(pitch);
            w.f32(roll);
        }
        Command::Helm(m) => {
            w.u8(2);
            w.u8(m as u8);
        }
        Command::AllStop => w.u8(3),
        Command::Lock(t) => {
            w.u8(4);
            w.u16(t.unwrap_or(0));
        }
        Command::WeaponsFree(f) => {
            w.u8(5);
            w.u8(u8::from(f));
        }
        Command::Preset(i) => {
            w.u8(6);
            w.u8(i);
        }
        Command::Load(i) => {
            w.u8(7);
            w.u8(i);
        }
        Command::Fire(i) => {
            w.u8(8);
            w.u8(i);
        }
    }
}

fn read_command(r: &mut Reader) -> Result<Command, DecodeError> {
    let c = match r.u8()? {
        1 => Command::Stick {
            speed_set_mps: r.f32(-1000.0, 5000.0)?,
            yaw: r.f32(-1.0, 1.0)?,
            pitch: r.f32(-1.0, 1.0)?,
            roll: r.f32(-1.0, 1.0)?,
        },
        2 => Command::Helm(HelmMode::from_u8(r.u8()?).ok_or(DecodeError("no such helm mode"))?),
        3 => Command::AllStop,
        4 => Command::Lock(match r.u16()? {
            0 => None,
            id => Some(id),
        }),
        5 => Command::WeaponsFree(r.bool()?),
        6 => Command::Preset(r.u8()?),
        7 => Command::Load(r.u8()?),
        8 => Command::Fire(r.u8()?),
        _ => return Err(DecodeError("no such command")),
    };
    Ok(c)
}

impl ClientMsg {
    /// Encode.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        match self {
            Self::Hello { protocol, name, bot, station } => {
                w.u8(1);
                w.u16(*protocol);
                w.str(name);
                w.u8(u8::from(*bot));
                w.u8(station_byte(*station));
            }
            Self::Claim(s) => {
                w.u8(2);
                w.u8(*s as u8);
            }
            Self::Ready(r) => {
                w.u8(3);
                w.u8(u8::from(*r));
            }
            Self::Command(c) => {
                w.u8(4);
                write_command(&mut w, c);
            }
            Self::Ping(t) => {
                w.u8(5);
                w.u32(*t);
            }
        }
        w.0
    }

    /// Decode and validate.
    pub fn decode(b: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(b);
        let m = match r.u8()? {
            1 => Self::Hello {
                protocol: r.u16()?,
                name: r.str(combat::MAX_NAME_CHARS)?,
                bot: r.bool()?,
                station: read_station(&mut r)?,
            },
            2 => Self::Claim(read_station(&mut r)?.ok_or(DecodeError("claim needs a station"))?),
            3 => Self::Ready(r.bool()?),
            4 => Self::Command(read_command(&mut r)?),
            5 => Self::Ping(r.u32()?),
            _ => return Err(DecodeError("no such client message")),
        };
        r.done()?;
        Ok(m)
    }
}

impl StickMsg {
    /// Encode.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        self.header.write(&mut w);
        write_command(&mut w, &self.stick);
        w.0
    }

    /// Decode and validate; anything but a stick is refused.
    pub fn decode(b: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(b);
        let header = Header::read(&mut r)?;
        let stick = read_command(&mut r)?;
        if !matches!(stick, Command::Stick { .. }) {
            return Err(DecodeError("the input channel carries only the stick"));
        }
        r.done()?;
        Ok(Self { header, stick })
    }
}

impl Snapshot {
    /// Encode. It stays under [`MAX_UNRELIABLE`] for the drill's two ships (a test checks).
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        self.header.write(&mut w);
        w.u64(self.tick);
        w.u32(self.round);
        w.u8(self.phase as u8);
        w.f32(self.phase_s);
        w.u8(self.outcome as u8);
        w.u8(self.ships.len() as u8);
        for s in &self.ships {
            w.u16(s.id);
            w.u8(u8::from(s.active) | u8::from(s.alive) << 1 | u8::from(s.weapons_free) << 2);
            w.v3_64(s.pos);
            w.quat(s.rot);
            w.v3_32(s.vel);
            w.v3_32(s.rates);
            w.f32(s.speed_set);
            w.u8(s.helm_mode as u8);
            for f in s.faces {
                w.f32(f);
            }
            w.u8(s.preset);
            w.f32(s.hull);
            w.u8(s.turrets.len() as u8);
            for t in &s.turrets {
                w.v3_32(t.aim);
                w.u8(u8::from(t.bearing));
                w.u8((t.hit_chance.clamp(0.0, 1.0) * 200.0).round() as u8);
                w.u8((t.charge.clamp(0.0, 1.0) * 200.0).round() as u8);
            }
            w.u8(s.tubes.len() as u8);
            for (st, timer) in &s.tubes {
                w.u8(*st as u8);
                w.f32(*timer);
            }
            w.u8(s.magazine);
            w.u16(s.lock_target.unwrap_or(0));
            w.u8((s.lock_frac.clamp(0.0, 1.0) * 200.0).round() as u8);
        }
        w.u8(self.missiles.len() as u8);
        for m in &self.missiles {
            w.u16(m.id);
            w.v3_64(m.pos);
            w.v3_32(m.vel);
        }
        let st = &self.stats;
        for v in [st.tern_shots, st.tern_hits, st.enemy_shots, st.enemy_hits, st.missiles_fired, st.missile_hits] {
            w.u32(v);
        }
        w.f32(st.damage_dealt_mj);
        w.f32(st.damage_taken_mj);
        w.f32(st.engage_s);
        w.u8(self.bodies.len() as u8);
        for b in &self.bodies {
            w.u8(b.slot);
            for v in b.pos {
                w.u16(((v * 100.0).round().clamp(-32768.0, 32767.0) as i16) as u16);
            }
            w.u8((b.yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 256.0) as u32 as u8);
            w.u8(b.posture as u8);
            w.u8(station_byte(b.going));
        }
        w.0
    }

    /// Decode and validate.
    pub fn decode(b: &[u8]) -> Result<Self, DecodeError> {
        if b.len() > MAX_UNRELIABLE {
            return Err(DecodeError("a snapshot is over the datagram size"));
        }
        let mut r = Reader::new(b);
        let header = Header::read(&mut r)?;
        let tick = r.u64()?;
        let round = r.u32()?;
        let phase = Phase::from_u8(r.u8()?).ok_or(DecodeError("no such phase"))?;
        let phase_s = r.f32(0.0, 1e7)?;
        let outcome = Outcome::from_u8(r.u8()?).ok_or(DecodeError("no such outcome"))?;
        let n = r.count(MAX_SHIPS)?;
        let mut ships = Vec::with_capacity(n);
        for _ in 0..n {
            let id = r.u16()?;
            let flags = r.u8()?;
            if flags > 7 {
                return Err(DecodeError("unknown ship flags"));
            }
            let pos = r.v3_64(POS_LIM)?;
            let rot = r.quat()?;
            let vel = r.v3_32(VEL_LIM)?;
            let rates = r.v3_32(100.0)?;
            let speed_set = r.f32(-VEL_LIM, VEL_LIM)?;
            let helm_mode = HelmMode::from_u8(r.u8()?).ok_or(DecodeError("no such helm mode"))?;
            let mut faces = [0.0; 6];
            for f in &mut faces {
                *f = r.f32(0.0, 1e6)?;
            }
            let preset = r.u8()?;
            let hull = r.f32(0.0, 1e7)?;
            let nt = r.count(MAX_TURRETS)?;
            let mut turrets = Vec::with_capacity(nt);
            for _ in 0..nt {
                let aim = r.v3_32(1.01)?;
                let bearing = r.bool()?;
                let hc = r.u8()?;
                let ch = r.u8()?;
                if hc > 200 || ch > 200 {
                    return Err(DecodeError("a share is over 100 %"));
                }
                turrets.push(TurretSnap {
                    aim,
                    bearing,
                    hit_chance: f64::from(hc) / 200.0,
                    charge: f64::from(ch) / 200.0,
                });
            }
            let nb = r.count(MAX_TUBES)?;
            let mut tubes = Vec::with_capacity(nb);
            for _ in 0..nb {
                let st = TubeState::from_u8(r.u8()?).ok_or(DecodeError("no such tube state"))?;
                tubes.push((st, r.f32(0.0, 3600.0)?));
            }
            let magazine = r.u8()?;
            let lock_target = match r.u16()? {
                0 => None,
                v => Some(v),
            };
            let lf = r.u8()?;
            if lf > 200 {
                return Err(DecodeError("a share is over 100 %"));
            }
            ships.push(ShipSnap {
                id,
                active: flags & 1 != 0,
                alive: flags & 2 != 0,
                weapons_free: flags & 4 != 0,
                pos,
                rot,
                vel,
                rates,
                speed_set,
                helm_mode,
                faces,
                preset,
                hull,
                turrets,
                tubes,
                magazine,
                lock_target,
                lock_frac: f64::from(lf) / 200.0,
            });
        }
        let nm = r.count(combat::MAX_MISSILES)?;
        let mut missiles = Vec::with_capacity(nm);
        for _ in 0..nm {
            missiles.push(MissileSnap { id: r.u16()?, pos: r.v3_64(POS_LIM)?, vel: r.v3_32(VEL_LIM)? });
        }
        let mut c = [0u32; 6];
        for v in &mut c {
            *v = r.u32()?;
        }
        let stats = StatsSnap {
            tern_shots: c[0],
            tern_hits: c[1],
            enemy_shots: c[2],
            enemy_hits: c[3],
            missiles_fired: c[4],
            missile_hits: c[5],
            damage_dealt_mj: r.f32(0.0, 1e9)?,
            damage_taken_mj: r.f32(0.0, 1e9)?,
            engage_s: r.f32(0.0, 1e7)?,
        };
        let nb = r.count(combat::MAX_PLAYERS)?;
        let mut bodies = Vec::with_capacity(nb);
        for _ in 0..nb {
            let slot = r.u8()?;
            if usize::from(slot) >= combat::MAX_PLAYERS {
                return Err(DecodeError("no such slot"));
            }
            let mut pos = [0.0f32; 3];
            for v in &mut pos {
                *v = f32::from(r.u16()? as i16) / 100.0;
            }
            let yaw = f32::from(r.u8()?) / 256.0 * std::f32::consts::TAU;
            let posture = Posture::from_u8(r.u8()?).ok_or(DecodeError("no such posture"))?;
            let going = read_station(&mut r)?;
            bodies.push(BodySnap { slot, pos, yaw, posture, going });
        }
        r.done()?;
        Ok(Self { header, tick, round, phase, phase_s, outcome, ships, missiles, stats, bodies })
    }

    /// The snapshot of a drill (the header is the caller's, per client).
    pub fn of(d: &combat::Drill) -> Self {
        let lock_time = [d.data.tern_combat.lock.time_s, d.data.enemy.combat.lock.time_s];
        let caps =
            [d.data.gun(&d.data.tern_combat.gun).capacitor_mj, d.data.gun(&d.data.enemy.combat.gun).capacitor_mj];
        let ships = d
            .ships
            .iter()
            .enumerate()
            .map(|(i, s)| ShipSnap {
                id: s.id,
                active: s.active,
                alive: s.alive,
                pos: s.pos,
                rot: s.rot,
                vel: s.vel,
                rates: s.rates,
                speed_set: s.speed_set,
                helm_mode: s.helm_mode,
                faces: s.faces,
                preset: s.preset,
                hull: s.hull.max(0.0),
                turrets: s
                    .turrets
                    .iter()
                    .map(|t| TurretSnap {
                        aim: t.aim,
                        bearing: t.bearing,
                        hit_chance: t.hit_chance,
                        charge: t.capacitor_mj / caps[i].max(1e-9),
                    })
                    .collect(),
                tubes: s.tubes.iter().map(|t| (t.state, t.timer_s.max(0.0))).collect(),
                magazine: s.magazine.min(255) as u8,
                lock_target: s.lock_target,
                lock_frac: if lock_time[i] > 0.0 { (s.lock_s / lock_time[i]).min(1.0) } else { 1.0 },
                weapons_free: s.weapons_free,
            })
            .collect();
        let st = &d.stats;
        Self {
            header: Header::default(),
            tick: d.tick,
            round: d.round,
            phase: d.phase,
            phase_s: d.phase_s,
            outcome: d.outcome,
            ships,
            missiles: d.missiles.iter().map(|m| MissileSnap { id: m.id, pos: m.pos, vel: m.vel }).collect(),
            bodies: d
                .bodies
                .iter()
                .map(|b| BodySnap { slot: b.slot, pos: b.pos, yaw: b.yaw, posture: b.posture, going: b.walking_to() })
                .collect(),
            stats: StatsSnap {
                tern_shots: st.tern_shots,
                tern_hits: st.tern_hits,
                enemy_shots: st.enemy_shots,
                enemy_hits: st.enemy_hits,
                missiles_fired: st.missiles_fired,
                missile_hits: st.missile_hits,
                damage_dealt_mj: st.damage_dealt_mj,
                damage_taken_mj: st.damage_taken_mj,
                engage_s: st.engage_s,
            },
        }
    }
}

impl ServerMsg {
    /// Encode.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        match self {
            Self::Welcome { protocol, slot, mission } => {
                w.u8(1);
                w.u16(*protocol);
                w.u8(*slot);
                w.str(mission);
            }
            Self::Refused(r) => {
                w.u8(2);
                w.u8(*r as u8);
            }
            Self::Crew(list) => {
                w.u8(3);
                w.u8(list.len() as u8);
                for c in list {
                    w.u8(c.slot);
                    w.str(&c.name);
                    w.u8(station_byte(c.station));
                    w.u8(u8::from(c.ready) | u8::from(c.bot) << 1);
                }
            }
            Self::Fired { bolt, owner, pos, vel } => {
                w.u8(4);
                w.u32(*bolt);
                w.u16(*owner);
                w.v3_64(*pos);
                w.v3_32(*vel);
            }
            Self::Hit { bolt, target, face, damage_mj } => {
                w.u8(5);
                w.u32(*bolt);
                w.u16(*target);
                w.u8(*face);
                w.f32(*damage_mj);
            }
            Self::Launched { missile } => {
                w.u8(6);
                w.u16(*missile);
            }
            Self::Detonated { missile, pos, damage_mj } => {
                w.u8(7);
                w.u16(*missile);
                w.v3_64(*pos);
                w.f32(*damage_mj);
            }
            Self::Destroyed(id) => {
                w.u8(8);
                w.u16(*id);
            }
            Self::Phase(p, o) => {
                w.u8(9);
                w.u8(*p as u8);
                w.u8(*o as u8);
            }
            Self::Pong(t) => {
                w.u8(10);
                w.u32(*t);
            }
        }
        w.0
    }

    /// Decode and validate.
    pub fn decode(b: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(b);
        let m = match r.u8()? {
            1 => Self::Welcome { protocol: r.u16()?, slot: r.u8()?, mission: r.str(64)? },
            2 => Self::Refused(Refusal::from_u8(r.u8()?).ok_or(DecodeError("no such refusal"))?),
            3 => {
                let n = r.count(combat::MAX_PLAYERS)?;
                let mut list = Vec::with_capacity(n);
                for _ in 0..n {
                    let slot = r.u8()?;
                    let name = r.str(combat::MAX_NAME_CHARS)?;
                    let station = read_station(&mut r)?;
                    let f = r.u8()?;
                    if f > 3 {
                        return Err(DecodeError("unknown crew flags"));
                    }
                    list.push(CrewEntry { slot, name, station, ready: f & 1 != 0, bot: f & 2 != 0 });
                }
                Self::Crew(list)
            }
            4 => Self::Fired { bolt: r.u32()?, owner: r.u16()?, pos: r.v3_64(POS_LIM)?, vel: r.v3_32(VEL_LIM)? },
            5 => Self::Hit { bolt: r.u32()?, target: r.u16()?, face: r.u8()?, damage_mj: r.f32(0.0, 1e6)? },
            6 => Self::Launched { missile: r.u16()? },
            7 => Self::Detonated { missile: r.u16()?, pos: r.v3_64(POS_LIM)?, damage_mj: r.f32(0.0, 1e6)? },
            8 => Self::Destroyed(r.u16()?),
            9 => Self::Phase(
                Phase::from_u8(r.u8()?).ok_or(DecodeError("no such phase"))?,
                Outcome::from_u8(r.u8()?).ok_or(DecodeError("no such outcome"))?,
            ),
            10 => Self::Pong(r.u32()?),
            _ => return Err(DecodeError("no such server message")),
        };
        if let Self::Hit { face, .. } = m {
            if face > 5 {
                return Err(DecodeError("no such face"));
            }
        }
        r.done()?;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::combat::data::DrillData;
    use sc_core::combat::{Drill, Station};

    fn engaged() -> Drill {
        let mut d = Drill::new(DrillData::shipped(), 3);
        let a = d.join("A", false).unwrap();
        d.claim(a, Station::Helm).unwrap();
        d.set_ready(a, true).unwrap();
        while d.phase != Phase::Engage {
            d.step();
        }
        for _ in 0..300 {
            d.step();
        }
        d
    }

    #[test]
    fn a_snapshot_round_trips_and_fits_one_datagram() {
        let d = engaged();
        let mut s = Snapshot::of(&d);
        s.header = Header { seq: 7, ack: 3, ack_bits: 0b101 };
        let b = s.encode();
        assert!(b.len() <= MAX_UNRELIABLE, "a snapshot is {} bytes; it must fit one datagram", b.len());
        let back = Snapshot::decode(&b).unwrap();
        assert_eq!(back.tick, s.tick);
        assert_eq!(back.header, s.header);
        assert_eq!(back.ships.len(), 2);
        assert_eq!(back.ships[0].pos, s.ships[0].pos, "positions travel as f64");
        assert!((back.ships[1].hull - s.ships[1].hull).abs() < 1e-3);
    }

    #[test]
    fn every_client_message_round_trips() {
        let msgs = [
            ClientMsg::Hello {
                protocol: PROTOCOL,
                name: "Lt. Imra Voss".into(),
                bot: true,
                station: Some(Station::Tactical),
            },
            ClientMsg::Claim(Station::Helm),
            ClientMsg::Ready(true),
            ClientMsg::Command(Command::Fire(1)),
            ClientMsg::Command(Command::Lock(None)),
            ClientMsg::Command(Command::Stick { speed_set_mps: 120.0, yaw: 0.5, pitch: -0.25, roll: 0.0 }),
            ClientMsg::Ping(12345),
        ];
        for m in msgs {
            assert_eq!(ClientMsg::decode(&m.encode()).unwrap(), m);
        }
    }

    #[test]
    fn every_server_message_round_trips() {
        let msgs = [
            ServerMsg::Welcome { protocol: PROTOCOL, slot: 2, mission: "drill-hound".into() },
            ServerMsg::Refused(Refusal::OutOfCone),
            ServerMsg::Crew(vec![CrewEntry {
                slot: 0,
                name: "A".into(),
                station: Some(Station::Helm),
                ready: true,
                bot: false,
            }]),
            ServerMsg::Fired { bolt: 9, owner: 1, pos: DVec3::new(1.0, 2.0, 3.0), vel: DVec3::new(0.0, 0.0, 1500.0) },
            ServerMsg::Hit { bolt: 9, target: 2, face: 4, damage_mj: 1.25 },
            ServerMsg::Phase(Phase::Debrief, Outcome::Victory),
            ServerMsg::Pong(77),
        ];
        for m in msgs {
            assert_eq!(ServerMsg::decode(&m.encode()).unwrap(), m);
        }
    }

    #[test]
    fn a_non_finite_stick_is_dropped_on_arrival() {
        let m = StickMsg {
            header: Header::default(),
            stick: Command::Stick { speed_set_mps: 10.0, yaw: f64::NAN, pitch: 0.0, roll: 0.0 },
        };
        assert_eq!(StickMsg::decode(&m.encode()), Err(DecodeError("a number is not finite")));
    }

    #[test]
    fn a_truncated_or_padded_message_is_dropped() {
        let b = ClientMsg::Ping(5).encode();
        assert!(ClientMsg::decode(&b[..b.len() - 1]).is_err());
        let mut long = b.clone();
        long.push(0);
        assert_eq!(ClientMsg::decode(&long), Err(DecodeError("trailing bytes")));
    }

    #[test]
    fn a_crew_list_over_its_cap_is_dropped() {
        let mut b = vec![3u8, 200];
        b.extend_from_slice(&[0; 16]);
        assert_eq!(ServerMsg::decode(&b), Err(DecodeError("a count is over its cap")));
    }

    #[test]
    fn a_stick_out_of_range_is_dropped() {
        let m = StickMsg {
            header: Header::default(),
            stick: Command::Stick { speed_set_mps: 10.0, yaw: 2.0, pitch: 0.0, roll: 0.0 },
        };
        assert!(StickMsg::decode(&m.encode()).is_err());
    }
}
