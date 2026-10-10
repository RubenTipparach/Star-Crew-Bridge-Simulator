//! `sc-net`: the messages a client and the server exchange, and the WebRTC data channel transport
//! (openspec/changes/netcode-and-sessions; engine-stack design section 3; the first slice is
//! openspec/changes/coop-drill design section 4).
//!
//! It holds no gameplay rule: messages carry intents and state that `sc-core` defines. sc-net depends on sc-core,
//! never the other way.

pub mod link;
pub mod msg;
pub mod transport;
