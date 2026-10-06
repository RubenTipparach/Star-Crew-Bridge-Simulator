/*
 * freecam.js: one free camera for the Star Crew interior mockups.
 *
 * What it owns: letting the viewer move the camera themselves, from wherever a page's fixed view
 * put it (owner, 2026-10-05: "I want to be able to look around, or orbit camera or pan around
 * instead of fixed camera angles too"). Two modes:
 *   - orbit: drag to turn around a point, right-drag (or Shift-drag) to pan, wheel to zoom. This is
 *     three.js's OrbitControls, passed in by the page so this file needs no imports. W A S D or the
 *     arrow keys slide the camera and its point together, along the floor.
 *   - look: drag to look around from where the camera stands, W A S D or the arrow keys to move,
 *     Q and E down and up, Shift to go faster, the wheel to step forward and back. setLook({ walk:
 *     true }) makes it a walk: the eye stays eye_m over the page's floorAt(x, z) and never leaves
 *     its walkable(x, z).
 * Speeds are crew-on-deck's (walk 1.8 m/s, run 4.0 m/s), so a walk takes as long as the game's.
 * A page's fixed views stay: a view button puts the camera back, and the viewer takes over again.
 *
 * A walk that changes decks (stairs, ladders, a lift, collision with the rooms as drawn) is shipwalk.js; this
 * walk keeps one floor, for the one-floor pages, until they move onto that one.
 * Why a lib of its own: the bridge page's walk mode and each page's orbit did this on their own;
 * this is the one copy (CLAUDE.md 6.1), inlined by tools/mockups/inline.py between the
 * "INLINE lib:freecam" markers. Edit it here, never inside a page.
 */
