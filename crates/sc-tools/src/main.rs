//! `sc-tools`: the offline tools (engine-stack design section 3): `deckc`, `meshc` and `bake` as
//! their changes are built, and the data validators now.
//!
//! `sc-tools check-data` loads every engine data file through `sc-core`'s one loader, so a file the
//! game would refuse at startup is refused here first, naming the file and the field.

use std::path::Path;
use std::process::ExitCode;

fn check_data(root: &Path) -> Result<usize, String> {
    let files = [("data/engine/render.json", "render")];
    for (rel, kind) in files {
        let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: cannot be read: {e}"))?;
        match kind {
            "render" => sc_core::data::parse::<sc_core::data::RenderConfig>(rel, &text).map(|_| ()),
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
        _ => {
            eprintln!("usage: sc-tools check-data [repository root]");
            ExitCode::from(2)
        }
    }
}
