/*
 * stationrepair.js: a damaged bridge station in a Star Crew mockup, walked up to and repaired in first person
 * (openspec/changes/repairs-on-deck, design 3f; the owner, 2026-10-10: "say I'm in fps mode, walk up to a damaged
 * station that is sparking and has burn marks (decals) on them").
 *
 * What it owns, for one station's console placed in a page's room:
 *   - the damage the room shows (design 3f's table): sparks from the seam under the desk every 1-4 s with a flash,
 *     a thin smoke while disabled, scorch decals projected onto the surfaces round the desk (never a texture of the
 *     console's own), and the screen's static over its face; after the repair the screen boots and the sparks stop,
 *     and the burn marks stay;
 *   - the repair (design 3e at a console): E within reach docks behind the desk (the page's ShipWalk holds the body
 *     still and gives the mouse back), the cover's screws are turned out by dragging round each anticlockwise (3a),
 *     the burnt card is clicked out into the tray, the right card is fitted from the pouch (a wrong one fails its test
 *     and comes back), and the whole bay is the target that opens the job's 2D game (repairs/kit.js) to calibrate it;
 *     the last round puts the cover back on and the console comes back up. Esc stands up at any point and the job
 *     keeps what is done.
 *
 * Why a lib: the deck plan (and any page that walks a room with stations) docks at a station the same way
 * (CLAUDE.md 6.1). repairs-on-deck.html and service-panels.html keep their own copies of the machine docking until they
 * move onto this (design 3f). Inlined by tools/mockups/inline.py between the "INLINE lib:stationrepair" markers. Edit
 * it here, never inside a page.
 *
 * Use: const S = StationRepair.create(THREE, { scene, camera, dom, walk, DecalGeometry, RepairKit, rec, back, yaw_deg,
 *   name, game, surfaces, decals, repairs }); then S.update(dt) each frame and S.setLight(i) when the lighting state
 * changes. rec is the console prop's record (props.json: bounds_m, screens), back and yaw_deg its placement, surfaces
 * the room meshes the decals and the service face are projected onto, decals the page's #ship-decals block and
 * repairs its #ship-repairs block, both parsed.
 */