(function () {
  const WALK_M_S = 1.8;            // crew-on-deck: walking speed
  const RUN_M_S = 4.0;             // crew-on-deck: running speed (Shift)
  const LOOK_RAD_PER_PX = 0.0035;  // drag to look
  const PITCH_LIMIT_RAD = 1.45;
  const WHEEL_STEP_M = 0.4;        // look mode: one wheel notch
  const ORBIT_START_M = 4.0;       // switching from look to orbit: the point is this far ahead

  /**
   * Make a free camera for `camera` on canvas `dom`. opts (all optional):
   *   walkable(x, z) -> bool: where a walk may go (default: anywhere);
   *   floorAt(x, z) -> y: with eye_m, where a walk keeps the eye (default: it keeps its height);
   *   eye_m: eye height over floorAt (default 1.65, a standing eye);
   *   onDrag(): called when the viewer starts moving the camera, so a page can mark its view as free.
   * Returns { update(dt_s), setOrbit([x, y, z]), toOrbit(), setLook({ walk }), setOff(), mode(), dragged(), orbit }.
   * Look mode never moves the camera by itself, so a page may still place it for a fixed view.
   * setOff() hands the camera back to the page (a seated console view, a fixed shot).
   */
  function create(THREE, camera, dom, OrbitControls, opts) {
    opts = opts || {};
    const eye = opts.eye_m === undefined ? 1.65 : opts.eye_m;
    const orbit = new OrbitControls(camera, dom);
    orbit.enableDamping = false;
    orbit.screenSpacePanning = true;
    const keys = {};
    let mode = "orbit", walking = false, yaw = 0, pitch = 0, drag = null, dragDist = 0;

    function syncAngles() {
      const e = new THREE.Euler().setFromQuaternion(camera.quaternion, "YXZ");
      pitch = e.x; yaw = e.y;
    }
    function applyAngles() { camera.rotation.set(pitch, yaw, 0, "YXZ"); }
    function forward() { return new THREE.Vector3(-Math.sin(yaw), 0, -Math.cos(yaw)); }
    function settle() {
      if (walking && opts.floorAt) camera.position.y = opts.floorAt(camera.position.x, camera.position.z) + eye;
    }
    function tryMove(d) {
      const n = camera.position.clone().add(d);
      if (walking && opts.walkable && !opts.walkable(n.x, n.z)) return false;
      camera.position.copy(n);
      settle();
      return true;
    }

    const fc = {
      orbit,
      mode: () => mode,
      /** True once the viewer has dragged or keyed the camera since the last fixed view. */
      dragged: () => dragDist > 3,
      /** Orbit around target, keeping the camera where it is. */
      setOrbit(target) {
        mode = "orbit"; orbit.enabled = true; dragDist = 0;
        orbit.target.set(target[0], target[1], target[2]);
        orbit.update();
      },
      /** Orbit around the point ORBIT_START_M ahead of where the camera looks. */
      toOrbit() {
        const d = new THREE.Vector3(); camera.getWorldDirection(d);
        const t = camera.position.clone().addScaledVector(d, ORBIT_START_M);
        fc.setOrbit([t.x, t.y, t.z]);
      },
      /** Look around from where the camera stands; { walk: true } keeps it on the floor. */
      setLook(o) {
        mode = "look"; walking = !!(o && o.walk); orbit.enabled = false; dragDist = 0;
        syncAngles(); settle(); applyAngles();
      },
      /** Leave the camera to the page: no orbit, no look, no keys. */
      setOff() { mode = "off"; orbit.enabled = false; drag = null; },
      /** Move by the held keys; call once a frame with the frame's seconds. */
      update(dt) {
        if (mode === "off") return;
        const fast = keys.ShiftLeft || keys.ShiftRight;
        if (mode === "look") syncAngles();
        const f = mode === "look" ? forward() : (() => { const d = new THREE.Vector3(); camera.getWorldDirection(d); d.y = 0; return d.lengthSq() > 1e-9 ? d.normalize() : new THREE.Vector3(0, 0, -1); })();
        const r = new THREE.Vector3(-f.z, 0, f.x);
        const d = new THREE.Vector3();
        if (keys.KeyW || keys.ArrowUp) d.add(f);
        if (keys.KeyS || keys.ArrowDown) d.sub(f);
        if (keys.KeyD || keys.ArrowRight) d.add(r);
        if (keys.KeyA || keys.ArrowLeft) d.sub(r);
        let up = 0;
        if (!(walking && opts.floorAt)) { if (keys.KeyE) up += 1; if (keys.KeyQ) up -= 1; }
        if (d.lengthSq() > 0 || up !== 0) {
          if (d.lengthSq() > 0) d.normalize();
          d.y = up;
          if (mode === "orbit") {
            // Slide the camera and its point together, faster the further out the camera is.
            const dist = camera.position.distanceTo(orbit.target);
            d.multiplyScalar(Math.max(2.0, 0.6 * dist) * (fast ? 2 : 1) * dt);
            camera.position.add(d); orbit.target.add(d);
          } else {
            d.multiplyScalar((fast ? RUN_M_S : WALK_M_S) * dt);
            tryMove(d);
          }
          dragDist += 10;
        }
        if (mode === "orbit") orbit.update();
      },
    };

    dom.addEventListener("pointerdown", (e) => {
      if (mode === "off") return;
      dragDist = 0;
      if (mode !== "look" || e.button !== 0) return;
      syncAngles();
      drag = { x: e.clientX, y: e.clientY };
      if (dom.setPointerCapture) dom.setPointerCapture(e.pointerId);
    });
    dom.addEventListener("pointermove", (e) => {
      if (!drag) { if (mode === "orbit" && e.buttons) { dragDist += Math.abs(e.movementX) + Math.abs(e.movementY); if (dragDist > 3 && opts.onDrag) opts.onDrag(); } return; }
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      drag = { x: e.clientX, y: e.clientY };
      dragDist += Math.abs(dx) + Math.abs(dy);
      if (dragDist > 3 && opts.onDrag) opts.onDrag();
      yaw -= dx * LOOK_RAD_PER_PX;
      pitch = Math.max(-PITCH_LIMIT_RAD, Math.min(PITCH_LIMIT_RAD, pitch - dy * LOOK_RAD_PER_PX));
      applyAngles();
    });
    const endDrag = (e) => { if (drag && dom.releasePointerCapture) { try { dom.releasePointerCapture(e.pointerId); } catch (_) { /* already released */ } } drag = null; };
    dom.addEventListener("pointerup", endDrag);
    dom.addEventListener("pointercancel", endDrag);
    dom.addEventListener("wheel", (e) => {
      if (mode !== "look") return;
      e.preventDefault();
      const d = new THREE.Vector3(); camera.getWorldDirection(d);
      if (walking && opts.floorAt) { d.y = 0; d.normalize(); }
      tryMove(d.multiplyScalar(e.deltaY < 0 ? WHEEL_STEP_M : -WHEEL_STEP_M));
      if (opts.onDrag) opts.onDrag();
    }, { passive: false });
    const typing = (e) => e.target && (e.target.tagName === "INPUT" || e.target.tagName === "SELECT" || e.target.tagName === "TEXTAREA");
    addEventListener("keydown", (e) => {
      if (typing(e) || mode === "off") return;
      keys[e.code] = true;
      if (/^Arrow/.test(e.code)) e.preventDefault();
      if (opts.onDrag && /^(Key[WASDQE]|Arrow)/.test(e.code)) opts.onDrag();
    });
    addEventListener("keyup", (e) => { keys[e.code] = false; });
    addEventListener("blur", () => { for (const k of Object.keys(keys)) keys[k] = false; });
    return fc;
  }

  window.FreeCam = { create, WALK_M_S, RUN_M_S };
})();
