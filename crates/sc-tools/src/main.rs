//! `sc-tools`: the offline tools (engine-stack design section 3): `deckc`, `meshc` and `bake` as
//! their changes are built, and the data validators now.
//!
//! `sc-tools check-data` loads every engine data file through `sc-core`'s one loader, so a file the
//! game would refuse at startup is refused here first, naming the file and the field.
//! `sc-tools deckc [ship]` compiles a ship's decks into `compiled/<ship>.deck` (see `deckc`).
//! `sc-tools names [count] [seed]` prints generated crew names (lobby design 2), to judge the table.
//! `sc-tools walk-report [ship]` measures the walk world's heap and a body's step time (see `walk_report`).

mod alloc;
mod deckc;
mod dock;
mod figure;
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
    Ok(files.len())
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
                "usage: sc-tools check-data [repository root] | sc-tools deckc [ship] | sc-tools names [count] [seed] | sc-tools walk-report [ship]"
            );
            ExitCode::from(2)
        }
    }
}
