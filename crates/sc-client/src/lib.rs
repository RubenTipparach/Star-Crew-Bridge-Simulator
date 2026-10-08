//! `sc-client`: the game client (openspec/changes/engine-stack, design section 3).
//!
//! The library holds the platform layer every windowed program of ours shares (the client, the
//! probe, the capture tool): SDL3's main callbacks, one OpenGL ES 3.0 context, the frame clock,
//! input events, and PNG captures of the output. The binary is the client itself. Gameplay rules
//! are never here: the client previews by calling `sc-core` and renders through `sc-render`.

pub mod capture;
pub mod platform;
