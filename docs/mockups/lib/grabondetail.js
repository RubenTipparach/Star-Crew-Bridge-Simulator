/* Detailed Grabon review extension. Reuses ShipConcepts navigation and room
   construction, ShipKit access fixtures, and Blender-built props and hulls. */
(function(){
 'use strict';
 async function prepare(T,K,GLTFLoader){
  const base=await K.loadMaterials(T),mats=await K.loadPanels(T,base,{px:256});
  const props=await PropKit.create(T,K,GLTFLoader,mats,{sets:['bridge','machinery']});
  const pack=JSON.parse(document.getElementById('ship-models-grabon').textContent),hulls={};
  for(const [id,uri] of Object.entries(pack.models)){
   const buf=Uint8Array.from(atob(uri.split(',')[1]),c=>c.charCodeAt(0)).buffer;
   hulls[id]=(await new Promise((res,rej)=>new GLTFLoader().parse(buf,'',res,rej))).scene;
  }
  let activeHull=null,liftCar=null,liftGoal=0,lastTime=0,api=null;
  const textureBytes=Math.max(...Object.values(pack.manifest.props).map(p=>p.texture_bytes_with_mips));
  function geometry(B,ctx,lamps=[]){
   const g=K.geometryOf(T,B.parts,mats,K.finishOf({finish:'working'},K.shipData('detailing')));
   K.bakeDirect(T,g,lamps,ctx.state.lighting,{gain:.95});
   const mesh=new T.Mesh(g,K.surfaceMaterial(T,mats,{lit:false}));ctx.model.add(mesh);return mesh;
  }
  function furnish(ctx,r,parts){
   const table={console:'free_console',seat:'crew_chair',bunk:'bunk',bed:'med_bed',table:'mess_table'};
   const machines={reactor:'shield_generator',drive:'impulse_drive',magazine:'magazine_rack',life_support:'ls_tanks',stbd_laser:'missile_tube',port_laser:'missile_tube',tubes:'missile_tube',computers:'switchboard'};
   for(const f of ctx.state.ship.furnishings.filter(f=>f.room===r.id&&f.kind!=='person')){
    const kind=table[f.kind]||machines[r.id]||'battery_bank',P={};props.placeProp(P,kind,[0,0,0],f.kind==='bunk'?90:0,'engineering',{wall:false,accent:new T.Color('#d07639')});
    const all=Object.values(P).flatMap(p=>p.position),lo=[Infinity,Infinity,Infinity],hi=lo.map(()=>-Infinity);
    for(let i=0;i<all.length;i++) {const k=i%3;lo[k]=Math.min(lo[k],all[i]);hi[k]=Math.max(hi[k],all[i]);}
    const scale=Math.min(...f.size_m.map((q,k)=>q/(hi[k]-lo[k])));
    for(const p of Object.values(P))for(let i=0;i<p.position.length;i+=3){p.position[i]=f.center_m[0]+(p.position[i]-(lo[0]+hi[0])/2)*scale;p.position[i+1]=f.center_m[1]+.02+(p.position[i+1]-lo[1])*scale;p.position[i+2]=f.center_m[2]+(p.position[i+2]-(lo[2]+hi[2])/2)*scale;}
    for(const [role,part] of Object.entries(P)){if(role==='screen'&&!part.glow)part.glow=Array(part.position.length/3).fill(1);parts['detail_'+Object.keys(parts).length+'_'+role]=part;}
   }
   for(const f of ctx.state.ship.craft.filter(f=>f.room===r.id)){const P={};props.placeProp(P,'swift_fighter',[f.center_m[0],f.center_m[1]+.03,f.center_m[2]],f.yaw_deg,'tactical',{wall:false});for(const [role,part] of Object.entries(P)){if(role==='screen'&&!part.glow)part.glow=Array(part.position.length/3).fill(1);parts['detail_'+Object.keys(parts).length+'_'+role]=part;}}
  }
  function access(ctx,whole=false){
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
     B.flat(T,'floor',[[-1.2,-6.3],[1.2,-6.3],[1.2,2],[-1.2,2]],holes,y,1);
     for(const z of [-6.3,2])B.pipe('railing',[-1.18,y+1.0,z],[1.18,y+1.0,z],.026,6);
    }
    const topY=fl.at(-1)+1;
    for(const x of [x0+.08,x1-.08])B.pipe('ladder',[x,.08,z0+.08],[x,topY,z0+.08],.025,6);
    for(let y=.2;y<topY;y+=.3)B.pipe('ladder',[x0+.11,y,z0+.08],[x1-.11,y,z0+.08],.023,6);
   }
   // These open landings have world UVs, so use the tiling deck material.
   // Fitted room floors use the panel builder's bounded atlas coordinates.
   for(const role of ['floor','platform'])if(B.parts[role])B.parts[role].material='deck_plate';
   B.parts.screen&&(B.parts.screen.material='light_panel');
   geometry(B,ctx,lamps);
  }
  function build(ctx){
   liftCar=null;
   if(ctx.state.stage==='hull'){
    const id=ctx.state.ship.id;
    if(activeHull&&activeHull!==id)hulls[activeHull].traverse(o=>{if(o.isMesh){o.material.map?.dispose();}});
    activeHull=id;
    const source=hulls[id];source.traverse(o=>{if(o.isMesh&&o.material.map)o.material.map.needsUpdate=true;});
    const clone=source.clone(true);clone.traverse(o=>{if(o.isMesh){o.geometry=o.geometry.clone();o.material.side=T.FrontSide;}});ctx.model.add(clone);return true;
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
   if(state.stage==='fit'){
    const b=document.createElement('button');b.textContent='Deck access';b.dataset.mode='access';b.setAttribute('aria-pressed',state.mode==='access');b.onclick=()=>show(state.ship.id,'fit',state.deck,'access');view.prepend(b);
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
    const b=document.createElement('button');b.textContent='Detail';b.dataset.detail='hull';b.onclick=()=>{const h=state.ship.hull_zones.find(z=>z.id==='head'),z=h.poly.reduce((v,p)=>v+p[1]/h.poly.length,0),y=(h.y_m[0]+h.y_m[1])/2;controls.target.set(0,y,z);camera.position.set(24,y+20,z+29);controls.update();};view.append(b);
    for(const mode of ['Rear','Armor']){
     const b=document.createElement('button');b.textContent=mode;b.dataset.detail=mode.toLowerCase();b.onclick=()=>{
      const h=state.ship.hull_zones.find(z=>z.id==='body'),aft=Math.min(...h.poly.map(p=>p[1]));
      controls.target.set(0,4,aft+9);camera.position.set(...(mode==='Rear'?[0,8,aft-85]:[31,29,aft-29]));controls.update();
     };view.append(b);
    }
   }
  }
  function frame(){const now=performance.now(),dt=Math.min(.05,(now-lastTime)/1000);lastTime=now;if(!liftCar)return;const goal=liftGoal-liftCar.userData.floorOrigin,delta=goal-liftCar.position.y;liftCar.position.y+=Math.sign(delta)*Math.min(Math.abs(delta),dt*1.5);}
  const cameraDirection=(state,view)=>state.mode==='access'&&state.stage==='fit'&&view==='orbit'?[-.93,.55,.9]:null;
  return {mats,textureBytes,build,furnish,access,decorate,frame,cameraDirection,get lift(){return {car:liftCar,goal:liftGoal};}};
 }
 function registerShots(){
  const v=SHIP_VIEW;
  for(const s of v.data.ships){
   MOCKUP_SHOTS.push({name:'access-'+s.id,setup:()=>v.show(s.id,'fit',s.layout.decks.at(-1).id,'access')});
   MOCKUP_SHOTS.push({name:'close-'+s.id,setup:()=>{v.show(s.id,'hull');document.querySelector('[data-detail="hull"]').click();}});
   for(const mode of ['rear','armor'])MOCKUP_SHOTS.push({name:mode+'-'+s.id,setup:()=>{v.show(s.id,'hull');document.querySelector('[data-detail="'+mode+'"]').click();}});
   MOCKUP_SHOTS.push({name:'bridge-'+s.id,setup:()=>{v.show(s.id,'fit','A','cutaway');document.querySelectorAll('#viewtools button').forEach(b=>{if(b.textContent==='Room')b.click();});}});
  }
 }
 window.GrabonDetail={prepare,registerShots};
})();
