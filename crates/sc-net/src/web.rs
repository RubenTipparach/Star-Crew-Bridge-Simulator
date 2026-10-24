//! The web build's WebRTC link (openspec/changes/matchmaker design 10.7): a join by code through the matchmaker on the
//! browser's own `RTCPeerConnection`, driven by `web/rtc.js` (linked with `--js-library`). A page cannot open the UDP
//! sockets the native transport's `str0m` needs, so in the browser `ClientNet` holds this link instead.
//!
//! It lives in `sc-net` beside the native and local links so the session sees the same three channels and the same
//! events whichever link carries them. The browser works asynchronously: [`WebLink::join`] returns at once, and
//! [`WebLink::poll`], called once a frame, reports the channels opening, the messages and the close, with its reason.

use crate::matchmaker::{normal_code, Matchmaker};
use crate::transport::{Chan, NetEvent};

/// What the browser said about a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Joining,
    Open,
    Closed,
}

/// The JavaScript's view of a link (`web/rtc.js`): 0 joining, 1 open, 2 closed.
fn state_of(code: i32) -> State {
    match code {
        0 => State::Joining,
        1 => State::Open,
        _ => State::Closed,
    }
}

/// The address a page fetches the matchmaker at: HTTPS on the default port (the deployed matchmaker, behind Fly's
/// TLS: an HTTPS page may not call plain HTTP), plain HTTP on any other port (a matchmaker on this machine, in a test).
pub fn page_url(m: &Matchmaker) -> String {
    if m.url.port == 80 {
        format!("https://{}", m.url.host)
    } else {
        format!("http://{}:{}", m.url.host, m.url.port)
    }
}

/// The channel a message from the JavaScript came on.
fn chan_of(i: i32) -> Option<Chan> {
    match i {
        0 => Some(Chan::Command),
        1 => Some(Chan::Input),
        2 => Some(Chan::Snapshot),
        _ => None,
    }
}

#[cfg(target_os = "emscripten")]
mod js {
    use std::ffi::c_char;
    unsafe extern "C" {
        pub fn sc_rtc_join(url: *const c_char, code: *const c_char, stun: *const c_char, protocol: u32) -> i32;
        pub fn sc_rtc_state(h: i32) -> i32;
        pub fn sc_rtc_reason(h: i32, buf: *mut u8, cap: i32) -> i32;
        pub fn sc_rtc_send(h: i32, chan: i32, data: *const u8, len: i32);
        pub fn sc_rtc_next(h: i32) -> i32;
        pub fn sc_rtc_take(h: i32, chan_out: *mut i32, buf: *mut u8, cap: i32) -> i32;
        pub fn sc_rtc_close(h: i32);
    }
}

/// One join through the browser.
pub struct WebLink {
    #[cfg_attr(not(target_os = "emscripten"), allow(dead_code))]
    handle: i32,
    seen: State,
    reason: Option<String>,
}

impl WebLink {
    /// Start joining the session holding `code` through `matchmaker`; the channels open later ([`NetEvent::Open`]).
    #[cfg(target_os = "emscripten")]
    pub fn join(matchmaker: &Matchmaker, code: &str) -> Result<Self, String> {
        use std::ffi::CString;
        let url = CString::new(page_url(matchmaker)).map_err(|e| e.to_string())?;
        let code = CString::new(normal_code(code)).map_err(|e| e.to_string())?;
        let stun = serde_json::to_string(&matchmaker.stun).map_err(|e| e.to_string())?;
        let stun = CString::new(stun).map_err(|e| e.to_string())?;
        // SAFETY: three NUL-terminated strings the call reads before it returns.
        let handle =
            unsafe { js::sc_rtc_join(url.as_ptr(), code.as_ptr(), stun.as_ptr(), u32::from(crate::msg::PROTOCOL)) };
        Ok(Self { handle, seen: State::Joining, reason: None })
    }

    /// Outside the browser there is no page to join with.
    #[cfg(not(target_os = "emscripten"))]
    pub fn join(_matchmaker: &Matchmaker, _code: &str) -> Result<Self, String> {
        let _ = normal_code;
        Err("the browser's WebRTC link exists only in the web build".into())
    }

