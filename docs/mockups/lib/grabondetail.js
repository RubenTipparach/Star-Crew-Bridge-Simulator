/* Detailed Grabon review extension. Reuses ShipConcepts navigation and room
   construction, ShipKit access fixtures, and Blender-built props and hulls. */
(function(){
 'use strict';
 async function prepare(T,K,GLTFLoader){
  const cfg=K.shipData('interior');
  const base=await K.loadMaterials(T),mats=await K.loadPanels(T,base,{px:256,atlasSets:['grabon_furniture','machinery']});
  base.texture.dispose();
  const props=await PropKit.create(T,K,GLTFLoader,mats,{sets:['grabon_furniture','machinery']});
  const pack=JSON.parse(document.getElementById('ship-models-grabon').textContent),hulls={};
  for(const [id,uri] of Object.entries(pack.models)){
   const buf=Uint8Array.from(atob(uri.split(',')[1]),c=>c.charCodeAt(0)).buffer;
   hulls[id]=(await new Promise((res,rej)=>new GLTFLoader().parse(buf,'',res,rej))).scene;
  }
  let activeHull=null,liftCar=null,liftGoal=0,lastTime=0,api=null,insideRoom='bridge',exterior=null,free=null;
  const merge=(a,b)=>{for(const [k,v] of Object.entries(b))if(v&&typeof v==='object'&&!Array.isArray(v))merge(a[k]||(a[k]={}),v);else a[k]=structuredClone(v);return a;};
  const detailing=()=>merge(structuredClone(K.shipData('detailing')),cfg.detailing);
  function dressRoom(ctx,r,built,parts){K.subdivideParts(parts,cfg.lighting.subdivision_m,['floor','wall','ceiling','cove']);}
  function lightRoom(ctx,r,built,geo){
   const tint=new T.Color(...cfg.lighting.lamp_tint_srgb).convertSRGBToLinear();
   const lamps=built.lamps.map(l=>({...l,color:[tint.r,tint.g,tint.b]}));
   K.bakeDirect(T,geo,lamps,ctx.state.lighting,{gain:cfg.lighting.gain});
   if(ctx.state.lighting==='normal'){
    K.paintRole(geo,'lamp',tint.clone().multiplyScalar(1.4));
    K.paintRole(geo,'status',new T.Color(...cfg.furniture.screen_srgb).convertSRGBToLinear().multiplyScalar(1.4));
   }
   if(mats._roomsMaterial?.userData.panelGlow)mats._roomsMaterial.userData.panelGlow.value=K.panelsData().manifest.glow[ctx.state.lighting];
  }
  const textureBytes=Math.max(...Object.values(pack.manifest.props).map(p=>p.texture_bytes_with_mips));
  function geometry(B,ctx,lamps=[]){
   const g=K.geometryOf(T,B.parts,mats,K.finishOf({finish:'working'},K.shipData('detailing')));
   K.bakeDirect(T,g,lamps,ctx.state.lighting,{gain:.95});
   const mesh=new T.Mesh(g,K.surfaceMaterial(T,mats,{lit:false}));ctx.model.add(mesh);return mesh;
  }
  function furnish(ctx,r,parts){
   const table={console:'grabon_console',seat:'grabon_chair',bunk:'grabon_bunk',bed:'med_bed',table:'grabon_table'};
   const machines={reactor:'shield_generator',drive:'impulse_drive',magazine:'magazine_rack',life_support:'ls_tanks',stbd_laser:'missile_tube',port_laser:'missile_tube',tubes:'missile_tube',computers:'switchboard'};
   for(const f of ctx.state.ship.furnishings.filter(f=>f.room===r.id&&f.kind!=='person')){
    const kind=table[f.kind]||machines[r.id]||'battery_bank',P={};props.placeProp(P,kind,[0,0,0],0,'engineering',{wall:false,accent:new T.Color('#d07639')});
    const all=Object.values(P).flatMap(p=>p.position),lo=[Infinity,Infinity,Infinity],hi=lo.map(()=>-Infinity);
    for(let i=0;i<all.length;i++) {const k=i%3;lo[k]=Math.min(lo[k],all[i]);hi[k]=Math.max(hi[k],all[i]);}
    const scale=Math.min(...f.size_m.map((q,k)=>q/(hi[k]-lo[k])));
    for(const p of Object.values(P))for(let i=0;i<p.position.length;i+=3){p.position[i]=f.center_m[0]+(p.position[i]-(lo[0]+hi[0])/2)*scale;p.position[i+1]=f.center_m[1]+.02+(p.position[i+1]-lo[1])*scale;p.position[i+2]=f.center_m[2]+(p.position[i+2]-(lo[2]+hi[2])/2)*scale;}
    for(const [role,part] of Object.entries(P)){if(role==='screen'&&!part.glow)part.glow=Array(part.position.length/3).fill(1);parts['detail_'+Object.keys(parts).length+'_'+role]=part;}
   }
   for(const f of ctx.state.ship.craft.filter(f=>f.room===r.id)){const P={};props.placeProp(P,'swift_fighter',[f.center_m[0],f.center_m[1]+.03,f.center_m[2]],f.yaw_deg,'tactical',{wall:false});for(const [role,part] of Object.entries(P)){if(role==='screen'&&!part.glow)part.glow=Array(part.position.length/3).fill(1);parts['detail_'+Object.keys(parts).length+'_'+role]=part;}}
  }
  function access(ctx,whole=false,roomsPresent=false){
   const s=ctx.state.ship,A=s.access,L=ctx.layout,deck=L.decks.find(d=>d.id===ctx.state.deck),floor=deck.floor_y_m;
   const B=new K.Builder(),X=[1,0,0],Y=[0,1,0],Z=[0,0,1],faces=['+u','-u','+v','-v','+w','-w'];
   const stair=structuredClone(A.stair),fl=stair.floors_y_m;
   if(whole||floor===0){const [x0,z0,x1,z1]=A.shaft_rects.stairs;B.flat(T,'floor',[[x0,z0],[x1,z0],[x1,z1],[x0,z1]],[],0,1);}
   if(!whole){const k=Math.min(fl.indexOf(floor),fl.length-2);stair.floors_y_m=fl.slice(k,k+2);}
   K.buildSpiralStair(T,B,stair);
   const pos=(a,y,r=1.66)=>[stair.center_m[0]+Math.sin(a*Math.PI/180)*r,y,stair.center_m[2]+Math.cos(a*Math.PI/180)*r];
   for(let k=0;k<stair.floors_y_m.length-1;k++){
    const y0=stair.floors_y_m[k];
    for(let i=0;i<19;i++){
     const a=90+270*i/19,b=90+270*(i+1)/19,y=y0+(i+1)*.18;
     B.pipe('railing',pos(a,y+1.04),pos(b,y+.18+1.04),.026,6);
     if(i%3===0)B.pipe('railing',pos(a,y+.05),pos(a,y+1.01),.022,6);
    }
   }
   // Landing guards face the open stair well and stop before its stair arrival.
   for(const y of fl.filter(y=>y>0&&(whole||Math.abs(y-floor)<3.61))){
    B.pipe('railing',[-1.85,y+1.05,-4.2],[-1.35,y+1.05,-4.2],.028,6);
    for(const x of [-1.82,-1.4])B.pipe('railing',[x,y+.04,-4.2],[x,y+1.02,-4.2],.025,6);
   }
   // The same car and shaft coordinates are used in deck and access views.
   const car=structuredClone(A.lift);car.center_m[1]=whole?liftGoal:floor;
   const shaft=L.compartments.find(r=>r.id==='stairs'),built=K.buildCompartment(T,L,shaft,{detail:false});
   const lamps=built.lamps.slice();
   // Visible landing fixtures also light the exposed corridor and ladder.
   for(const y of fl.filter(y=>whole||y===floor)){
    const p=[.95,y+2.45,-2.0];lamps.push({p,R:5,I:1.3,emergency:true});
    B.box('lamp_housing',p,X,Y,Z,.13,.06,.45,faces);
    B.box('screen',[p[0],p[1]-.07,p[2]],X,Y,Z,.10,.012,.40,faces);
   }
   const CB=new K.Builder();K.buildLiftCar(CB,car);
   const carLamp=[3.95,car.center_m[1]+2.02,-.4];
   CB.box('screen',carLamp,X,Y,Z,.018,.08,.6,faces);CB.parts.screen.material='light_panel';
   liftCar=geometry(CB,ctx,[...lamps,{p:carLamp,R:3,I:1.4,emergency:true}]);liftCar.name='Elevator car';liftCar.userData.floorOrigin=car.center_m[1];liftGoal=car.center_m[1];
   const bottom=whole?0:floor,top=whole?fl.at(-1)+2.8:floor+2.8;
   for(const z of [-1.55,.75])B.pipe('railing',[4.12,bottom+.04,z],[4.12,top-.04,z],.055,6);
   for(const y of fl.filter(y=>whole||y===floor)){
    B.box('platform',[1.35,y-.07,-.4],X,Y,Z,.15,.07,.66,faces);
    B.box('screen',[1.185,y+1.35,.47],X,Y,Z,.025,.14,.10,faces);
   }
   if(whole){
    // Open corridor landings expose the real ladder cuts and shaft entries.
    const portal=L.portals.find(p=>p.kind==='ladder'),[lx,,lz]=portal.center_m,[lw,ld]=portal.size_m;
    const x0=lx-lw/2,x1=lx+lw/2,z0=lz-ld/2,z1=lz+ld/2;
    for(const d of L.decks){
     const y=d.floor_y_m,holes=y?[[[x0,z0],[x1,z0],[x1,z1],[x0,z1]]]:[];
     if(!roomsPresent){
      B.flat(T,'floor',[[-1.2,-6.3],[1.2,-6.3],[1.2,2],[-1.2,2]],holes,y,1);
      for(const z of [-6.3,2])B.pipe('railing',[-1.18,y+1.0,z],[1.18,y+1.0,z],.026,6);
     }
    }
    const topY=fl.at(-1)+1;
    if(!roomsPresent){
     for(const x of [x0+.08,x1-.08])B.pipe('ladder',[x,.08,z0+.08],[x,topY,z0+.08],.025,6);
     for(let y=.2;y<topY;y+=.3)B.pipe('ladder',[x0+.11,y,z0+.08],[x1-.11,y,z0+.08],.023,6);
    }
   }
   // These open landings have world UVs, so use the tiling deck material.
   // Fitted room floors use the panel builder's bounded atlas coordinates.
   for(const role of ['floor','platform'])if(B.parts[role])B.parts[role].material='deck_plate';
   B.parts.screen&&(B.parts.screen.material='light_panel');
   geometry(B,ctx,lamps);
  }
  function build(ctx){
   liftCar=null;exterior=null;
   if(ctx.state.stage==='hull'){
    const id=ctx.state.ship.id;
    if(activeHull&&activeHull!==id)hulls[activeHull].traverse(o=>{if(o.isMesh){o.material.map?.dispose();}});
    activeHull=id;
    const source=hulls[id];source.traverse(o=>{if(o.isMesh&&o.material.map)o.material.map.needsUpdate=true;});
    const clone=source.clone(true);clone.name='Grabon exterior';clone.traverse(o=>{if(o.isMesh){o.geometry=o.geometry.clone();o.material.side=T.FrontSide;o.userData.exterior=true;}});ctx.model.add(clone);exterior=clone;
    const windows=pack.manifest.props[id].exterior_refit.windows;
    for(const r of ctx.layout.compartments)ShipConcepts.buildRoom(ctx,r);
    liftGoal=ctx.layout.decks.at(-1).floor_y_m;
    access(ctx,true,true);
    ctx.model.add(ExteriorInterior.windowReveals(T,windows,{color:0x59464c,glass:0xe6b174,opacity:.035}));
    return true;
   }
   if(ctx.state.stage==='fit'&&ctx.state.mode==='interior'){
    const rooms=ctx.layout.compartments.filter(r=>r.decks.includes(ctx.state.deck)&&r.kind!=='trunk');
    const room=rooms.find(r=>r.id===insideRoom)||rooms.find(r=>r.id==='bridge')||rooms.find(r=>r.id==='mess')||rooms.find(r=>r.group!=='route')||rooms[0];
    insideRoom=room.id;ShipConcepts.buildRoom(ctx,room);return true;
   }
   if(ctx.state.stage==='fit'&&ctx.state.mode==='access'){
    liftGoal=ctx.layout.decks.find(d=>d.id===ctx.state.deck).floor_y_m;
    access(ctx,true);return true;
   }
   return false;
  }
  function decorate(a){
   api=a;const {state,show,update,setCamera,camera,controls}=a,view=document.getElementById('viewtools');
   const dims=pack.manifest.props[state.ship.id].dimensions_m;
   if(dims){const stats=document.querySelectorAll('#stats b');stats[0].textContent=dims[2].toFixed(1)+' m';stats[1].textContent=dims[0].toFixed(1)+' m';}
   document.getElementById('viewlabel').querySelector('small').textContent=state.stage==='hull'?'Painted armor / service fittings / detailed engines':state.mode==='access'?'Stairs / ladder / elevator':'Textured interiors / connected deck access';
   if(state.mode==='access'&&state.stage==='fit')document.getElementById('viewlabel').querySelector('b').textContent='03 / DECK ACCESS';
   controls.minDistance=state.mode==='interior'?.1:1;
   if(state.stage==='fit'){
    const b=document.createElement('button');b.textContent='Deck access';b.dataset.mode='access';b.setAttribute('aria-pressed',state.mode==='access');b.onclick=()=>show(state.ship.id,'fit',state.deck,'access');view.prepend(b);
    const inside=document.createElement('button');inside.textContent='Inside';inside.dataset.mode='interior';inside.setAttribute('aria-pressed',state.mode==='interior');inside.onclick=()=>{insideRoom=state.selected||insideRoom;show(state.ship.id,'fit',state.deck,'interior');eyeCamera();};view.append(inside);
    if(state.mode==='interior'){
     a.roomInfo(insideRoom);view.querySelector('[data-camera="orbit"]').textContent='Eye level';
     const select=document.createElement('select');select.id='room-select';select.setAttribute('aria-label','Interior room');
     for(const r of state.ship.layout.compartments.filter(r=>r.decks.includes(state.deck)&&r.kind!=='trunk')){const o=document.createElement('option');o.value=r.id;o.textContent=r.name;o.selected=r.id===insideRoom;select.append(o);}
     select.onchange=()=>{insideRoom=select.value;update();eyeCamera();};view.append(select);
     document.getElementById('viewlabel').querySelector('b').textContent='03 / '+state.ship.layout.compartments.find(r=>r.id===insideRoom).name.toUpperCase();
     document.getElementById('hint').innerHTML='Drag to look / WASD or arrows to walk<br>Scroll to step / choose a room above';
    }
   }
   let lift=document.getElementById('lift-controls');if(lift)lift.remove();
   if(state.mode==='access'&&state.stage==='fit'){
    lift=document.createElement('div');lift.id='lift-controls';lift.style.cssText='position:absolute;left:24px;bottom:82px;display:flex;gap:6px;align-items:center;z-index:4';
    lift.innerHTML='<span class="eyebrow">CALL LIFT</span>';
    for(const d of state.ship.layout.decks){const b=document.createElement('button');b.textContent=d.id;b.dataset.lift=d.id;b.onclick=()=>{liftGoal=d.floor_y_m;};lift.append(b);}
    document.getElementById('view').append(lift);
   }
   if(state.stage==='fit'&&state.mode==='cutaway'){
    const b=document.createElement('button');b.textContent='Room';b.onclick=()=>{
     const r=state.ship.layout.compartments.find(r=>r.id===state.selected)||state.ship.layout.compartments.find(r=>r.id==='bridge'&&r.decks.includes(state.deck))||state.ship.layout.compartments.find(r=>r.id==='reactor'&&r.decks.includes(state.deck))||state.ship.layout.compartments.find(r=>r.decks.includes(state.deck)&&r.kind!=='corridor');
     const box=new T.Box3();for(const br of r.brushes)for(const [x,z] of br.poly){box.expandByPoint(new T.Vector3(x,br.y[0],z));box.expandByPoint(new T.Vector3(x,br.y[1],z));}
     box.getCenter(controls.target);const size=box.getSize(new T.Vector3()),dist=Math.max(size.x,size.z)*1.25;camera.position.copy(controls.target).add(new T.Vector3(dist*.7,dist*.9,dist));controls.update();
    };view.append(b);
   }
   if(state.stage==='hull'){
    document.getElementById('normal').disabled=document.getElementById('alert').disabled=false;
    const peek=document.createElement('button'),picker=document.createElement('select');
    peek.textContent='Windows';peek.dataset.detail='windows';picker.id='window-select';picker.hidden=true;picker.setAttribute('aria-label','Exterior window');
    for(const [i,w] of pack.manifest.props[state.ship.id].exterior_refit.windows.entries()){
     const r=state.ship.layout.compartments.find(r=>r.id===w.room),o=document.createElement('option');o.value=w.id;o.textContent=w.deck+' / '+r.name+' / '+(i+1);picker.append(o);
    }
    peek.onclick=()=>{picker.hidden=false;windowCamera(picker.value);};picker.onchange=()=>windowCamera(picker.value);view.append(peek,picker);
    const b=document.createElement('button');b.textContent='Detail';b.dataset.detail='hull';b.onclick=()=>{const h=state.ship.hull_zones.find(z=>z.id==='head'),z=h.poly.reduce((v,p)=>v+p[1]/h.poly.length,0),y=(h.y_m[0]+h.y_m[1])/2;controls.target.set(0,y,z);camera.position.set(24,y+20,z+29);controls.update();};view.append(b);
    for(const mode of ['Rear','Armor']){
     const b=document.createElement('button');b.textContent=mode;b.dataset.detail=mode.toLowerCase();b.onclick=()=>{
      const h=state.ship.hull_zones.find(z=>z.id==='body'),aft=Math.min(...h.poly.map(p=>p[1]));
      controls.target.set(0,4,aft+9);camera.position.set(...(mode==='Rear'?[0,8,aft-85]:[31,29,aft-29]));controls.update();
     };view.append(b);
    }
   }
  }
  function eyeCamera(){
   const {state,camera,controls}=api,r=state.ship.layout.compartments.find(r=>r.id===insideRoom),b=r.brushes[0],xs=b.poly.map(p=>p[0]),zs=b.poly.map(p=>p[1]);
   const x=(Math.min(...xs)+Math.max(...xs))/2,z=(Math.min(...zs)+Math.max(...zs))/2,y=b.y[0]+1.65;
   let spawn=[x,z-Math.min(2,(Math.max(...zs)-Math.min(...zs))*.2)];
   if(!walkable(...spawn)){
    const candidates=[];for(let a=Math.min(...xs)+.3;a<Math.max(...xs);a+=.4)for(let b=Math.min(...zs)+.3;b<Math.max(...zs);b+=.4)if(walkable(a,b))candidates.push([a,b]);
    candidates.sort((a,b)=>Math.hypot(a[0]-spawn[0],a[1]-spawn[1])-Math.hypot(b[0]-spawn[0],b[1]-spawn[1]));
    if(!candidates.length)throw new Error(r.id+': no clear standing view');spawn=candidates[0];
   }
   let look=[x+.5,Math.max(...zs)-.4];
   if(r.id!=='bridge'){
    // Face an open aisle rather than the back of the nearest bunk or cabinet.
    let best=-1;
    for(let i=0;i<16;i++){
     const angle=i*Math.PI/8,dx=Math.sin(angle),dz=Math.cos(angle);let reach=.2;
     while(reach<8&&walkable(spawn[0]+dx*reach,spawn[1]+dz*reach))reach+=.2;
     if(reach>best){best=reach;look=[spawn[0]+dx*4,spawn[1]+dz*4];}
    }
   }
   camera.position.set(spawn[0],y,spawn[1]);controls.target.set(look[0],y-.1,look[1]);controls.update();
   free?.setLook({walk:true});
  }
  function windowCamera(id){
   const {state,camera,controls}=api,windows=pack.manifest.props[state.ship.id].exterior_refit.windows;
   const w=windows.find(w=>w.id===id)||windows.find(w=>w.id==='window_bridge_bow_-1')||windows[0],c=new T.Vector3(...w.center_m),n=new T.Vector3(...w.normal);
   camera.position.copy(c).addScaledVector(n,w.depth_m+2.1);controls.target.copy(c).addScaledVector(n,-2);controls.update();
   document.getElementById('viewlabel').querySelector('small').textContent=w.deck+' / '+state.ship.layout.compartments.find(r=>r.id===w.room).name+' / hull window';
   api.roomInfo(w.room);
  }
  function frame(){const now=performance.now(),dt=Math.min(.05,(now-lastTime)/1000);lastTime=now;if(!liftCar)return;const goal=liftGoal-liftCar.userData.floorOrigin,delta=goal-liftCar.position.y;liftCar.position.y+=Math.sign(delta)*Math.min(Math.abs(delta),dt*1.5);}
  const cameraDirection=(state,view)=>state.mode==='access'&&state.stage==='fit'&&view==='orbit'?[-.93,.55,.9]:null;
  function cameraPoints(state){
   if(state.stage!=='hull'||!exterior)return null;
   const points=[];exterior.updateMatrixWorld(true);
   exterior.traverse(o=>{if(o.isMesh){const p=o.geometry.attributes.position;for(let i=0;i<p.count;i++)points.push(new T.Vector3().fromBufferAttribute(p,i).applyMatrix4(o.matrixWorld));}});
   return points;
  }
  const setCamera=(state,view)=>{
   const inside=state.stage==='fit'&&state.mode==='interior';
   api.camera.fov=inside?65:36;api.camera.updateProjectionMatrix();
   if(inside&&view==='orbit'){eyeCamera();return true;}return false;
  };
  const floorAt=()=>api.state.ship.layout.compartments.find(r=>r.id===insideRoom).brushes[0].y[0];
  const walkable=(x,z)=>{
   const s=api.state.ship,r=s.layout.compartments.find(r=>r.id===insideRoom);
   if(!r.brushes.some(b=>K.insidePoly(b.poly,x,z,-.24)))return false;
   return ![...s.furnishings,...s.craft].filter(f=>f.room===r.id&&f.kind!=='person').some(f=>Math.abs(x-f.center_m[0])<f.size_m[0]/2+.20&&Math.abs(z-f.center_m[2])<f.size_m[2]/2+.20);
  };
  const validateWindows=()=>ExteriorInterior.validateWindows(T,exterior,{L:api.state.ship.layout,windows:pack.manifest.props[api.state.ship.id].exterior_refit.windows},api.state.ship.layout.decks.map(d=>d.id),cfg.window_inspection_distance_m);
  return {mats,textureBytes,build,furnish,access,decorate,frame,cameraDirection,cameraPoints,setCamera,detailing,dressRoom,lightRoom,eyeCamera,windowCamera,validateWindows,floorAt,walkable,bindCamera:f=>{free=f;},get exterior(){return exterior;},get windows(){return pack.manifest.props[api.state.ship.id].exterior_refit.windows;},get lift(){return {car:liftCar,goal:liftGoal};}};
 }
 function registerShots(){
  const v=SHIP_VIEW;
  for(const s of v.data.ships){
   MOCKUP_SHOTS.push({name:'windows-'+s.id,setup:()=>{v.show(s.id,'hull');document.querySelector('[data-detail="windows"]').click();}});
   for(const light of ['normal','red_alert'])MOCKUP_SHOTS.push({name:'inside-'+s.id+'-'+light,setup:()=>{v.show(s.id,'fit','A','interior',light);GRABON_DETAIL.eyeCamera();}});
   for(const room of ['mess','bunks','reactor'])MOCKUP_SHOTS.push({name:room+'-'+s.id,setup:()=>{
    const r=s.layout.compartments.find(r=>r.id===room);v.show(s.id,'fit',r.decks[0],'interior');
    const select=document.getElementById('room-select');select.value=room;select.dispatchEvent(new Event('change'));
   }});
   MOCKUP_SHOTS.push({name:'access-'+s.id,setup:()=>v.show(s.id,'fit',s.layout.decks.at(-1).id,'access')});
   MOCKUP_SHOTS.push({name:'close-'+s.id,setup:()=>{v.show(s.id,'hull');document.querySelector('[data-detail="hull"]').click();}});
   for(const mode of ['rear','armor'])MOCKUP_SHOTS.push({name:mode+'-'+s.id,setup:()=>{v.show(s.id,'hull');document.querySelector('[data-detail="'+mode+'"]').click();}});
   MOCKUP_SHOTS.push({name:'bridge-'+s.id,setup:()=>{v.show(s.id,'fit','A','cutaway');document.querySelectorAll('#viewtools button').forEach(b=>{if(b.textContent==='Room')b.click();});}});
  }
 }
 window.GrabonDetail={prepare,registerShots};
})();
