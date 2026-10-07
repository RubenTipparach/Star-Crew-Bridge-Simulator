//! `sc-net`: the messages a client and the server exchange, and the WebRTC data channel transport
//! (openspec/changes/netcode-and-sessions; engine-stack design section 3).
//!
//! It holds no gameplay rule: messages carry intents and state that `sc-core` defines. Nothing is
//! built here yet; the crate exists from the first commit so the dependency graph is the design's
//! (sc-net depends on sc-core, never the other way), and the netcode change fills it.
