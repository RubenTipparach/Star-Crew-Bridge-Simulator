/*
 * propkit.js: the Blender-built props in a Star Crew mockup, and the console faces on them.
 *
 * What it owns: turning the prop sets a page carries (assets/models/<set>, inlined by
 * tools/mockups/inline.py between "INLINE models:<set>" markers) into parts the kit's builder
 * merges into a room's one mesh: loading the glb files, choosing a station's own variant of a prop
 * (props.json variant_of and stations, the rule of tools/bridge_variants.py prop_for), placing a
 * prop by its back and the way it faces, a seated crew figure for scale, and the console faces of
 * bridge-stations design 11.6 (each screen showing its own station's console, baked by
 * tools/mockups/console_screens.py into the "INLINE screens" block, and key panels from the wall
 * panels' keys image), all in one atlas so a room's faces are one draw.
 *
 * Why a lib of its own: the bridge variants page and the command deck page place, face and light
 * props the same way, so this is the one copy (CLAUDE.md 6.1), inlined between the
 * "INLINE lib:propkit" markers. Edit it here, never inside a page.
 *
 * Use: const P = await window.PropKit.create(THREE, ShipKit, GLTFLoader, mats, { sets: ["bridge", "suite"] });
 * then P.placeProp(parts, kind, back_m, yaw_deg, station), P.addFaces(F, kind, back_m, yaw_deg, station),
 * P.placeCrew(parts, seat_m, yaw_deg, station), P.faceMat for the faces' mesh, P.bytes for the HUD.
 */
