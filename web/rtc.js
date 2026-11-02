// The browser's WebRTC link (openspec/changes/matchmaker design 10.7): the web build joins a crew by its code
// through the matchmaker with the page's own RTCPeerConnection, because a page cannot open the UDP sockets the
// native client's str0m uses. Linked into sc-client with --js-library (crates/sc-client/build.rs); the Rust side is
// crates/sc-net/src/web.rs, which polls these functions once a frame.
//
// One join is one handle. The three channels are the native client's (crates/sc-net/src/transport.rs LABELS):
// "command" reliable and ordered, "input" and "snapshot" unordered with no retransmits. The offer carries every
// candidate (no trickle, design 10.4) except Chrome's ".local" names, which the server's SDP parser cannot read; the
// server-reflexive one from STUN is the one that crosses the internet.

mergeInto(LibraryManager.library, {
  $SCRTC: {
    links: [],
    LABELS: ["command", "input", "snapshot"],
    // Wait for ICE gathering to finish, or give up waiting after `ms` and offer what was gathered.
    gathered: function (pc, ms) {
      return new Promise(function (done) {
        if (pc.iceGatheringState === "complete") return done();
        var t = setTimeout(done, ms);
        pc.addEventListener("icegatheringstatechange", function () {
          if (pc.iceGatheringState === "complete") { clearTimeout(t); done(); }
        });
      });
    },
    // Drop the candidates the server cannot parse: a host name instead of an IP address (Chrome's mDNS names).
    plainCandidates: function (sdp) {
      return sdp.split("\r\n").filter(function (line) {
        if (line.indexOf("a=candidate:") !== 0) return true;
        var addr = line.split(" ")[4] || "";
        return addr.indexOf(".local") < 0 && /^[0-9a-fA-F.:]+$/.test(addr);
      }).join("\r\n");
    },
    fail: function (link, reason) {
      if (link.state !== 2) { link.state = 2; link.reason = reason; }
      try { link.pc && link.pc.close(); } catch (e) {}
    },
  },

  sc_rtc_join__deps: ["$SCRTC"],
  sc_rtc_join: function (urlPtr, codePtr, stunPtr, protocol) {
    var url = UTF8ToString(urlPtr), code = UTF8ToString(codePtr), stun = JSON.parse(UTF8ToString(stunPtr));
    var link = { state: 0, reason: "", inbox: [], pc: null, chans: [null, null, null], open: 0 };
    var h = SCRTC.links.length;
    SCRTC.links.push(link);
    var ice = stun.length ? [{ urls: stun.map(function (s) { return "stun:" + s; }) }] : [];
    var pc;
    try {
      pc = new RTCPeerConnection({ iceServers: ice });
    } catch (e) {
      SCRTC.fail(link, "This browser has no WebRTC: " + e);
      return h;
    }
    link.pc = pc;
    SCRTC.LABELS.forEach(function (label, i) {
      var opts = i === 0 ? { ordered: true } : { ordered: false, maxRetransmits: 0 };
      var dc = pc.createDataChannel(label, opts);
      dc.binaryType = "arraybuffer";
      dc.onopen = function () {
        link.open |= 1 << i;
        if (link.open === 7 && link.state === 0) link.state = 1;
      };
      dc.onclose = function () { SCRTC.fail(link, "The connection to the crew closed"); };
      dc.onmessage = function (ev) {
        var data = typeof ev.data === "string" ? new TextEncoder().encode(ev.data) : new Uint8Array(ev.data);
        link.inbox.push([i, data]);
      };
      link.chans[i] = dc;
    });
    pc.onconnectionstatechange = function () {
      if (pc.connectionState === "failed") {
        SCRTC.fail(link, "Could not reach the crew's game (no direct path, and no relay yet)");
      }
    };
    pc.createOffer()
      .then(function (offer) { return pc.setLocalDescription(offer); })
      .then(function () { return SCRTC.gathered(pc, 4000); })
      .then(function () {
        var offer = { type: "offer", sdp: SCRTC.plainCandidates(pc.localDescription.sdp) };
        // text/plain: a simple request, so no preflight is needed across origins (design 10.7).
        return fetch(url + "/v1/join/" + encodeURIComponent(code), {
          method: "POST",
          headers: { "Content-Type": "text/plain" },
          body: JSON.stringify({ protocol: protocol, offer: offer }),
        });
      })
      .then(function (r) {
        return r.text().then(function (t) {
          var body = null;
          try { body = JSON.parse(t); } catch (e) {}
          if (!r.ok) throw new Error((body && body.reason) || ("The matchmaker said HTTP " + r.status));
          if (!body || body.type !== "answer") throw new Error("The crew's game sent no answer");
          return pc.setRemoteDescription(body);
        });
      })
      .catch(function (e) {
        var m = (e && e.message) || String(e);
        if (m === "Failed to fetch" || m.indexOf("NetworkError") >= 0) m = "Could not reach the matchmaker at " + url;
        SCRTC.fail(link, m);
      });
    return h;
  },

  // 0 joining, 1 open (all three channels), 2 closed.
  sc_rtc_state__deps: ["$SCRTC"],
  sc_rtc_state: function (h) {
    var link = SCRTC.links[h];
    return link ? link.state : 2;
  },

  // The reason a link closed, into buf (at most cap bytes); the bytes written.
  sc_rtc_reason__deps: ["$SCRTC"],
  sc_rtc_reason: function (h, buf, cap) {
    var link = SCRTC.links[h];
    if (!link || cap <= 0) return 0;
    return stringToUTF8(link.reason, buf, cap);
  },

  sc_rtc_send__deps: ["$SCRTC"],
  sc_rtc_send: function (h, chan, ptr, len) {
    var link = SCRTC.links[h];
    if (!link || link.state !== 1) return;
    var dc = link.chans[chan];
    if (!dc || dc.readyState !== "open") return;
    // A copy: the wasm heap may move before the browser sends it.
    try { dc.send(HEAPU8.slice(ptr, ptr + len)); } catch (e) {}
  },

  // The size of the next message waiting, or -1 when none is.
  sc_rtc_next__deps: ["$SCRTC"],
  sc_rtc_next: function (h) {
    var link = SCRTC.links[h];
    return link && link.inbox.length ? link.inbox[0][1].length : -1;
  },

  // Take the next message into buf (cap at least its size) and its channel into *chanOut; its size, or -1.
  sc_rtc_take__deps: ["$SCRTC"],
  sc_rtc_take: function (h, chanOut, buf, cap) {
    var link = SCRTC.links[h];
    if (!link || !link.inbox.length || link.inbox[0][1].length > cap) return -1;
    var m = link.inbox.shift();
    HEAPU8.set(m[1], buf);
    HEAP32[chanOut >> 2] = m[0];
    return m[1].length;
  },

  sc_rtc_close__deps: ["$SCRTC"],
  sc_rtc_close: function (h) {
    var link = SCRTC.links[h];
    if (link) SCRTC.fail(link, "Left the crew");
  },
});