(function () {
  "use strict";
  const REACH_M = 1.2;          // crew-on-deck's use reach (repairs-on-deck 2)
  const FACING_COS = 0.5;       // within 60 degrees of facing the point
  const TURN = Math.PI * 2;     // one loop frees a screw (repairs-on-deck 3a)
  const MAXP = 120;             // sparks; with MAXS the ship's 200 (repairs-on-deck 10)
  const DECAL_LIFT_M = 0.002;   // a decal stands 2 mm off its surface (CLAUDE.md 8)
  const COVER_M = [0.46, 0.36]; // the service cover on the pedestal's back, metres
  // The parts (design 3f): the burnt card and the pouch's three, one of them right.
  const PARTS = {
    burnt: { id: "SP-4", name: "Signal processor", rating: "SP-4", note: "burnt" },
    pouch: [
      { id: "SP-2", name: "Signal processor", rating: "SP-2", right: false },
      { id: "SP-4", name: "Signal processor", rating: "SP-4", right: true },
      { id: "PS-4", name: "Power card", rating: "PS-4", right: false },
    ],
  };
  const TINTS = [[0.86, 0.86, 0.86], [0.95, 0.42, 0.4], [0.55, 0.6, 0.72]];

  function css() {
    if (document.getElementById("sr-style")) return;
    const s = document.createElement("style");
    s.id = "sr-style";
    s.textContent = `
.sr-prompt{position:fixed;left:50%;top:calc(50% + 40px);transform:translateX(-50%);z-index:60;pointer-events:none;padding:6px 12px;
  background:rgba(6,9,14,.84);border:1px solid #2b3540;border-radius:6px;font:600 14px system-ui,sans-serif;color:#e8eef6;white-space:nowrap;display:none}
.sr-prompt .k{display:inline-block;min-width:18px;padding:0 5px;margin-right:6px;border:1px solid #f2a046;border-radius:4px;color:#f2a046;text-align:center}
.sr-event{position:fixed;left:50%;top:12%;transform:translateX(-50%);z-index:66;pointer-events:none;font:700 22px system-ui,sans-serif;
  text-shadow:0 2px 10px #000;opacity:0;transition:opacity .25s}
.sr-dim{position:fixed;inset:0;background:rgba(0,0,0,.35);z-index:61;pointer-events:none;opacity:0;transition:opacity .3s}
.sr-panel{position:fixed;left:50%;top:50%;z-index:62;transform:translate(-50%,-50%);display:none;width:min(66.7vw,calc(66.7vh*16/9));aspect-ratio:16/9;
  border:1px solid #2b3540;border-radius:10px;overflow:hidden;box-shadow:0 10px 40px rgba(0,0,0,.6)}
.sr-panel canvas{width:100%;height:100%;display:block;touch-action:none}
.sr-ov{position:fixed;inset:0;z-index:62;pointer-events:none}
.sr-tray{position:fixed;right:16px;top:50%;transform:translateY(-50%);z-index:63;display:none;width:230px;padding:10px;background:rgba(6,9,14,.88);
  border:1px solid #2b3540;border-radius:8px;font:13px system-ui,sans-serif;color:#e8eef6}
.sr-tray h4{margin:0 0 6px;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:#9fb0c0}
.sr-card{display:block;width:100%;text-align:left;margin:0 0 6px;padding:7px 9px;border-radius:6px;border:1px solid #2b3540;background:#121a25;color:#e8eef6;font:inherit;cursor:pointer;min-height:40px}
.sr-card b{display:block;font-size:15px}.sr-card small{color:#9fb0c0}
.sr-card.out{border-color:#ff4757;background:#24121a;cursor:default}.sr-card.bad{opacity:.45}
.sr-card:hover:not(.out){border-color:#f2a046}
.sr-bar{position:fixed;left:50%;bottom:18px;transform:translateX(-50%);z-index:63;display:none;color:#9fb0c0;font:13px system-ui,sans-serif;white-space:nowrap;
  background:rgba(6,9,14,.8);padding:5px 10px;border-radius:6px}
.sr-bar .k{display:inline-block;padding:0 5px;margin:0 3px;border:1px solid #6f7f94;border-radius:4px;color:#e8eef6}
body.sr-pick canvas{cursor:pointer}`;
    document.head.appendChild(s);
  }
  const el = (cls, parent, tag) => { const e = document.createElement(tag || "div"); e.className = cls; (parent || document.body).appendChild(e); return e; };
  const ease = (u) => u * u * (3 - 2 * u);

  function create(THREE, o) {
    css();
    const { scene, camera, dom, walk } = o;
    const rad = (d) => (d * Math.PI) / 180, th = rad(o.yaw_deg), c = Math.cos(th), s = Math.sin(th);
    const back = new THREE.Vector3(...o.back);
    /** A point in the prop's own space (x right, y up, z its front) in the ship's. */
    const toShip = (p) => new THREE.Vector3(back.x + p[0] * c + p[2] * s, back.y + p[1], back.z - p[0] * s + p[2] * c);
    const dirShip = (d) => new THREE.Vector3(d[0] * c + d[2] * s, d[1], -d[0] * s + d[2] * c).normalize();
    const surfaces = o.surfaces;
    const ray = new THREE.Raycaster();
    function hitSurface(from, dir, far) {
      ray.set(from, dir); ray.far = far;
      const h = ray.intersectObjects(surfaces, false)[0];
      if (!h) return null;
      const n = h.face.normal.clone().transformDirection(h.object.matrixWorld);
      if (n.dot(dir) > 0) n.negate();
      return { p: h.point.clone(), n };
    }
    const rng = (() => { let x = o.seed || 7331; return () => ((x = (x * 1664525 + 1013904223) >>> 0) / 4294967296); })();

    // ---------------------------------------------------------------- textures from the page's inlined blocks
    const load = (uri) => { const t = new THREE.TextureLoader().load(uri); t.colorSpace = THREE.SRGBColorSpace; t.anisotropy = 4; return t; };
    const D = o.decals, REP = o.repairs;
    const decalTex = Object.fromEntries(Object.entries(D.decals.decals).map(([k, d]) => [k, load(D.images[d.file])]));
    const coverRec = REP.covers.covers.crew;
    const coverTex = load(REP.images[coverRec.file]);
    const boardTex = load(REP.images[REP.covers.interiors.circuit.file]);

    // ---------------------------------------------------------------- the burn marks (design 3f: decals)
    // Where they lie, in the prop's space: a burn on the touch panel's corner, soot up the screen's housing and the
    // pedestal's front, the deck in front of the desk where the burning bits fell, and soot over the service panel.
    const decalMats = {};
    const decalMat = (k) => decalMats[k] || (decalMats[k] = new THREE.MeshBasicMaterial({ map: decalTex[k], transparent: true, depthWrite: false,
      polygonOffset: true, polygonOffsetFactor: -4, polygonOffsetUnits: -4 }));
    const MARKS = [
      { at: [0.34, 0.80, 0.30], dir: [0, -0.98, -0.21], size: 0.34, k: "scorch_flat" },
      { at: [-0.38, 0.95, 0.12], dir: [0, -0.34, -0.94], size: 0.36, k: "scorch_streak" },
      { at: [0.10, 0.42, 0.62], dir: [0, 0, -1], size: 0.42, k: "scorch_streak" },
      { at: [0.25, 0.04, 0.95], dir: [0, -1, 0], size: 0.85, k: "scorch_flat" },
      { at: [-0.12, 0.64, -0.6], dir: [0, 0, 1], size: 0.4, k: "scorch_streak" },
    ];
    const decals = new THREE.Group();
    decals.renderOrder = 2;
    const helper = new THREE.Object3D();
    for (const m of MARKS) {
      const from = toShip(m.at), dir = dirShip(m.dir);
      const h = hitSurface(from.clone().addScaledVector(dir, -0.25), dir, 0.9);
      if (!h || !o.DecalGeometry) continue;
      helper.position.copy(h.p); helper.lookAt(h.p.clone().add(h.n));
      // A vertical face's soot rises: its decal's up is the ship's up; a flat one is turned at random.
      const vertical = Math.abs(h.n.y) < 0.5;
      helper.rotateZ(vertical ? 0 : rng() * Math.PI * 2);
      if (vertical) { const up = new THREE.Vector3(0, 1, 0); helper.up.copy(up); helper.lookAt(h.p.clone().add(h.n)); }
      const size = new THREE.Vector3(m.size, m.size, 0.3);
      for (const surf of surfaces) {
        const g = new o.DecalGeometry(surf, h.p, helper.rotation, size);
        if (!g.attributes.position || g.attributes.position.count === 0) { g.dispose(); continue; }
        const P = g.attributes.position.array, N = g.attributes.normal.array;
        for (let i = 0; i < P.length; i++) P[i] += N[i] * DECAL_LIFT_M;
        const mesh = new THREE.Mesh(g, decalMat(vertical ? "scorch_streak" : "scorch_flat"));
        mesh.renderOrder = 2;
        decals.add(mesh);
      }
    }
    scene.add(decals);

    // ---------------------------------------------------------------- the screen's static (design 3f)
    const scr = (o.rec.screens || []).find((x) => x.label === "screen_upright") || (o.rec.screens || [])[0];
    const noiseCv = document.createElement("canvas"); noiseCv.width = 128; noiseCv.height = 48;
    const ng = noiseCv.getContext("2d"), noiseTex = new THREE.CanvasTexture(noiseCv);
    noiseTex.magFilter = THREE.NearestFilter; noiseTex.colorSpace = THREE.SRGBColorSpace;
    // It writes depth, so the console's own face behind it (drawn in the room's batch) cannot paint over it.
    const screenMat = new THREE.MeshBasicMaterial({ map: noiseTex, transparent: true, opacity: 1, depthWrite: true });
    let screen = null;
    if (scr) {
      screen = new THREE.Mesh(new THREE.PlaneGeometry(scr.width_m, scr.height_m), screenMat);
      const n = dirShip(scr.normal), up = dirShip(scr.up);
      // In front of the console's own face and its glass (propkit stands faces 1 cm proud of their recess, behind a
      // 12 mm bezel), 3 cm out along the screen's normal.
      screen.position.copy(toShip(scr.centre_m)).addScaledVector(n, 0.03);
      screen.up.copy(up); screen.lookAt(screen.position.clone().add(n));
      screen.renderOrder = 10;
      scene.add(screen);
    }
    function drawStatic(level, boot) {
      const w = noiseCv.width, h = noiseCv.height, img = ng.createImageData(w, h);
      for (let y = 0; y < h; y++) {
        const band = Math.sin(y * 0.6 + perf * 9) > 0.7 ? 1.6 : 1;
        for (let x = 0; x < w; x++) {
          const v = Math.min(255, (rng() * 120 + 20) * level * band), i = (y * w + x) * 4;
          img.data[i] = v * 0.8; img.data[i + 1] = v * 0.9; img.data[i + 2] = v; img.data[i + 3] = 255;
        }
      }
      ng.putImageData(img, 0, 0);
      if (boot) { ng.fillStyle = `rgba(72,214,255,${boot})`; ng.fillRect(0, 0, w, h); }
      noiseTex.needsUpdate = true;
    }

    // ---------------------------------------------------------------- the service face on the pedestal's back
    // Found by casting from behind the console toward its back (its -z side) at knee height.
    const backDir = dirShip([0, 0, 1]);
    const coverAtProp = [0, 0.36, -1.4];
    const bh = hitSurface(toShip(coverAtProp), backDir, 1.6) || { p: toShip([0, 0.36, (o.rec.bounds_m || { min: [0, 0, 0] }).min[2]]), n: backDir.clone().negate() };
    const target = bh.p.clone(), normal = new THREE.Vector3(bh.n.x, 0, bh.n.z).normalize();
    const right = new THREE.Vector3(-normal.z, 0, normal.x);
    const floorY = back.y;
    const point = target.clone().addScaledVector(normal, 0.7); point.y = floorY;
    const coverEdge = new THREE.MeshBasicMaterial({ color: 0x2c2f33 }), coverMat = new THREE.MeshBasicMaterial({ map: coverTex });
    const cover = new THREE.Group();
    cover.add(new THREE.Mesh(new THREE.BoxGeometry(COVER_M[0], COVER_M[1], 0.02), [coverEdge, coverEdge, coverEdge, coverEdge, coverMat, coverEdge]));
    cover.position.copy(target).addScaledVector(normal, 0.012);
    cover.lookAt(cover.position.clone().add(normal));
    const coverHome = { p: cover.position.clone(), q: cover.quaternion.clone() };
    const offP = target.clone().addScaledVector(normal, 0.45).addScaledVector(right, COVER_M[0] * 1.2); offP.y = floorY + COVER_M[1] / 2 * 0.97;
    const coverOff = { p: offP, q: coverHome.q.clone().multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), -0.26)) };
    const screwMat = new THREE.MeshBasicMaterial({ color: 0xa9b4c2 }), slotMat = new THREE.MeshBasicMaterial({ color: 0x1a1f26 });
    const screws = coverRec.holes_uv.map(([hu, hv]) => {
      const g = new THREE.Group();
      g.add(new THREE.Mesh(new THREE.CylinderGeometry(0.016, 0.016, 0.012, 14).rotateX(Math.PI / 2), screwMat));
      const sl = new THREE.Mesh(new THREE.BoxGeometry(0.024, 0.005, 0.004), slotMat); sl.position.z = 0.0065; g.add(sl);
      g.position.set((hu - 0.5) * COVER_M[0], (0.5 - hv) * COVER_M[1], 0.016);
      g.userData.home = g.position.z;
      cover.add(g);
      return { g, t: 0 };
    });
    scene.add(cover);
    // Behind it: the console's board, the card cage's slot and the burnt card in it.
    const bay = new THREE.Group();
    bay.position.copy(target).addScaledVector(normal, 0.003);
    bay.quaternion.copy(coverHome.q);
    const bw = COVER_M[0] * 0.9, bhh = COVER_M[1] * 0.9;
    bay.add(new THREE.Mesh(new THREE.PlaneGeometry(COVER_M[0] * 0.96, COVER_M[1] * 0.96), new THREE.MeshBasicMaterial({ color: 0x0b0f14 })));
    const frameMat = new THREE.MeshBasicMaterial({ color: 0x2b3540 });
    const frame = new THREE.Mesh(new THREE.PlaneGeometry(bw + 0.014, bhh + 0.014), frameMat); frame.position.z = 0.002; bay.add(frame);
    const boardMat = new THREE.MeshBasicMaterial({ map: boardTex });
    const board = new THREE.Mesh(new THREE.PlaneGeometry(bw, bhh), boardMat); board.position.z = 0.003; bay.add(board);
    const SLOT = new THREE.Vector3(0.07, 0.02, 0.0);
    const slot = new THREE.Mesh(new THREE.BoxGeometry(0.15, 0.016, 0.02), new THREE.MeshBasicMaterial({ color: 0x15191f })); slot.position.copy(SLOT).setZ(0.013); bay.add(slot);
    const cardGeo = new THREE.BoxGeometry(0.13, 0.012, 0.09);
    const burntMat = new THREE.MeshBasicMaterial({ color: 0x1c1410 }), goodMat = new THREE.MeshBasicMaterial({ color: 0x1f6b3a });
    const card = new THREE.Mesh(cardGeo, burntMat);
    const cardSeat = new THREE.Vector3(SLOT.x, SLOT.y, 0.058);
    card.position.copy(cardSeat); bay.add(card);
    const ember = new THREE.Mesh(new THREE.SphereGeometry(0.009, 8, 6), new THREE.MeshBasicMaterial({ color: 0xff7a2e })); ember.position.set(0.035, 0.008, 0.02); card.add(ember);
    scene.add(bay);

    // ---------------------------------------------------------------- sparks and smoke: one batch
    const sparkTex = (() => {
      const cv = document.createElement("canvas"); cv.width = cv.height = 32; const g = cv.getContext("2d");
      const gr = g.createRadialGradient(16, 16, 0, 16, 16, 16); gr.addColorStop(0, "rgba(255,255,255,1)"); gr.addColorStop(0.4, "rgba(255,255,255,0.5)"); gr.addColorStop(1, "rgba(255,255,255,0)");
      g.fillStyle = gr; g.fillRect(0, 0, 32, 32); return new THREE.CanvasTexture(cv);
    })();
    const pPos = new Float32Array(MAXP * 3), pCol = new Float32Array(MAXP * 3), pts = [];
    const pGeo = new THREE.BufferGeometry();
    pGeo.setAttribute("position", new THREE.BufferAttribute(pPos, 3)); pGeo.setAttribute("color", new THREE.BufferAttribute(pCol, 3));
    const sparks = new THREE.Points(pGeo, new THREE.PointsMaterial({ size: 0.045, map: sparkTex, vertexColors: true, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
    sparks.frustumCulled = false; scene.add(sparks);
    const flash = new THREE.Sprite(new THREE.SpriteMaterial({ map: sparkTex, color: 0x9fd8ff, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, opacity: 0 }));
    flash.scale.setScalar(0.9); scene.add(flash);
    // Smoke: soft dark puffs that rise, spread and fade (ordinary blending, so it darkens what is behind it, not a
    // glow), one batch of its own (owner, 2026-10-10: "Smoke and spark should come out of damaged stations").
    const MAXS = 80;
    const sPos = new Float32Array(MAXS * 3), sCol = new Float32Array(MAXS * 4), sSize = new Float32Array(MAXS), puffs = [];
    const sGeo = new THREE.BufferGeometry();
    sGeo.setAttribute("position", new THREE.BufferAttribute(sPos, 3)); sGeo.setAttribute("color", new THREE.BufferAttribute(sCol, 4));
    sGeo.setAttribute("pSize", new THREE.BufferAttribute(sSize, 1));
    // Each puff's size in pixels (pSize, from its size in metres and its distance), so it grows as it rises.
    const smokeMat = new THREE.PointsMaterial({ size: 1, sizeAttenuation: false, map: sparkTex, vertexColors: true, transparent: true, depthWrite: false });
    smokeMat.onBeforeCompile = (sh) => {
      sh.vertexShader = sh.vertexShader.replace("#include <common>", "#include <common>\nattribute float pSize;")
        .replace("gl_PointSize = size;", "gl_PointSize = size * pSize;");
    };
    const smoke = new THREE.Points(sGeo, smokeMat);
    smoke.frustumCulled = false; smoke.renderOrder = 4; scene.add(smoke);
    function puff(at, { rise = 0.35, life = 3.2, grey = 0.16, size = 0.14, grow = 0.5, dense = 0.55 } = {}) {
      if (puffs.length >= MAXS) return;
      puffs.push({ p: at.clone(), v: new THREE.Vector3((rng() - 0.5) * 0.08, rise * (0.7 + rng() * 0.6), (rng() - 0.5) * 0.08), life, t: 0, grey, size, grow, dense });
    }
    function stepSmoke(dt) {
      for (const q of puffs) { q.t += dt; q.p.addScaledVector(q.v, dt); q.v.x += (rng() - 0.5) * 0.05 * dt; q.v.z += (rng() - 0.5) * 0.05 * dt; }
      for (let i = puffs.length - 1; i >= 0; i--) if (puffs[i].t >= puffs[i].life) puffs.splice(i, 1);
      // Pixels per metre at 1 m from the camera; a puff's size in metres over its distance is its size on screen.
      const k = (innerHeight * devicePixelRatio) / (2 * Math.tan((camera.fov * Math.PI) / 360));
      for (let i = 0; i < MAXS; i++) {
        const q = puffs[i];
        if (!q) { sPos[i * 3 + 1] = -1000; sCol[i * 4 + 3] = 0; continue; }
        sPos[i * 3] = q.p.x; sPos[i * 3 + 1] = q.p.y; sPos[i * 3 + 2] = q.p.z;
        const u = q.t / q.life, a = q.dense * Math.min(1, u * 5) * (1 - u);
        sCol[i * 4] = sCol[i * 4 + 1] = sCol[i * 4 + 2] = q.grey * (1 - 0.3 * u); sCol[i * 4 + 3] = a;
        const d = Math.max(0.3, camera.position.distanceTo(q.p));
        sSize[i] = ((q.size + q.grow * u) * k) / d;
      }
      sGeo.attributes.position.needsUpdate = sGeo.attributes.color.needsUpdate = sGeo.attributes.pSize.needsUpdate = true;
    }
    function emit(at, n, { speed = 2.2, col = [1, 0.8, 0.45], life = 0.6, up = 1.1, grav = -7 } = {}) {
      for (let i = 0; i < n && pts.length < MAXP; i++) {
        const a = rng() * Math.PI * 2, sp = speed * (0.4 + rng() * 0.6);
        pts.push({ p: at.clone(), v: new THREE.Vector3(Math.cos(a) * sp * 0.6, up * (0.2 + rng()), Math.sin(a) * sp * 0.6), life: life * (0.6 + rng() * 0.6), t: 0, col, grav });
      }
    }
    function stepParticles(dt) {
      for (const q of pts) { q.t += dt; q.v.y += q.grav * dt; q.p.addScaledVector(q.v, dt); }
      for (let i = pts.length - 1; i >= 0; i--) if (pts[i].t >= pts[i].life) pts.splice(i, 1);
      for (let i = 0; i < MAXP; i++) {
        const q = pts[i];
        if (!q) { pPos[i * 3 + 1] = -1000; continue; }
        pPos[i * 3] = q.p.x; pPos[i * 3 + 1] = q.p.y; pPos[i * 3 + 2] = q.p.z;
        const k = 1 - q.t / q.life;
        pCol[i * 3] = q.col[0] * k; pCol[i * 3 + 1] = q.col[1] * k; pCol[i * 3 + 2] = q.col[2] * k;
      }
      pGeo.attributes.position.needsUpdate = pGeo.attributes.color.needsUpdate = true;
    }

    // ---------------------------------------------------------------- the page's UI
    const promptEl = el("sr-prompt"), eventEl = el("sr-event"), dimEl = el("sr-dim"), panelEl = el("sr-panel"), ov = el("sr-ov", null, "canvas");
    const og = ov.getContext("2d"), tray = el("sr-tray"), bar = el("sr-bar");
    const gameCv = el("", panelEl, "canvas"); gameCv.width = 1280; gameCv.height = 720;
    let eventT = 0;
    function say(text, colour) { eventEl.textContent = text; eventEl.style.color = colour || "#e8eef6"; eventEl.style.opacity = 1; eventT = 2.6; }
    const runner = o.RepairKit ? o.RepairKit.start(gameCv, {}) : null;
    if (runner) runner.opts.paused = true;

    // ---------------------------------------------------------------- state
    // state: "damaged" (sparking), "disabled" (dark, smoking), "repaired". stage while docked: "cover" (screws out),
    // "lift" (the plate coming off), "pull" (click the burnt card), "fit" (pick from the pouch), "calibrate" (click the
    // bay), "game" (the 2D game over the room), "close" (the cover going back on), "done".
    const S = { sparkEvery: 0, state: "repaired", stage: "cover", ca: 0, cardOut: 0, fitted: null, tried: new Set(), docked: null, sparkT: 1, boot: 0, value: 100 };
    function damage(state) {
      S.state = state || "damaged"; S.stage = "cover"; S.ca = 0; S.cardOut = 0; S.fitted = null; S.tried.clear(); S.boot = 0;
      S.value = S.state === "disabled" ? 12 : 25;
      for (const sc of screws) sc.t = 0;
      card.material = burntMat; ember.visible = true; card.position.copy(cardSeat); card.visible = true;
      decals.visible = true; if (screen) screen.visible = true;
      if (runner) { runner.opts.paused = true; runner._job = null; }
      // Whatever was in hand is put away: a page resetting its view mid-repair starts the job clean.
      if (S.docked) { S.docked = null; if (walk.held()) walk.hold(false); }
      panelEl.style.display = "none"; dimEl.style.opacity = 0; tray.style.display = "none"; bar.style.display = "none";
      document.body.classList.remove("sr-pick");
      if (o.onChange) o.onChange();
    }
    const name = o.name || "Station";

    // ---------------------------------------------------------------- walking up and docking
    function nearby() {
      if (!walk.active() || walk.held() || S.state === "repaired" || S.docked) return false;
      const p = walk.pose();
      const dx = point.x - p.x, dz = point.z - p.z, d = Math.hypot(dx, dz);
      const tx = target.x - p.x, tz = target.z - p.z, tl = Math.hypot(tx, tz) || 1;
      const fx = -Math.sin(p.yaw), fz = -Math.cos(p.yaw);
      return d <= REACH_M && Math.abs(p.y - floorY) < 0.6 && (fx * tx + fz * tz) / tl >= FACING_COS;
    }
    function dockFrame() {
      const eye = target.clone().addScaledVector(normal, 0.62); eye.y = floorY + 0.92;
      const dir = target.clone().sub(eye).normalize();
      return { eye, yaw: Math.atan2(-dir.x, -dir.z), pitch: Math.asin(dir.y) };
    }
    function dock() {
      walk.hold(true);
      S.docked = { t: 0, dir: 1, from: { eye: camera.position.clone(), yaw: camera.rotation.y, pitch: camera.rotation.x }, to: dockFrame() };
      bar.style.display = "block";
      bar.innerHTML = `<span class="k">Esc</span> stand up`;
      say(`${name}: ${S.stage === "cover" ? "take the cover off" : "back to it"}`, "#9fb0c0");
    }
    function undock() {
      if (!S.docked) return;
      if (S.stage === "game") closeGame();
      const p = walk.pose();
      S.docked = { t: 0, dir: -1, from: { eye: camera.position.clone(), yaw: camera.rotation.y, pitch: camera.rotation.x }, to: { eye: new THREE.Vector3(p.x, p.eye, p.z), yaw: p.yaw, pitch: p.pitch } };
      tray.style.display = "none"; bar.style.display = "none"; document.body.classList.remove("sr-pick");
    }
    function camStep(dt) {
      const d = S.docked; if (!d) return;
      d.t = Math.min(1, d.t + dt / 0.3);
      const u = ease(d.t);
      camera.position.lerpVectors(d.from.eye, d.to.eye, u);
      const dy = Math.atan2(Math.sin(d.to.yaw - d.from.yaw), Math.cos(d.to.yaw - d.from.yaw));
      camera.rotation.set(d.from.pitch + (d.to.pitch - d.from.pitch) * u, d.from.yaw + dy * u, 0, "YXZ");
      if (d.dir < 0 && d.t >= 1) { S.docked = null; walk.hold(false); }
    }
    const docked = () => !!S.docked && S.docked.dir > 0 && S.docked.t >= 1;

    // ---------------------------------------------------------------- input while docked
    const ndc = new THREE.Vector2();
    function pick(e, obj) { ndc.set((e.clientX / innerWidth) * 2 - 1, -(e.clientY / innerHeight) * 2 + 1); ray.setFromCamera(ndc, camera); ray.far = 10; return ray.intersectObject(obj, true).length > 0; }
    const screwPx = (sc) => { const v = new THREE.Vector3(); sc.g.getWorldPosition(v); v.project(camera); return { x: (v.x + 1) / 2 * innerWidth, y: (1 - v.y) / 2 * innerHeight }; };
    let screwDrag = null, wrongT = 0;
    const out = () => S.stage === "cover" || S.stage === "close";
    const turnDir = () => (S.stage === "cover" ? 1 : -1);
    const leftScrews = () => screws.filter((sc) => (S.stage === "cover" ? sc.t < TURN : sc.t > 0));
    dom.addEventListener("pointerdown", (e) => {
      if (!docked() || e.button !== 0) return;
      if (out()) {
        let best = null, bd = 70;
        for (const sc of leftScrews()) { const q = screwPx(sc), dd = Math.hypot(q.x - e.clientX, q.y - e.clientY); if (dd < bd) { bd = dd; best = sc; } }
        if (best) { const q = screwPx(best); screwDrag = { sc: best, a: Math.atan2(-(e.clientY - q.y), e.clientX - q.x) }; dom.setPointerCapture(e.pointerId); }
        return;
      }
      if (S.stage === "pull" && pick(e, card)) { S.stage = "pulling"; say(`${PARTS.burnt.name} ${PARTS.burnt.rating}: burnt`, "#ff4757"); emit(card.getWorldPosition(new THREE.Vector3()), 10, { col: [0.5, 0.5, 0.55], speed: 0.5, up: 0.6, grav: 0.6, life: 1.4 }); return; }
      if (S.stage === "calibrate" && pick(e, board)) { openGame(); }
    });
    dom.addEventListener("pointermove", (e) => {
      if (!docked()) return;
      if (screwDrag && out()) {
        const q = screwPx(screwDrag.sc), a = Math.atan2(-(e.clientY - q.y), e.clientX - q.x);
        let d = a - screwDrag.a; d = Math.atan2(Math.sin(d), Math.cos(d)); screwDrag.a = a;
        if (d * turnDir() > 0) { const sc = screwDrag.sc, b = sc.t; sc.t = Math.max(0, Math.min(TURN, sc.t + d)); if (S.stage === "cover" && b < TURN && sc.t >= TURN) emit(new THREE.Vector3().setFromMatrixPosition(sc.g.matrixWorld), 4, { col: [0.7, 0.75, 0.8], speed: 0.6, up: 0.3, life: 0.4 }); }
        else if (Math.abs(d) > 0.05) wrongT = 0.5;
        return;
      }
      const hot = (S.stage === "pull" && pick(e, card)) || (S.stage === "calibrate" && pick(e, board));
      document.body.classList.toggle("sr-pick", hot);
      frameMat.color.set(S.stage === "calibrate" && hot ? 0xf2a046 : 0x2b3540);
    });
    addEventListener("pointerup", () => { screwDrag = null; });
    addEventListener("keydown", (e) => {
      if (e.code === "KeyE" && nearby()) { e.stopPropagation(); dock(); return; }
      if (e.code === "Escape" && S.docked && S.docked.dir > 0) { e.stopPropagation(); if (S.stage === "game") closeGame(); else undock(); }
    }, true);

    // ---------------------------------------------------------------- the tray: the burnt card and the pouch (3e)
    function showTray() {
      tray.style.display = "block";
      const outCard = `<h4>Removed</h4><div class="sr-card out"><b>${PARTS.burnt.rating}</b><small>${PARTS.burnt.name}, ${PARTS.burnt.note}</small></div>`;
      const pouch = PARTS.pouch.map((p, i) => `<button class="sr-card${S.tried.has(p.id) ? " bad" : ""}" data-i="${i}"><b>${p.rating}</b><small>${p.name}${S.tried.has(p.id) ? ", failed its test" : ""}</small></button>`).join("");
      tray.innerHTML = outCard + `<h4>Pouch</h4>` + pouch;
      for (const b of tray.querySelectorAll("button")) b.onclick = () => fit(PARTS.pouch[+b.dataset.i]);
    }
    function fit(p) {
      if (S.stage !== "fit") return;
      card.visible = true; card.material = goodMat; ember.visible = false; card.position.copy(cardSeat);
      if (!p.right) {
        // Seated, tested, failed: a spark, and it comes back out to the pouch, marked (3e).
        S.tried.add(p.id);
        emit(card.getWorldPosition(new THREE.Vector3()), 24, { speed: 2.6 }); flashT = 0.25;
        say(`Wrong part: ${p.rating} fails its test`, "#ff4757");
        setTimeout(() => { if (S.stage === "fit") card.visible = false; }, 450);
        showTray();
        return;
      }
      S.fitted = p; S.stage = "calibrate"; tray.style.display = "none";
      say(`${p.rating} fitted: calibrate it`, "#3ddc84");
    }

    // ---------------------------------------------------------------- the 2D game: the calibration (3e step 4)
    let seen = null;
    function openGame() {
      if (!runner) { finish(); return; }
      S.stage = "game"; document.body.classList.remove("sr-pick");
      if (!runner.run || runner._job !== "station") {
        runner.opts.state = S.value < 25 ? "disabled" : "damaged"; runner.opts.who = "officer";
        runner.open(o.game || "conduits"); runner._job = "station";
        if (runner.run.panel && runner.run.panel.phase !== "work") { runner.run.panel.phase = "work"; runner.run.panel.lift = 1; }
      }
      if (runner.guide && runner.guide.open) runner.guide.hide();
      runner.opts.paused = false;
      seen = { step: runner.run.step, fumbles: runner.run.fumblesTotal };
      panelEl.style.display = "block"; dimEl.style.opacity = 1;
      bar.innerHTML = `<span class="k">Esc</span> put it away`;
    }
    function closeGame() {
      if (runner) runner.opts.paused = true;
      panelEl.style.display = "none"; dimEl.style.opacity = 0;
      if (S.stage === "game") S.stage = "calibrate";
      bar.innerHTML = `<span class="k">Esc</span> stand up`;
    }
    function watchGame() {
      if (S.stage !== "game" || !runner || !runner.run) return;
      const r = runner.run;
      S.value = r.value;
      if (r.step > seen.step && r.step < r.job.steps) { say(`Round ${r.step} landed: ${Math.round(r.value)}%`, "#3ddc84"); emit(target.clone().addScaledVector(normal, 0.1), 20, { col: [0.6, 1, 0.75], speed: 1.2, up: 1.4, grav: -2, life: 0.9 }); }
      if (r.fumblesTotal > seen.fumbles) { say(r.note || r.game.hazard || "A spark", "#ff4757"); emit(target.clone().addScaledVector(normal, 0.08), 40, { speed: 3.4, up: 2 }); flashT = 0.4; }
      seen = { step: r.step, fumbles: r.fumblesTotal };
      if (r.step >= r.job.steps) finish();
    }
    function finish() {
      closeGame();
      S.stage = "close"; S.value = 100;
      for (const sc of screws) sc.t = TURN;
      say("Last round landed: cover back on", "#3ddc84");
    }

    // ---------------------------------------------------------------- each frame
    let perf = 0, flashT = 0;
    const flashAt = toShip([0, 0.7, 0.45]);
    function update(dt) {
      perf += dt;
      dt = Math.min(dt, 0.1);
      camStep(dt);
      // The prompt, walking up.
      const near = nearby();
      promptEl.style.display = near ? "block" : "none";
      if (near) promptEl.innerHTML = `<span class="k">E</span>${name}: Repair, ${S.state}`;
      // The cover: worked while docked, lifting off, going back on (3a).
      if (S.stage === "cover" && docked() && !leftScrews().length) S.stage = "lift";
      if (S.stage === "lift") { S.ca = Math.min(1, S.ca + dt / 0.6); if (S.ca >= 1) { S.stage = "pull"; say("Cover off: pull the burnt card", "#9fb0c0"); } }
      if (S.stage === "close") {
        S.ca = Math.max(0, S.ca - dt / 0.6);
        if (S.ca <= 0) {
          // Its screws are driven home on their own here: the job is done.
          for (const sc of screws) sc.t = Math.max(0, sc.t - dt * TURN * 1.5);
          if (screws.every((sc) => sc.t <= 0)) {
            S.stage = "done"; S.state = "repaired"; S.boot = 1;
            if (o.onChange) o.onChange();
            say(`${name}: repaired`, "#3ddc84");
            setTimeout(() => { if (S.docked && S.docked.dir > 0) undock(); }, 1400);
          }
        }
      }
      if (S.stage === "pulling") {
        S.cardOut = Math.min(1, S.cardOut + dt / 0.5);
        card.position.copy(cardSeat).add(new THREE.Vector3(0, 0, 0.12 * ease(S.cardOut)));
        if (S.cardOut >= 1) { card.visible = false; S.stage = "fit"; showTray(); say("Fit the right card from the pouch", "#9fb0c0"); }
      }
      const u = ease(S.ca);
      cover.position.lerpVectors(coverHome.p, coverOff.p, u);
      cover.quaternion.slerpQuaternions(coverHome.q, coverOff.q, u);
      for (const sc of screws) {
        sc.g.rotation.z = sc.t;
        sc.g.position.z = sc.g.userData.home + 0.03 * Math.min(1, sc.t / TURN);
        sc.g.visible = !(sc.t >= TURN && (S.stage === "lift" || S.stage === "pull" || S.stage === "pulling" || S.stage === "fit" || S.stage === "calibrate" || S.stage === "game"));
      }
      bay.visible = S.ca > 0.02;
      ember.material.color.setHSL(0.06, 1, 0.45 + 0.15 * Math.sin(perf * 13));
      watchGame();
      // The damage the room shows (3f).
      if (S.state !== "repaired") {
        S.sparkT -= dt;
        if (S.sparkT <= 0) {
          const at = toShip([(rng() - 0.5) * 1.0, 0.72, 0.5]);
          emit(at, 22 + Math.floor(rng() * 18), { speed: 2.8 });
          for (let k = 0; k < 3; k++) puff(at, { rise: 0.45, life: 2.2, grey: 0.22, size: 0.12, grow: 0.45, dense: 0.5 });
          flash.position.copy(at); flashT = 0.18;
          S.sparkT = S.sparkEvery || 1 + rng() * 3;
        }
        // A thin wisp from the seam under the desk while damaged, a plume from it and the back panel while disabled.
        const wisp = S.state === "disabled" ? 7 : 2.2;
        if (rng() < dt * wisp) puff(toShip([(rng() - 0.5) * 0.5, 0.72, 0.48]), S.state === "disabled" ? { grey: 0.1, dense: 0.7, size: 0.18, grow: 0.8, life: 4 } : {});
        if (S.state === "disabled" && rng() < dt * 3) puff(toShip([(rng() - 0.5) * 0.3, 0.45, -0.12]), { grey: 0.1, dense: 0.6, size: 0.16, grow: 0.7, life: 3.6 });
        // Now and then a single spark off the screen's broken corner.
        if (rng() < dt * 0.8) emit(toShip([-0.38, 0.9, 0.15]), 3, { speed: 1.2, up: 0.6 });
        if (S.stage === "pull" || S.stage === "pulling") if (rng() < dt * 4) puff(card.getWorldPosition(new THREE.Vector3()), { rise: 0.18, life: 2, grey: 0.2, size: 0.03, grow: 0.12, dense: 0.6 });
      }
      flashT = Math.max(0, flashT - dt);
      flash.material.opacity = Math.min(1, flashT * 6);
      if (flashT <= 0 && S.state === "repaired") flash.position.copy(flashAt);
      stepParticles(dt);
      stepSmoke(dt);
      // The screen: static while damaged, a dim flicker while disabled, a boot flash then gone once repaired.
      if (screen) {
        if (S.state === "repaired" && S.boot <= 0) screen.visible = false;
        else {
          screen.visible = true;
          if (S.state === "repaired") { S.boot = Math.max(0, S.boot - dt / 1.2); drawStatic(0.6 * S.boot, S.boot); screenMat.opacity = Math.min(1, S.boot * 1.5); }
          else {
            const flick = S.state === "disabled" ? (rng() < 0.08 ? 0.5 : 0.05) : (Math.sin(perf * 23) > 0.2 ? 1 : 0.35);
            drawStatic(flick, 0); screenMat.opacity = 0.96;
          }
        }
      }
      // The screws' rings (3a).
      drawRings(dt);
      if (eventT > 0) { eventT -= dt; if (eventT <= 0) eventEl.style.opacity = 0; }
    }
    function drawRings(dt) {
      if (ov.width !== innerWidth || ov.height !== innerHeight) { ov.width = innerWidth; ov.height = innerHeight; }
      og.clearRect(0, 0, ov.width, ov.height);
      wrongT = Math.max(0, wrongT - dt);
      const show = docked() && S.stage === "cover";
      ov.style.visibility = show ? "visible" : "hidden";
      if (!show) return;
      for (const sc of leftScrews()) {
        const { x, y } = screwPx(sc), f = sc.t / TURN;
        og.lineWidth = 4; og.strokeStyle = "rgba(20,26,36,0.85)"; og.beginPath(); og.arc(x, y, 24, 0, Math.PI * 2); og.stroke();
        og.strokeStyle = wrongT > 0 ? "#ff4757" : "#f2a046";
        og.beginPath(); og.arc(x, y, 24, -Math.PI / 2, -Math.PI / 2 - f * Math.PI * 2, true); og.stroke();
        og.fillStyle = og.strokeStyle; og.beginPath(); og.moveTo(x - 8, y - 32); og.lineTo(x - 8, y - 16); og.lineTo(x - 20, y - 24); og.fill();
      }
    }
    /** The room's lighting state (0 normal, 1 red alert, 2 emergency): the cover and board are lit by it. */
    function setLight(i) {
      const t = TINTS[i] || TINTS[0];
      coverMat.color.setRGB(...t); boardMat.color.setRGB(...t);
    }
    setLight(0);
    const own = [decals, screen, cover, bay, sparks, flash, smoke].filter(Boolean);
    /** Show or hide everything it adds to the scene (a page's plan views); the damage keeps going either way. */
    function setVisible(v) { for (const x of own) x.visible = v; if (!v) { promptEl.style.display = "none"; } }
    return {
      update, damage, setLight, undock, setVisible,
      /** The state: "damaged", "disabled" or "repaired"; the stage of a repair in hand; its integrity, percent. */
      status: () => ({ state: S.state, stage: S.stage, value: Math.round(S.value), docked: !!S.docked }),
      /** Where the body stands to dock (feet, ship coordinates) and the point it faces: for tools and tests. */
      dockPoint: () => ({ x: point.x, y: point.y, z: point.z, face: [target.x - point.x, target.z - point.z] }),
      /** A place to walk up from: dist metres out from the console's front (the operator's side, where its screen and
       * sparks are seen), off to one side, facing the desk. */
      approach: (dist) => {
        const p = toShip([0.55, 0, 0.3 + dist]), c0 = toShip([0, 0, 0.3]);
        return { x: p.x, y: back.y, z: p.z, face: [c0.x - p.x, c0.z - p.z] };
      },
      /** For tools (a measurement instrument, CLAUDE.md 4): play the repair's steps without a pointer. */
      test: {
        dock: () => { if (!S.docked) dock(); S.docked.t = 1; camStep(0); },
        unscrew: () => { for (const sc of screws) sc.t = TURN; S.ca = 1; S.stage = "pull"; },
        pull: () => { if (S.stage === "pull") { S.stage = "pulling"; S.cardOut = 1; } },
        fit: (id) => fit(PARTS.pouch.find((p) => p.id === id)),
        open: () => openGame(),
        finish: () => { finish(); S.ca = 0; for (const sc of screws) sc.t = 0; },
        /** Spark every this many seconds (0: the usual 1-4 s), so a shot catches a burst. */
        sparkEvery: (sec) => { S.sparkEvery = sec; S.sparkT = 0; },
      },
    };
  }

  window.StationRepair = { create, PARTS };
})();
