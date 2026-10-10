/* Furnished command-suite rooms shared by the command-deck and exterior previews.
 * All positions come from the same patched layout; ShipKit and PropKit own geometry and materials.
 */
(function () {
  "use strict";
  /** Create a room builder. parent receives the room meshes; material is the shared baked surface. */
  function create(THREE, K, PK, L, S, D, pmats, { parent: scene, material: baked }) {
    const FLOOR = L.decks.find(d => d.id === "A").floor_y_m;
    const SHELL_ROLES = ["floor", "wall", "ceiling", "cove", "platform"];
    const STATES = ["normal", "red_alert", "emergency"];
    const { baseOf, addFaces, faceMat, operatorZ, placeProp, placeCrew, mergeParts, roleColor } = PK;
    const UPHOLSTERY = { ready_room: "captain", captains_quarters: "captain" };
    const accentOf = room => new THREE.Color(K.PALETTE.furniture[UPHOLSTERY[room] || "crew"]);
    const facesOf = () => "captain";
    /** What keeps a wall's ribs and trims clear: the bridge's wall banks and viewscreen, and every piece of furniture's back. */
    function wallFixtures(cid) {
      const out = [];
      const add = (back, yaw, w) => { const th = (yaw * Math.PI) / 180; out.push({ center: [back[0], back[1] + 1.0, back[2]], normal: [Math.sin(th), 0, Math.cos(th)], width: w + 0.5 }); };
      if (cid === "bridge") {
        for (const p of S.bridge.props) {
          if (!p.prop.startsWith("wall_bank")) continue;
          add(p.back_m, p.yaw_deg, { wall_bank_core: 1.4, wall_bank: 1.1, wall_bank_double: 2.4 }[baseOf(p.prop)]);
        }
        const vs = K.byId(L.fixtures, "viewscreen");
        out.push({ center: vs.center_m, normal: [0, 0, -1], width: vs.size_m[0] + 0.6 });
      }
      for (const f of S.furnishings) if (f.room === cid && f.set === "suite") add(f.back_m, f.yaw_deg, S.prop_brief_m[f.prop][0]);
      return out;
    }

    /** One room as one mesh (one draw), its lamps baked in the three lighting states, and its console faces as a second mesh. */
    function buildRoom(cid) {
      const comp = K.compartment(L, cid);
      // The bridge's platforms and the bands at their feet hide the floor under them (floor-panels design 6).
      const room = (cid === "bridge" ? comp.brushes.find((b) => Math.abs(b.y[0] - FLOOR) < 0.3) : null);
      const edgeOpts = room ? { edgeLayer: K.panelLayerName(comp.finish, "floor_edges"), room: room.poly } : {};
      const built = K.buildCompartment(THREE, L, comp, { detailing: D, wallFixtures: wallFixtures(cid), panels: true,
        floorCovers: room ? K.platformCovers(S.bridge.platforms, FLOOR, edgeOpts) : [] });
      const parts = built.parts, F = { position: [], normal: [], uv: [] }, lamps = built.lamps.slice();
      const glow = (back, yaw, h, station) => {
        const th = (yaw * Math.PI) / 180, c = roleColor(station);
        lamps.push({ p: [back[0] + Math.sin(th) * 0.5, back[1] + h, back[2] + Math.cos(th) * 0.5], R: 1.3, I: 0.5, fixed: true, color: [c.r * 0.6, c.g * 0.6, c.b * 0.6] });
      };
      if (cid === "bridge") {
        const B = new K.Builder();
        K.buildPlatforms(THREE, B, S.bridge.platforms, S.bridge.stairs, FLOOR, D, Object.assign({ topLayer: K.panelLayerName(comp.finish, "walkway", "floor"), riserLayer: K.panelLayerName(comp.finish, "platform") }, edgeOpts));   // the floors' tread on top, risers fitted (ceilings-and-trims 9)
        mergeParts(parts, B.parts);
        for (const p of S.bridge.props) {
          placeProp(parts, p.prop, p.back_m, p.yaw_deg, p.station);
          addFaces(F, p.prop, p.back_m, p.yaw_deg, p.station);
          if (!p.prop.includes("chair")) glow(p.back_m, p.yaw_deg, p.prop.startsWith("wall_bank") ? 1.5 : 1.0, p.station);
        }
        for (const s of S.bridge.seats) {
          if (s.station !== "captain") {
            const th = (s.yaw_deg * Math.PI) / 180, c = operatorZ("crew_chair", 0.3);
            placeProp(parts, "crew_chair", [s.seat_m[0] - Math.sin(th) * c, s.seat_m[1], s.seat_m[2] - Math.cos(th) * c], s.yaw_deg, s.station);
          }
          placeCrew(parts, s.seat_m, s.yaw_deg, s.station);
        }
      }
      let pieces = 0;
      for (const f of S.furnishings) {
        if (f.room !== cid) continue;
        pieces++;
        placeProp(parts, f.prop, f.back_m, f.yaw_deg, "furniture", { accent: accentOf(cid) });
        addFaces(F, f.prop, f.back_m, f.yaw_deg, facesOf(f));
        if (f.prop === "desk" || f.prop === "wall_screen") glow(f.back_m, f.yaw_deg, f.prop === "desk" ? 1.0 : 1.8, "captain");
      }
      K.subdivideParts(parts, 0.9, SHELL_ROLES);
      const geo = K.geometryOf(THREE, parts, pmats, K.finishOf(comp, D));
      const sets = {};
      for (const k of STATES) { K.bakeDirect(THREE, geo, lamps, k); sets[k] = geo.attributes.color.array.slice(); }
      const mesh = new THREE.Mesh(geo, baked);
      scene.add(mesh);
      let faces = null, faceSets = null;
      if (F.position.length) {
        const fg = new THREE.BufferGeometry();
        fg.setAttribute("position", new THREE.Float32BufferAttribute(F.position, 3));
        fg.setAttribute("normal", new THREE.Float32BufferAttribute(F.normal, 3));
        fg.setAttribute("uv", new THREE.Float32BufferAttribute(F.uv, 2));
        fg.setAttribute("color", new THREE.Float32BufferAttribute(new Float32Array(F.position.length).fill(1), 3));
        fg.userData.roles = {};
        faceSets = {};
        for (const k of STATES) { K.bakeDirect(THREE, fg, lamps, k); faceSets[k] = fg.attributes.color.array.slice(); }
        faces = new THREE.Mesh(fg, faceMat);
        scene.add(faces);
      }
      return { cid, comp, built, mesh, sets, faces, faceSets, pieces, tris: geo.attributes.position.count / 3, faceTris: faces ? faces.geometry.attributes.position.count / 3 : 0 };
    }

    return { buildRoom };
  }
  window.CommandRooms = { create };
})();
