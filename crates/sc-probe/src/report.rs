//! The probe's report: JSON for tools and a Markdown summary for people, naming the machine, the
//! GL driver, the resolution, the build, the repeats and the spread between them (CLAUDE.md 12).

use crate::{Options, Sample, Step};
use sc_client::platform::GlInfo;
use serde_json::{json, Value};
use std::path::Path;

/// Facts about the run, gathered once at the start.
pub struct RunInfo {
    /// `/proc/device-tree/model` on a Pi, else the CPU's model name.
    pub machine: String,
    /// Total RAM in MB.
    pub ram_mb: u64,
    /// The OS.
    pub os: String,
    /// SDL's video driver.
    pub video_driver: String,
    /// GL_VERSION.
    pub gl_version: String,
    /// GL_RENDERER.
    pub gl_renderer: String,
    /// The output size.
    pub output: (u32, u32),
    /// Frames measured and warm-up frames a step, repeats.
    pub frames: u32,
    /// Warm-up frames a step.
    pub warmup: u32,
    /// Repeats of the whole list.
    pub repeats: u32,
    /// Debug or release.
    pub build: &'static str,
    /// Resident memory before sokol_gfx was set up, after, and with the scenes' resources made.
    pub rss: (u64, u64, u64),
    /// The date, UTC, YYYY-MM-DD.
    pub date: String,
    /// Whether this can be a Pi measurement at all.
    pub is_pi: bool,
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Resident memory of this process in bytes (Linux; 0 elsewhere).
pub fn rss_bytes() -> u64 {
    read("/proc/self/statm").split_whitespace().nth(1).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0) * 4096
}

fn today() -> String {
    let days =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0)
            as i64;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