(function () {
  const ROLE_OF = { captain: "command", helm: "helm", tactical: "tactical", engineering: "engineering", science: "science", comms: "comms", flight_ops: "flight_ops" };
  const LIFT_M = 0.01, BEZEL_M = 0.012;   // faces stand 1 cm proud of their recess floor (CLAUDE.md 8); a 12 mm black-glass bezel
  const v3 = { add: (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]], mul: (a, k) => [a[0] * k, a[1] * k, a[2] * k],
    cross: (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] };

  async function create(THREE, K, GLTFLoader, mats, opts) {
    opts = opts || {};
    const sets = opts.sets || ["bridge"];
    const roleColor = (station) => new THREE.Color(ROLE_OF[station] ? K.PALETTE.role[ROLE_OF[station]] : K.PALETTE.screen);
    const STAR_MATS = new Set(Object.keys(mats.layer));

    // ---------------------------------------------------------------- the prop sets
    async function loadSet(set) {
      const el = document.getElementById(`ship-models-${set}`);
      if (!el) return null;
      const d = JSON.parse(el.textContent), loader = new GLTFLoader(), out = {};
      for (const [name, uri] of Object.entries(d.models)) {
        const buf = Uint8Array.from(atob(uri.slice(uri.indexOf(",") + 1)), (c) => c.charCodeAt(0)).buffer;
        const gltf = await new Promise((res, rej) => loader.parse(buf, "", res, rej));
        gltf.scene.updateMatrixWorld(true);
        const parts = {};
        gltf.scene.traverse((o) => {
          if (!o.isMesh) return;
          const g = (o.geometry.index ? o.geometry.toNonIndexed() : o.geometry.clone()).applyMatrix4(o.matrixWorld);
          const ms = Array.isArray(o.material) ? o.material : [o.material];
          const groups = g.groups.length ? g.groups : [{ start: 0, count: g.attributes.position.count, materialIndex: 0 }];
          const pa = g.attributes.position, na = g.attributes.normal, ua = g.attributes.uv;
          for (const gr of groups) {
            const mname = ((ms[gr.materialIndex] || ms[0]).name || "trim").split(".")[0];
            const P = parts[mname] || (parts[mname] = { position: [], normal: [], uvm: [] });
            for (let i = gr.start; i < gr.start + gr.count; i++) {
              const p = [pa.getX(i), pa.getY(i), pa.getZ(i)], n = na ? [na.getX(i), na.getY(i), na.getZ(i)] : [0, 1, 0];
              P.position.push(...p); P.normal.push(...n);
              const uv = ua ? [ua.getX(i), ua.getY(i)] : K.worldUv(p, n);
              P.uvm.push(uv[0], uv[1]);
            }
          }
        });
        out[name] = { parts, rec: d.manifest.props[name], set };
      }
      return out;
    }
    let PROPS = null;
    for (const set of sets) {
      const s = await loadSet(set);
      if (s) PROPS = Object.assign(PROPS || {}, s);
    }
    /** A placed prop's base model: a station's variant (props.json variant_of) is its base with that station's hand controls.
     * Without the models in the page, a variant's name starts with its base's. */
    const baseOf = (kind) => (PROPS && PROPS[kind] && PROPS[kind].rec.variant_of)
      || ["wall_bank_double", "wall_bank_core", "wall_bank", "free_console"].find((k) => kind.startsWith(k)) || kind;
    /** The prop for a station where base would stand: its own variant when the manifest has one (the rule of tools/bridge_variants.py prop_for). */
    function propFor(base, station) {
      if (!PROPS) return base;
      for (const [name, p] of Object.entries(PROPS)) if (p.rec.variant_of === base && (p.rec.stations || []).includes(station)) return name;
      return base;
    }

    // ---------------------------------------------------------------- console faces
    // Design 11.6: each console screen shows its own station's console, baked into images by
    // tools/mockups/console_screens.py (assets/textures/screens, inlined as ship-screens); a key
    // panel is blocks of the wall panels' keys_crew.png at their real size and a trackpad. Every
    // image goes into one atlas texture, two 256 x 128 images to 256 rows as the engine's array will
    // hold them, so all the faces in a room are one draw. Alpha is the emission mask: a screen glows,
    // a key panel's unlit keys take the room's baked light.
    async function loadScreens() {
      const el = document.getElementById("ship-screens");
      if (!el || !PROPS) return null;
      const d = JSON.parse(el.textContent), man = d.manifest, W = man.image_px[0];
      const files = Object.keys(d.images), img = {}, at = {};
      let rows = 0;
      for (const f of files) {
        img[f] = await K.decodePng(d.images[f]);
        if (img[f].w !== W) throw new Error(`screens: ${f} is ${img[f].w} px wide, not ${W}`);
        at[f] = rows; rows += Math.ceil(img[f].h / 128) * 128;
      }
      const data = new Uint8Array(W * rows * 4);
      for (const f of files) data.set(img[f].rgba, at[f] * W * 4);
      const tex = new THREE.DataTexture(data, W, rows, THREE.RGBAFormat, THREE.UnsignedByteType);
      tex.colorSpace = THREE.SRGBColorSpace; tex.magFilter = THREE.NearestFilter; tex.minFilter = THREE.LinearMipmapLinearFilter;
      tex.generateMipmaps = true; tex.needsUpdate = true;
      // A tile: a content rectangle [x, y, w, h] (px from the image's top left) as atlas UVs; v grows down the image.
      const tile = (f, r) => ({ u0: r[0] / W, u1: (r[0] + r[2]) / W, v0: (at[f] + r[1]) / rows, v1: (at[f] + r[1] + r[3]) / rows, aspect: r[2] / r[3] });
      const main = {}, upper = {};
      for (const [sid, r] of Object.entries(man.stations)) {
        main[sid] = tile(r.main.file, r.main.content_px);
        upper[sid] = r.upper.halves.map((h) => tile(r.upper.file, h.content_px));
      }
      const g = man.shared.generic, k = man.shared.keys;
      main.generic = tile(g.file, g.content_px); upper.generic = [main.generic, main.generic];
      return { texture: tex, main, upper, keys: tile(k.file, k.content_px), pad: tile(k.file, k.pad_px), keySize: k.size_m, boards: man.boards, bytes: Math.round((data.length * 4) / 3) };
    }
    const SCREENS = await loadScreens();
    /** Whose images a station's screens show: its own, or for a board without a console the one screens.json names. */
    function imagesOf(station) {
      const s = station.replace(/_[ps]$/, "");
      return SCREENS.main[s] ? s : (SCREENS.boards[s] || "generic");
    }

    /** Append a placed prop's console faces to F ({ position, normal, uv }): turned by yaw and moved to back_m as placeProp does. */
    function addFaces(F, kind, back, yawDeg, station) {
      const pr = PROPS && PROPS[kind];
      if (!pr || !SCREENS || !pr.rec.screens) return;
      const th = (yawDeg * Math.PI) / 180, c = Math.cos(th), s = Math.sin(th);
      const world = (p) => [back[0] + p[0] * c + p[2] * s, back[1] + p[1], back[2] - p[0] * s + p[2] * c];
      const turn = (d) => [d[0] * c + d[2] * s, d[1], -d[0] * s + d[2] * c];
      const who = imagesOf(station);
      for (const sc of pr.rec.screens) {
        const n = sc.normal, o = v3.add(sc.centre_m, v3.mul(n, LIFT_M));
        let ax = v3.cross(sc.up, n), ay = sc.up, W = sc.width_m, H = sc.height_m;
        // A quad centred (cx, cy) in the layout axes, half sizes (hx, hy), showing tile t the right way up.
        const quad = (cx, cy, hx, hy, t) => {
          const at = (u, v) => world(v3.add(o, v3.add(v3.mul(ax, cx + u), v3.mul(ay, cy + v))));
          const tl = at(-hx, hy), bl = at(-hx, -hy), br = at(hx, -hy), tr = at(hx, hy), nw = turn(n);
          F.position.push(...tl, ...bl, ...br, ...tl, ...br, ...tr);
          for (let i = 0; i < 6; i++) F.normal.push(...nw);
          F.uv.push(t.u0, t.v0, t.u0, t.v1, t.u1, t.v1, t.u0, t.v0, t.u1, t.v1, t.u1, t.v0);
        };
        if (sc.shows === "console" || sc.shows === "upper") {
          const t = sc.shows === "console" ? SCREENS.main[who] : SCREENS.upper[who][sc.half || 0];
          const w = W - 2 * BEZEL_M, h = Math.min(H - 2 * BEZEL_M, w / t.aspect);
          quad(0, 0, (h * t.aspect) / 2, h / 2, t);
          continue;
        }
        if (sc.shows === "strip") {
          // A long display (a table's inlay): the station's two upper halves in turn, end to end along it.
          if (H > W) { [W, H] = [H, W]; [ax, ay] = [ay, v3.mul(ax, -1)]; }
          const halves = SCREENS.upper[who], h = H - 2 * BEZEL_M, w = h * halves[0].aspect;
          const n = Math.max(1, Math.floor((W - 2 * BEZEL_M) / w));
          for (let i = 0; i < n; i++) quad(-((n - 1) * w) / 2 + i * w, 0, w / 2, h / 2, halves[i % halves.length]);
          continue;
        }
        // A key panel: keys_crew blocks at their real size across, a trackpad below them (toward the
        // operator) or beside them; a panel taller than it is wide is laid out along its length.
        if (H > W) { [W, H] = [H, W]; [ax, ay] = [ay, v3.mul(ax, -1)]; }
        const m = 0.012, g = 0.02, [bw0, bh0] = SCREENS.keySize;
        const k = Math.min(1, (W - 2 * m) / bw0, (H - 2 * m) / bh0), bw = bw0 * k, bh = bh0 * k;
        const nb = Math.max(1, Math.floor((W - 2 * m + g) / (bw + g))), rowW = nb * bw + (nb - 1) * g;
        const below = H - 2 * m - bh >= 0.06, side = !below && W - 2 * m - rowW - g >= 0.08;
        const padW = Math.min(0.12, W * 0.3), padH = below ? Math.min(0.08, H - 2 * m - bh - 0.015) : Math.min(0.09, bh);
        let x = -(rowW + (side ? g + padW : 0)) / 2;
        const yKeys = below ? H / 2 - m - bh / 2 : 0;
        for (let i = 0; i < nb; i++) { quad(x + bw / 2, yKeys, bw / 2, bh / 2, SCREENS.keys); x += bw + g; }
        if (side) quad(x + padW / 2, 0, padW / 2, padH / 2, SCREENS.pad);
        else if (below) quad(0, -H / 2 + m + padH / 2, padW / 2, padH / 2, SCREENS.pad);
      }
    }
    /** The faces' material: the atlas, times the baked vertex colour where the mask is 0, unshaded where it is 1. */
    const faceMat = new THREE.MeshBasicMaterial({ map: SCREENS ? SCREENS.texture : null, vertexColors: true });
    faceMat.onBeforeCompile = (sh) => {
      sh.fragmentShader = sh.fragmentShader.replace("#include <color_fragment>",
        "#if defined( USE_MAP ) && defined( USE_COLOR )\n\tdiffuseColor.rgb *= mix( vColor, vec3( 1.0 ), sampledDiffuseColor.a );\n\tdiffuseColor.a = 1.0;\n#endif");
    };

    // ---------------------------------------------------------------- placing
    /** A plain stand-in for a prop when the Blender models are not in the page: boxes of the prop's size
     * (the bridge set's shapes, or a box of opts.sizes[kind] = [w, h, d] for any other). */
    function standIn(kind) {
      const B = new K.Builder(), X = [1, 0, 0], Y = [0, 1, 0], Z = [0, 0, 1];
      const box = (role, c, hx, hy, hz) => B.box(role, c, X, Y, Z, hx, hy, hz, ["+u", "-u", "+v", "-v", "+w", "-w"]);
      const wide = { wall_bank_core: 1.4, wall_bank: 1.1, wall_bank_double: 2.4 }[baseOf(kind)];
      if (wide) {
        box("machinery", [0, 0.4, 0.3], wide / 2, 0.4, 0.3);
        box("screen", [0, 1.5, 0.06], wide / 2 - 0.08, 0.45, 0.04);
        box("trim", [-wide / 2 - 0.06, 1.05, 0.33], 0.06, 1.05, 0.33);
        box("trim", [wide / 2 + 0.06, 1.05, 0.33], 0.06, 1.05, 0.33);
      } else if (baseOf(kind) === "free_console" || kind === "helm_arc") {
        const w = kind === "helm_arc" ? 1.2 : 0.7;
        box("machinery", [0, 0.37, 0.35], w, 0.37, 0.3);
        box("screen", [0, 0.95, 0.15], w - 0.1, 0.2, 0.03);
      } else if (kind === "standup_console") {
        box("machinery", [0, 0.55, 0.2], 0.3, 0.55, 0.2);
        box("screen", [0, 1.15, 0.2], 0.28, 0.02, 0.18);
      } else if (kind === "captain_chair" || kind === "crew_chair") {
        const s = kind === "captain_chair" ? 1.2 : 1.0;
        box("machinery", [0, 0.2, 0.3], 0.08, 0.2, 0.08);
        box("trim", [0, 0.48, 0.3], 0.28 * s, 0.06, 0.26 * s);
        box("trim", [0, 0.85, 0.06], 0.26 * s, 0.35 * s, 0.05);
      } else if (opts.sizes && opts.sizes[kind]) {
        const [w, h, d] = opts.sizes[kind];
        box("machinery", [0, h / 2, d / 2], w / 2, h / 2, d / 2);
      }
      return { parts: B.parts, rec: null };
    }
    const propOf = (kind) => (PROPS && PROPS[kind]) || standIn(kind);
    /** How far ahead of a prop's back its operator sits (props.json operators_m; the stand-ins' own sizes otherwise). */
    const operatorZ = (kind, fallback) => (PROPS && PROPS[kind] && PROPS[kind].rec.operators_m && PROPS[kind].rec.operators_m[0][2]) || fallback;

    /** Append a prop's parts, turned by yaw (degrees, 0 faces the bow) and moved to back_m, into parts by role.
     * Its accent takes the station's role colour, or o.accent (a THREE.Color) when given. */
    function placeProp(parts, kind, back, yawDeg, station, o) {
      const pr = propOf(kind), th = (yawDeg * Math.PI) / 180, c = Math.cos(th), s = Math.sin(th);
      const tint = (o && o.accent) || roleColor(station);
      for (const [mname, P] of Object.entries(pr.parts)) {
        const screen = mname === "screen", accent = mname === "accent";
        // Under a console face (design 11.6) a screen's recess floor is black glass; without the faces it glows the station's colour.
        const glass = screen && SCREENS && PROPS && PROPS[kind];
        const role = screen ? "screen" : `prop_${STAR_MATS.has(mname) ? mname : "trim"}${accent ? "_accent" : ""}`;
        const dst = parts[role] || (parts[role] = { position: [], normal: [], uvm: [], tint: [], material: screen ? "light_panel" : (STAR_MATS.has(mname) ? mname : "trim") });
        dst.tint = dst.tint || [];
        for (let i = 0; i < P.position.length / 3; i++) {
          const x = P.position[i * 3], y = P.position[i * 3 + 1], z = P.position[i * 3 + 2];
          dst.position.push(back[0] + x * c + z * s, back[1] + y, back[2] - x * s + z * c);
          const nx = P.normal[i * 3], ny = P.normal[i * 3 + 1], nz = P.normal[i * 3 + 2];
          dst.normal.push(nx * c + nz * s, ny, -nx * s + nz * c);
          dst.uvm.push(P.uvm[i * 2], P.uvm[i * 2 + 1]);
          const t = glass ? [0.03, 0.035, 0.045] : screen ? [tint.r * 0.9, tint.g * 0.9, tint.b * 0.9] : accent ? [tint.r, tint.g, tint.b] : [1, 1, 1];
          dst.tint.push(...t);
        }
      }
    }

    /** A seated crew member for scale: legs, torso and head in the station's role colour. */
    function placeCrew(parts, seat, yawDeg, station) {
      const th = (yawDeg * Math.PI) / 180, f = [Math.sin(th), 0, Math.cos(th)], r = [Math.cos(th), 0, -Math.sin(th)], up = [0, 1, 0];
      const B = new K.Builder(), col = roleColor(station), skin = [0.86, 0.69, 0.55];
      const at = (df, dy, dr) => [seat[0] + f[0] * df + r[0] * dr, seat[1] + dy, seat[2] + f[2] * df + r[2] * dr];
      const all = ["+u", "-u", "+v", "-v", "+w", "-w"];
      B.box("body", at(-0.05, 0.85, 0), r, up, f, 0.2, 0.3, 0.13, all);
      B.box("legs", at(0.2, 0.5, 0), r, up, f, 0.18, 0.08, 0.25, all);
      B.box("legs", at(0.42, 0.25, 0), r, up, f, 0.16, 0.25, 0.07, all);
      B.box("head", at(-0.02, 1.32, 0), r, up, f, 0.11, 0.13, 0.11, all);
      for (const [role, P] of Object.entries(B.parts)) {
        const dst = parts.crew || (parts.crew = { position: [], normal: [], uvm: [], tint: [], material: "trim" });
        const t = role === "head" ? skin : role === "legs" ? [0.18, 0.2, 0.24] : [col.r, col.g, col.b];
        dst.position.push(...P.position); dst.normal.push(...P.normal); dst.uvm.push(...P.uvm);
        for (let i = 0; i < P.position.length / 3; i++) dst.tint.push(...t);
      }
    }

    function mergeParts(dst, src) {
      for (const [r, P] of Object.entries(src)) {
        const d = dst[r];
        if (!d) { dst[r] = P; continue; }
        d.position.push(...P.position); d.normal.push(...P.normal); d.uvm.push(...P.uvm);
      }
    }

    return {
      PROPS, SCREENS, baseOf, propFor, imagesOf, addFaces, faceMat, standIn, propOf, operatorZ,
      placeProp, placeCrew, mergeParts, roleColor, bytes: SCREENS ? SCREENS.bytes : 0,
    };
  }

  window.PropKit = { create, ROLE_OF, LIFT_M, BEZEL_M };
})();
