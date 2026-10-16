//! `sc-tools`: the offline tools (engine-stack design section 3): `deckc`, `meshc` and `bake` as
//! their changes are built, and the data validators now.
//!
//! `sc-tools check-data` loads every engine data file through `sc-core`'s one loader, so a file the
//! game would refuse at startup is refused here first, naming the file and the field.
//! `sc-tools deckc [ship]` compiles a ship's decks into `compiled/<ship>.deck` (see `deckc`).
//! `sc-tools names [count] [seed]` prints generated crew names (lobby design 2), to judge the table.
//! `sc-tools walk-report [ship]` measures the walk world's heap and a body's step time (see `walk_report`).
//! `sc-tools safety-points [ship]` places the ship's extinguisher brackets from its layout and compiled deck into
//! `data/ships/<ship>/kit.json` (see `safety_points`).

mod alloc;
mod deckc;
mod dock;
mod figure;
mod glb;
mod safety_points;
mod walk_report;

/// Every allocation is counted, for `walk-report`'s heap figures.
#[global_allocator]
static ALLOC: alloc::Counting = alloc::Counting;

use std::path::Path;
use std::process::ExitCode;

fn check_data(root: &Path) -> Result<usize, String> {
    let files = [
        ("data/engine/render.json", "render"),
        ("data/crew/walk.json", "walk"),
        ("data/space/exterior.json", "exterior"),
        ("data/crew/names.json", "names"),
        ("data/crew/company.json", "company"),
    ];
    for (rel, kind) in files {
        let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: cannot be read: {e}"))?;
        match kind {
            "render" => sc_core::data::parse::<sc_core::data::RenderConfig>(rel, &text).map(|_| ()),
            "walk" => sc_core::data::parse::<sc_core::walk::WalkData>(rel, &text).map(|_| ()),
            "company" => sc_core::data::parse::<sc_core::crew::CompanyData>(rel, &text).map(|_| ()),
            "names" => sc_core::data::parse::<sc_core::names::NameData>(rel, &text).map(|_| ()),
            "exterior" => sc_core::data::parse::<sc_core::exterior::ExteriorData>(rel, &text).map(|_| ()),
            _ => unreachable!("every listed file has a schema"),
        }
        .map_err(|e| e.to_string())?;
    }
    // The drill's files are cross-checked together: an id one names must exist in another.
    let read = |rel: &str| std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: cannot be read: {e}"));
    let drill = sc_core::combat::data::DRILL_FILES.map(read);
    let drill = drill.into_iter().collect::<Result<Vec<String>, String>>()?;
    let missions = ["data/missions/patrol-shoals.json", "data/missions/drill-hound.json"];
    let mut drill_data = None;
    for m in missions {
        let texts: [&str; sc_core::combat::data::DRILL_FILE_COUNT] = std::array::from_fn(|i| drill[i].as_str());
        drill_data = Some(sc_core::combat::data::DrillData::parse(texts, m, &read(m)?).map_err(|e| e.to_string())?);
    }
    // Engineering's schematic (power-routing 6.4): the board and the families, held to the power data.
    let pd = &drill_data.expect("a mission was checked").power;
    sc_core::schematic::PowerBoard::load(&read(sc_core::schematic::BOARD_FILE)?, pd).map_err(|e| e.to_string())?;
    sc_core::schematic::Families::load(&read(sc_core::schematic::FAMILIES_FILE)?, pd).map_err(|e| e.to_string())?;
    // The repair files: the rules, the kit's rates, the Tern's jobs, and each game the jobs name (repair-minigames 8).
    let mut ids: Vec<String> = std::fs::read_dir(root.join("data/repairs"))
        .map_err(|e| format!("data/repairs: cannot be read: {e}"))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".json").map(str::to_owned))
        .filter(|id| id != "rules")
        .collect();
    ids.sort();
    sc_core::repair::data::RepairData::load(&ids, read)?;
    // The skies (nebula-skies design 9): the stars' file, every generated sky against it, and the home system's sky
    // must be one of them.
    let starfield_text = read(sc_core::sky::STARFIELD_FILE)?;
    let starfield =
        sc_core::sky::Starfield::load(sc_core::sky::STARFIELD_FILE, &starfield_text).map_err(|e| e.to_string())?;
    let mut skies: Vec<String> = std::fs::read_dir(root.join("data/space/skies"))
        .map_err(|e| format!("data/space/skies: cannot be read: {e}"))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".json").map(str::to_owned))
        .filter(|id| id != "recipes")
        .collect();
    skies.sort();
    for id in &skies {
        let rel = sc_core::sky::sky_file(id);
        sc_core::sky::NebulaSky::load(&rel, &read(&rel)?, &starfield).map_err(|e| e.to_string())?;
    }
    let home: sc_core::system::SystemFile =
        sc_core::data::parse(sc_core::system::HOME_FILE, &read(sc_core::system::HOME_FILE)?)
            .map_err(|e| e.to_string())?;
    if !skies.contains(&home.sky) {
        return Err(format!("{}: sky: no sky named {:?} in data/space/skies", sc_core::system::HOME_FILE, home.sky));
    }
    Ok(files.len() + drill.len() + missions.len() + 2 + 3 + ids.len() + 1 + skies.len())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check-data") => {
            let root = Path::new(args.get(1).map(String::as_str).unwrap_or("."));
            match check_data(root) {
                Ok(n) => {
                    println!("ok: {n} engine data files valid");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("FAIL {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("deckc") => match deckc::run(Path::new("."), args.get(1).map(String::as_str).unwrap_or("tern")) {
            Ok(s) => {
                println!("deckc: {s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("deckc: {e}");
                ExitCode::FAILURE
            }
        },
        Some("names") => {
            let n: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(30);
            let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
            let text = std::fs::read_to_string("data/crew/names.json").unwrap_or_default();
            match sc_core::data::parse::<sc_core::names::NameData>("data/crew/names.json", &text) {
                Ok(d) => {
                    for slot in 0..n {
                        println!("{}", d.for_slot(seed, slot));
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("names: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("safety-points") => {
            match safety_points::run(Path::new("."), args.get(1).map(String::as_str).unwrap_or("tern")) {
                Ok(s) => {
                    println!("safety-points: {s}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("safety-points: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("walk-report") => {
            match walk_report::run(Path::new("."), args.get(1).map(String::as_str).unwrap_or("tern")) {
                Ok(s) => {
                    println!("{s}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("walk-report: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "usage: sc-tools check-data [repository root] | sc-tools deckc [ship] | sc-tools names [count] [seed] | sc-tools walk-report [ship] | sc-tools safety-points [ship]"
            );
            ExitCode::from(2)
        }
    }
}