impl RunInfo {
    /// Gather the facts.
    pub fn collect(gl: &GlInfo, o: &Options, rss0: u64, rss1: u64, rss2: u64) -> Self {
        let model = read("/proc/device-tree/model").trim_end_matches('\0').trim().to_owned();
        let cpu = read("/proc/cpuinfo")
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .unwrap_or("")
            .trim()
            .to_owned();
        let ram_kb: u64 = read("/proc/meminfo")
            .lines()
            .find(|l| l.starts_with("MemTotal"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let os = read("/etc/os-release")
            .lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .map(|l| l[12..].trim_matches('"').to_owned())
            .unwrap_or_default();
        Self {
            is_pi: model.contains("Raspberry Pi 5"),
            machine: if model.is_empty() { cpu } else { model },
            ram_mb: ram_kb / 1024,
            os,
            video_driver: gl.video_driver.clone(),
            gl_version: gl.version.clone(),
            gl_renderer: gl.renderer.clone(),
            output: (o.width, o.height),
            frames: o.frames,
            warmup: o.warmup,
            repeats: o.repeats,
            build: if cfg!(debug_assertions) { "debug" } else { "release" },
            rss: (rss0, rss1, rss2),
            date: today(),
        }
    }
}

fn percentile(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let i = ((p / 100.0) * (s.len() - 1) as f64).round() as usize;
    s[i.min(s.len() - 1)]
}

/// The built report.
pub struct Report {
    json: Value,
    md: String,
}

impl Report {
    /// Summarise every step over its repeats.
    pub fn build(info: &RunInfo, steps: &[Step], samples: &[Vec<Sample>]) -> Self {
        let mut rows = Vec::new();
        let mut md = String::new();
        let warning = if info.is_pi {
            String::new()
        } else {
            format!(
                "**Not a Raspberry Pi 5.** This run is on {} through {}: it proves every scene draws, and its \
                 times say nothing about the Pi (CLAUDE.md 2, 12).\n\n",
                info.machine, info.gl_renderer
            )
        };
        md.push_str(&format!("# sc-probe, {}\n\n{warning}", info.date));
        md.push_str(&format!(
            "| Machine | RAM | OS | Video | GL | Output | 3D | Build | Frames a step | Warm-up | Repeats |\n| --- | ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |\n| {} | {} MB | {} | {} | {} ({}) | {} x {} | {} x {} | {} | {} | {} | {} |\n\n",
            info.machine, info.ram_mb, info.os, info.video_driver, info.gl_version, info.gl_renderer, info.output.0, info.output.1,
            crate::RENDER_3D.0, crate::RENDER_3D.1, info.build, info.frames, info.warmup, info.repeats
        ));
        md.push_str("| Scene | Step | Triangles | Draws | Frame p50 ms | p95 | p99 | Spread of p50 over repeats | CPU submit p50 ms | CPU a draw us |\n| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        for (step, reps) in steps.iter().zip(samples) {
            let all_frame: Vec<f64> = reps.iter().flat_map(|s| s.frame_ms.iter().copied()).collect();
            let all_submit: Vec<f64> = reps.iter().flat_map(|s| s.submit_ms.iter().copied()).collect();
            let p50s: Vec<f64> = reps.iter().map(|s| percentile(&s.frame_ms, 50.0)).filter(|x| x.is_finite()).collect();
            let spread = p50s.iter().copied().fold(f64::MIN, f64::max) - p50s.iter().copied().fold(f64::MAX, f64::min);
            let draws = reps.last().map(|s| s.draws).unwrap_or(0);
            let tris = reps.last().map(|s| s.triangles).unwrap_or(0);
            let sub50 = percentile(&all_submit, 50.0);
            let per_draw_us = if draws > 0 { sub50 * 1e3 / f64::from(draws) } else { f64::NAN };
            let (f50, f95, f99) =
                (percentile(&all_frame, 50.0), percentile(&all_frame, 95.0), percentile(&all_frame, 99.0));
            md.push_str(&format!(
                "| {} | {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.3} | {:.1} |\n",
                step.scene(),
                step.label(),
                tris,
                draws,
                f50,
                f95,
                f99,
                spread,
                sub50,
                per_draw_us
            ));
            rows.push(json!({
                "scene": step.scene(), "step": step.label(), "triangles": tris, "draws": draws,
                "frame_ms": { "p50": f50, "p95": f95, "p99": f99, "p50_spread_over_repeats": spread, "p50_per_repeat": p50s },
                "cpu_submit_ms": { "p50": sub50, "p95": percentile(&all_submit, 95.0) }, "cpu_per_draw_us": per_draw_us,
            }));
        }
        let mb = |b: u64| b as f64 / 1_048_576.0;
        md.push_str(&format!(
            "\nScene 8, sokol_gfx's memory: resident {:.1} MB before setup, {:.1} MB after (pools at `data/engine/render.json`'s sizes, so \
             {:.2} MB for sokol_gfx and the context's first allocations), {:.1} MB with every scene's meshes, textures and targets made.\n",
            mb(info.rss.0), mb(info.rss.1), mb(info.rss.1.saturating_sub(info.rss.0)), mb(info.rss.2)
        ));
        md.push_str(
            "\nNot yet measured: scene 2 with instancing (the instanced program comes with the projectiles), scene 3 with a lightmap \
             fetch (none planned, `light-baking`), scene 5 (UI: the egui painter), scene 6 (memory with the Tern's decks: `deckc`), \
             scene 7 (the ship systems), scene 9 (the browser build).\n",
        );
        let json = json!({
            "schema": "starcrew.probe-report/1", "date": info.date, "is_pi5": info.is_pi,
            "machine": info.machine, "ram_mb": info.ram_mb, "os": info.os, "video_driver": info.video_driver,
            "gl_version": info.gl_version, "gl_renderer": info.gl_renderer,
            "output_px": [info.output.0, info.output.1], "render_3d_px": [crate::RENDER_3D.0, crate::RENDER_3D.1],
            "build": info.build, "frames_per_step": info.frames, "warmup_frames": info.warmup, "repeats": info.repeats,
            "rss_bytes": { "before_setup": info.rss.0, "after_setup": info.rss.1, "with_scenes": info.rss.2 },
            "steps": rows,
        });
        Self { json, md }
    }

    /// The Markdown summary.
    pub fn markdown(&self) -> &str {
        &self.md
    }

    /// Write `report.json` and `report.md` into `dir`.
    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(
            dir.join("report.json"),
            serde_json::to_string_pretty(&self.json).map_err(std::io::Error::other)? + "\n",
        )?;
        std::fs::write(dir.join("report.md"), &self.md)
    }
}
