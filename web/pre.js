// The browser build's start (engine-stack design 10a), linked in by crates/sc-client/build.rs. Before
// main runs it fetches the compressed deck beside the page, unpacks it with the browser's own gzip
// (DecompressionStream) and writes it where the client reads it on every platform:
// /compiled/tern.deck (or, where a host serves only text, tern.deck.gz.txt in base64). A host that already unpacked it (Content-Encoding: gzip) is fine too: the file
// is unpacked only when it starts with gzip's magic bytes. The page shows what Module.setStatus says.
Module.preRun = Module.preRun || [];
Module.preRun.push(function () {
  Module.addRunDependency("deck");
  const say = (t) => (Module.setStatus ? Module.setStatus(t) : console.log(t));
  say("Loading the ship...");
  // A host that serves only text (a claude.ai artifact) carries the same bytes as base64 in tern.deck.gz.txt.
  const fromText = () => fetch("tern.deck.gz.txt").then((r) => {
    if (!r.ok) throw new Error("tern.deck.gz: " + r.status);
    return r.text().then((t) => Uint8Array.from(atob(t.trim()), (c) => c.charCodeAt(0)).buffer);
  });
  fetch("tern.deck.gz")
    .then((r) => (r.ok ? r.arrayBuffer() : fromText()), fromText)
    .then((buf) => {
      const b = new Uint8Array(buf);
      if (b[0] !== 0x1f || b[1] !== 0x8b) return b;
      const s = new Blob([b]).stream().pipeThrough(new DecompressionStream("gzip"));
      return new Response(s).arrayBuffer().then((u) => new Uint8Array(u));
    })
    .then((deck) => {
      FS.mkdirTree("/compiled");
      FS.writeFile("/compiled/tern.deck", deck);
      say("");
      Module.removeRunDependency("deck");
    })
    .catch((e) => say("Could not load the ship: " + e.message));
});
