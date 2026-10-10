/* The Tern exterior preview's furnished rooms and window reveals, in ship-local metres.
 * CommandRooms owns the shared furnishing and lighting; the GLB manifest supplies hull intersections.
 */
(function () {
  "use strict";
  /** Verify the delivered GLB opens onto the same portals used by the actual rooms. */
  function validateWindows(THREE, model, interior) {
    model.updateMatrixWorld(true);
    let rays=0;
    for(const w of interior.windows){
      if(!(w.depth_m>=0&&w.depth_m<=w.max_depth_m))throw new Error(`${w.id}: excessive hull-to-room depth`);
      const p=interior.L.portals.find(p=>p.id===w.id);
      for(const k of ["center_m","normal","size_m"]){
        if(JSON.stringify(w[k])!==JSON.stringify(p[k]))throw new Error(`${w.id}: exported ${k} differs from interior`);
      }
      const n=new THREE.Vector3(...w.normal).normalize(),t=new THREE.Vector3(n.z,0,-n.x),c=new THREE.Vector3(...w.center_m);
      for(const [i,[u,v]] of [[-1,-1],[1,-1],[1,1],[-1,1]].entries()){
        for(const corners of [w.inner_corners_m,w.outer_corners_m]){
          const q=new THREE.Vector3(...corners[i]).sub(c);
          if(Math.abs(q.dot(t)-u*w.size_m[0]/2)>.001||Math.abs(q.y-v*w.size_m[1]/2)>.001)
            throw new Error(`${w.id}: liner edge does not meet the aperture`);
          if(q.dot(n)<0||q.dot(n)>w.max_depth_m)throw new Error(`${w.id}: deep aperture corner`);
        }
      }
      for(const u of [-.49,0,.49])for(const v of [-.49,0,.49]){
        const origin=c.clone().addScaledVector(n,w.depth_m+4).addScaledVector(t,u*w.size_m[0]);origin.y+=v*w.size_m[1];
        const ray=new THREE.Raycaster(origin,n.clone().negate(),0,w.depth_m+3.98);
        if(ray.intersectObject(model,true).length)throw new Error(`${w.id}: delivered hull blocks aperture at ${u}, ${v}`);
        rays++;
      }
      for(const side of [-1,1]){
        const origin=new THREE.Vector3(...w.outer_center_m).addScaledVector(n,2).addScaledVector(t,side*w.size_m[0]*.7);
        const ray=new THREE.Raycaster(origin,n.clone().negate(),0,4);
        if(!ray.intersectObject(model,true).length)throw new Error(`${w.id}: missing hull beside aperture`);
        rays++;
      }
      // Require a real frame directly outside each opening in the delivered mesh.
      for(const side of [-1,1])for(const fraction of [-.3,0,.3])for(const axis of [0,1]){
        const offsets=[fraction*w.size_m[0],fraction*w.size_m[1]];
        offsets[axis]=side*(w.size_m[axis]+w.rim_width_m)/2;
        const origin=c.clone().addScaledVector(n,w.depth_m+4).addScaledVector(t,offsets[0]);origin.y+=offsets[1];
        const ray=new THREE.Raycaster(origin,n.clone().negate(),0,w.depth_m+4);
        const hit=ray.intersectObject(model,true)[0];
        const mat=hit&&(Array.isArray(hit.object.material)?hit.object.material[hit.face.materialIndex]:hit.object.material);
        if(!mat||mat.name!==(w.rim_material||"accent"))throw new Error(`${w.id}: physical rim missing at edge ${axis}, ${side}, ${fraction}`);
        rays++;
      }
    }
    return rays;
  }
  /** Build the same furnished rooms as command-deck, without opaque scenery over its windows. */
  async function create(THREE, K, GLTFLoader, scene, windows) {
    const W = K.shipData("exterior_windows"), S0 = K.shipData("command_suite"), D = K.shipData("detailing");
    const S = {...S0,furnishings:[...S0.furnishings,...W.furnishings]}, L = K.applyPatch(K.applyPatch(K.layout(), S), W);
    const base = await K.loadMaterials(THREE);
    const mats = await K.loadPanels(THREE, base, { px: 128, atlasSets: ["bridge", "suite"] });
    base.texture.dispose();
    const PK = await window.PropKit.create(THREE, K, GLTFLoader, mats, { sets: ["bridge", "suite"], sizes: S.prop_brief_m });
    const group = new THREE.Group(); group.name = "Tern furnished decks"; scene.add(group);
    const material = K.surfaceMaterial(THREE, mats, { lit: false });
    const { buildRoom } = window.CommandRooms.create(THREE, K, PK, L, S, D, mats, { parent: group, material });
    const roomIds = [...new Set(["bridge", "ready_room", "briefing_room", "captains_quarters", "computer_core", "head", "bridge_locker", "a_corridor", "b_spine", "c_spine",...windows.map(w=>w.room)])];
    const rooms = roomIds.map(buildRoom);
    const screens = PK.spaceViews(L, { keep: f => f.kind === "viewscreen" }); group.add(screens);
    const reveals = new THREE.Group(); reveals.name = "Window passages and glazing"; group.add(reveals);
    const revealMat = new THREE.MeshBasicMaterial({ vertexColors: true, side: THREE.DoubleSide });
    const glassMat = new THREE.MeshBasicMaterial({ color: 0x98d8e8, transparent: true, opacity: 0.065, depthWrite: false, side: THREE.DoubleSide });
    const positions = [], colours = [], glassPositions = [], steel = new THREE.Color(0x56636d);
    for (const w of windows) {
      const n = new THREE.Vector3(...w.normal), c = new THREE.Vector3(...w.center_m), t = new THREE.Vector3(n.z, 0, -n.x);
      const inner = [[-1,-1],[1,-1],[1,1],[-1,1]].map(([u,v]) => c.clone().addScaledVector(t,u*w.size_m[0]/2).add(new THREE.Vector3(0,v*w.size_m[1]/2,0)));
      const outer = w.inner_corners_m.map(p => new THREE.Vector3(...p));
      for (let i=0;i<4;i++) {
        const j=(i+1)%4;
        // Face shade and depth falloff make the real passage read as a recess from outside.
        for (const [p,depth] of [[inner[i],.5],[outer[i],1],[outer[j],1],[inner[i],.5],[outer[j],1],[inner[j],.5]]) {
          positions.push(...p);
          colours.push(...steel.clone().multiplyScalar([1,.5,.25,.65][i]*depth));
        }
      }
      for(const i of [0,1,2,0,2,3])glassPositions.push(...inner[i].clone().addScaledVector(n,.06));
    }
    // All windows share two batches, keeping new decks within the draw-call reservation.
    const geo=new THREE.BufferGeometry();geo.setAttribute("position",new THREE.Float32BufferAttribute(positions,3));geo.setAttribute("color",new THREE.Float32BufferAttribute(colours,3));geo.computeVertexNormals();
    const reveal=new THREE.Mesh(geo,revealMat);reveal.name="Window liners";reveals.add(reveal);
    const glassGeo=new THREE.BufferGeometry();glassGeo.setAttribute("position",new THREE.Float32BufferAttribute(glassPositions,3));glassGeo.computeVertexNormals();
    const panes=new THREE.Mesh(glassGeo,glassMat);panes.name="Window glazing";reveals.add(panes);
    const clip = new THREE.Plane(new THREE.Vector3(0,-1,0),6.2);
    const materials=[material,PK.faceMat,revealMat,glassMat,...screens.children.map(m=>m.material)];
    let light="normal",cut=false;
    function lighting(key) {
      light=key;
      material.userData.panelGlow.value=K.panelsData().manifest.glow[key];
      for(const r of rooms) {
        r.mesh.geometry.attributes.color.array.set(r.sets[key]);r.mesh.geometry.attributes.color.needsUpdate=true;
        if(r.faces){r.faces.geometry.attributes.color.array.set(r.faceSets[key]);r.faces.geometry.attributes.color.needsUpdate=true;}
      }
    }
    function cutaway(value,deck="A") {
      cut=value;
      clip.constant=L.decks.find(d=>d.id===deck).floor_y_m+2.7;
      for(const r of rooms){r.mesh.visible=!cut||r.comp.decks.includes(deck);if(r.faces)r.faces.visible=r.mesh.visible;}
      screens.visible=!cut||deck==="A";
      reveals.visible=!cut;
      for(const m of materials){m.clippingPlanes=cut?[clip]:[];m.needsUpdate=true;}
    }
    lighting(light);
    const triangles=rooms.reduce((a,r)=>a+r.tris+r.faceTris,0)+windows.length*10+screens.children.length*2;
    if(triangles>120000)throw new Error(`Furnished-deck preview uses ${triangles} triangles, above 120000`);
    return { group, rooms, L, windows, triangles, lighting, cutaway, textureBytes:mats.bytes+PK.bytes+512*256*4*4/3 };
  }
  window.ExteriorInterior={create,validateWindows};
})();