    /// Everything since the last call: the channels opening, the messages, and the close.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        let now = self.state();
        if now == State::Open && self.seen == State::Joining {
            out.push(NetEvent::Open(0));
        }
        out.extend(self.take_all());
        if now == State::Closed && self.seen != State::Closed {
            self.reason = Some(self.read_reason());
            out.push(NetEvent::Closed(0));
        }
        self.seen = now;
        out
    }

    /// Whether the three channels are open.
    pub fn is_open(&self) -> bool {
        self.seen == State::Open
    }

    /// Why the link closed, once it has.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Send a message (dropped while the channels are not open, as a full send buffer drops one).
    pub fn send(&self, chan: Chan, data: &[u8]) {
        #[cfg(target_os = "emscripten")]
        // SAFETY: the bytes are read and copied before the call returns.
        unsafe {
            js::sc_rtc_send(self.handle, chan as i32, data.as_ptr(), data.len() as i32)
        };
        #[cfg(not(target_os = "emscripten"))]
        let _ = (chan, data);
    }

    #[cfg(target_os = "emscripten")]
    fn state(&self) -> State {
        // SAFETY: a read of the JavaScript's table.
        state_of(unsafe { js::sc_rtc_state(self.handle) })
    }
    #[cfg(not(target_os = "emscripten"))]
    fn state(&self) -> State {
        state_of(2)
    }

    #[cfg(target_os = "emscripten")]
    fn read_reason(&self) -> String {
        let mut buf = vec![0u8; 512];
        // SAFETY: the JavaScript writes at most `cap` bytes into `buf`.
        let n = unsafe { js::sc_rtc_reason(self.handle, buf.as_mut_ptr(), buf.len() as i32) };
        buf.truncate(n.clamp(0, buf.len() as i32) as usize);
        String::from_utf8_lossy(&buf).into_owned()
    }
    #[cfg(not(target_os = "emscripten"))]
    fn read_reason(&self) -> String {
        String::new()
    }

    #[cfg(target_os = "emscripten")]
    fn take_all(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        loop {
            // SAFETY: reads of the JavaScript's inbox; `take` writes at most `cap` bytes and one i32.
            let size = unsafe { js::sc_rtc_next(self.handle) };
            if size < 0 {
                break;
            }
            let mut buf = vec![0u8; size as usize];
            let mut chan = -1i32;
            let n = unsafe { js::sc_rtc_take(self.handle, &mut chan, buf.as_mut_ptr(), size) };
            if n < 0 {
                break;
            }
            buf.truncate(n as usize);
            if let Some(c) = chan_of(chan) {
                out.push(NetEvent::Data(0, c, buf));
            }
        }
        out
    }
    #[cfg(not(target_os = "emscripten"))]
    fn take_all(&mut self) -> Vec<NetEvent> {
        let _ = chan_of;
        Vec::new()
    }
}

impl Drop for WebLink {
    fn drop(&mut self) {
        #[cfg(target_os = "emscripten")]
        // SAFETY: closes the JavaScript's connection; the handle is not used again.
        unsafe {
            js::sc_rtc_close(self.handle)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_calls_the_deployed_matchmaker_over_https_and_a_local_one_over_http() {
        // An HTTPS page (itch.io) may not fetch plain HTTP, and Fly ends TLS for the deployed matchmaker; a matchmaker
        // on this machine has no TLS, and a test page served from localhost may call it plainly.
        let fly = Matchmaker::new("http://star-crew-bridge-simulator.fly.dev").unwrap();
        assert_eq!(page_url(&fly), "https://star-crew-bridge-simulator.fly.dev");
        let local = Matchmaker::new("http://127.0.0.1:7800").unwrap();
        assert_eq!(page_url(&local), "http://127.0.0.1:7800");
    }

    #[test]
    fn the_scripts_states_and_channels_read_as_the_transports() {
        assert_eq!([0, 1, 2, 7].map(state_of), [State::Joining, State::Open, State::Closed, State::Closed]);
        assert_eq!([0, 1, 2, 3].map(chan_of), [Some(Chan::Command), Some(Chan::Input), Some(Chan::Snapshot), None]);
    }
}
