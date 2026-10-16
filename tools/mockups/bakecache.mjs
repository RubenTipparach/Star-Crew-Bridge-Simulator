/*
 * bakecache.mjs: write a page's bake cache (docs/mockups/cache/<name>.bin), the one format the mockups' baked light
 * is kept in: gzip of "SCBK", version 1 (u32), the index's length (u32), the index as JSON, then the data. The index
 * carries the sha256 of docs/mockups/lib/lightbake.js, so tools/mockups/inline.py --check fails a cache baked by
 * another baker; the pages read it with LightBake.readBakeCache(name). Used by bake_ship.mjs (the deck plan) and
 * bake_spacedock.mjs (the spacedock's bay). Documentation tooling (CLAUDE.md 4).
 */
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import crypto from "node:crypto";

/** The sha256 of the baker the pages carry. */
export function bakerSha(root) {
  return crypto.createHash("sha256").update(fs.readFileSync(path.join(root, "docs", "mockups", "lib", "lightbake.js"))).digest("hex");
}

/** Write <root>/docs/mockups/cache/<name>.bin from an index (baker_sha256 is added) and the data's chunks; returns its path. */
export function writeBakeCache(root, name, index, chunks) {
  const json = Buffer.from(JSON.stringify({ baker_sha256: bakerSha(root), ...index })), head = Buffer.alloc(12);
  head.write("SCBK", 0, "ascii"); head.writeUInt32LE(1, 4); head.writeUInt32LE(json.length, 8);
  const out = path.join(root, "docs", "mockups", "cache", name + ".bin");
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, zlib.gzipSync(Buffer.concat([head, json, ...chunks]), { level: 9, mtime: 0 }));
  return out;
}
