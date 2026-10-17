/* Faction ship studies from one catalogue. ShipKit owns room shells and portals;
   this module owns concept-stage selection, plan diagrams and rough hull lofts. */
(function () {
  'use strict';
  const $ = id => document.getElementById(id);
  const area = p => Math.abs(p.reduce((a,v,i) => a+v[0]*p[(i+1)%p.length][1]-p[(i+1)%p.length][0]*v[1],0)/2);
  const bounds = p => [Math.min(...p.map(v=>v[0])),Math.min(...p.map(v=>v[1])),Math.max(...p.map(v=>v[0])),Math.max(...p.map(v=>v[1]))];
  const center = p => {const b=bounds(p);return [(b[0]+b[2])/2,(b[1]+b[3])/2];};
  const svgPoints = p => p.map(v=>v[1]+','+(-v[0])).join(' ');
  const text = s => String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('"','&quot;');

  async function start(THREE, OrbitControls, detail=null) {
    const K=window.ShipKit, data=K.shipData('layouts');
    const mats=detail?.mats||await K.loadMaterials(THREE);
    // Rooms bake for real (lib/roombake.js) when the page carries the baker; its cache is the page's bake-cache block.
    const cacheEl=document.querySelector('script[id^="bake-cache-"]');
    if(window.RoomBake&&window.LightBake&&cacheEl)window.ShipConcepts.bakePage=RoomBake.page({name:cacheEl.id.replace(/^bake-cache-/,'').replace(/-bake$/,''),THREE,K,LBK:window.LightBake,pmats:mats});
    const hud=K.budgetHud({textureBytes:mats.bytes+(detail?.textureBytes||0),note:'Design review; no game or Pi timing claim.'});
    const renderer=new THREE.WebGLRenderer({antialias:true,alpha:true});
    renderer.setPixelRatio(Math.min(devicePixelRatio,2));
    $('canvas').appendChild(renderer.domElement);
    const scene=new THREE.Scene();
    const camera=new THREE.PerspectiveCamera(36,1,.1,1000);
    const free=detail&&window.FreeCam?FreeCam.create(THREE,camera,renderer.domElement,OrbitControls,{walkable:(x,z)=>detail.walkable(x,z),floorAt:()=>detail.floorAt()}):null;
    const controls=free?.orbit||new OrbitControls(camera,renderer.domElement);
    detail?.bindCamera?.(free);
    controls.enableDamping=true;controls.minDistance=5;controls.maxDistance=160;
    const state={ship:data.ships[0],stage:'plan',deck:'B',mode:'plan',lighting:'normal',compare:false,selected:null};
    let model=new THREE.Group();scene.add(model);
    window.MOCKUP_SCENE=scene;
    window.MOCKUP_BUDGET=()=>hud.read();
    function layout(){
      const L=structuredClone(state.ship.layout);
      if(state.stage==='plan') for(const r of L.compartments)r.brushes=structuredClone(r.initial_brushes);
      return L;
    }
    function roomInfo(id) {
      const r=layout().compartments.find(r=>r.id===id);state.selected=r?.id||null;
      if(!r){$('room-info').innerHTML='<h2>Room study</h2><p>Select a room to see its purpose, floor area and clear height.</p>';return;}
      const a=r.brushes.reduce((v,b)=>v+area(b.poly),0),h=r.brushes[0].y[1]-r.brushes[0].y[0];
      $('room-info').innerHTML=`<div class="eyebrow">ROOM ${String(r.poi).padStart(2,'0')} / DECK ${r.decks[0]}</div><h2>${text(r.name)}</h2><p>${text(r.purpose)}</p><p><strong>${a.toFixed(1)} m²</strong> floor / <strong>${h.toFixed(1)} m</strong> clear<br><strong>${(a*h).toFixed(1)} m³</strong> air volume</p>`;
      if(innerWidth<=760&&$('viewlabel').querySelector('small'))$('viewlabel').querySelector('small').textContent=`${r.name} / ${a.toFixed(1)} m² / ${h.toFixed(1)} m clear`;
      document.querySelectorAll('.room-shape').forEach(e=>e.classList.toggle('selected',e.dataset.room===id));
    }
    function drawPlan(){
      const L=layout(),rooms=L.compartments.filter(r=>r.decks.includes(state.deck));
      const pts=rooms.flatMap(r=>r.brushes.flatMap(b=>b.poly));
      const B=bounds(pts),w=B[3]-B[1]+9,h=B[2]-B[0]+12;
      const fs=Math.min(1.05,Math.max(innerWidth<=760?.9:.7,w/55));
      const vb=[B[1]-4.5,-B[2]-6,w,h],phone=innerWidth<=760;
      const viewBox=phone?[vb[1],-vb[0]-vb[2],vb[3],vb[2]]:vb;
      let svg=`<svg viewBox="${viewBox.join(' ')}" role="img" aria-label="${state.ship.name} deck ${state.deck}, ${state.stage==='plan'?'initial':'fitted'} floor plan"><g${phone?' transform="rotate(-90)"':''}>`;
      if(state.stage==='fit')for(const zone of state.ship.hull_zones.filter(z=>z.deck===state.deck))svg+=`<polygon points="${svgPoints(zone.poly)}" fill="#31434b" stroke="#8e9ca0" stroke-width=".1"/>`;
      for(const r of rooms){
        const col=data.palette[r.group];
        for(const b of r.brushes)svg+=`<polygon class="room-shape ${r.id===state.selected?'selected':''}" data-room="${r.id}" points="${svgPoints(b.poly)}" fill="${col}" fill-opacity=".22" stroke="${col}" stroke-opacity=".8" stroke-width=".08"><title>${text(r.name)}: ${area(b.poly).toFixed(1)} m²</title></polygon>`;
      }
      if(state.compare && state.stage==='fit')for(const r of rooms)for(const b of r.initial_brushes)svg+=`<polygon points="${svgPoints(b.poly)}" class="original"/>`;
      for(const f of state.ship.craft){
        if(!rooms.some(r=>r.id===f.room))continue;
        const [x,,z]=f.center_m, s=f.yaw_deg===180?-1:1;
        svg+=`<polygon points="${svgPoints([[x,z+4*s],[x+4,z-3.5*s],[x,z-2*s],[x-4,z-3.5*s]])}" fill="#728084" fill-opacity=".3" stroke="#a3b3b6" stroke-width=".08"/><text x="${z-1*s}" y="${-x+.3}" text-anchor="middle" fill="#b8c3c5" font-size=".65">JACKAL</text>`;
      }
      for(const p of L.portals){
        if(!rooms.some(r=>p.between.includes(r.id)))continue;
        const [x,,z]=p.center_m;
        if(Math.abs(p.normal[1])>.5){svg+=`<rect x="${z-.7}" y="${-x-.7}" width="1.4" height="1.4" fill="#17262e" stroke="#ddb178" stroke-width=".1"/><path d="M${z-.4} ${-x-.45}h.8m-.8 .3h.8m-.8 .3h.8m-.8 .3h.8" class="portal"/>`;continue;}
        const t=[p.normal[2],-p.normal[0]],d=p.size_m[0]/2;
        svg+=`<line x1="${z-t[1]*d}" y1="${-x+t[0]*d}" x2="${z+t[1]*d}" y2="${-x-t[0]*d}" stroke="#0c161d" stroke-width=".3"/><line x1="${z-t[1]*d}" y1="${-x+t[0]*d}" x2="${z+t[1]*d}" y2="${-x-t[0]*d}" class="portal"/>`;
      }
      for(const r of rooms){
        if(r.group==='route')continue;
        const b=r.brushes[0],c=center(b.poly),bb=bounds(b.poly);
        // Keep labels clear of the fighter silhouettes in the hangars.
        const labelZ=r.group==='flight'&&r.id!=='hangar'?bb[3]-1.15:c[1];
        const names=r.name.split(' / ');
        svg+=`<text class="plan-label" font-size="${fs}" x="${labelZ}" y="${-c[0]-.1}">${text(names[0])}</text><text class="plan-label plan-area" font-size="${fs*.7}" x="${labelZ}" y="${-c[0]+fs*1.05}">${area(b.poly).toFixed(0)} m²${names.length>1?' / '+text(names[1]):''}</text>`;
      }
      const sz=B[1],sx=-B[0]+2.1;
      svg+=`<path d="M${sz} ${sx-.3}v.6m0-.3h5m0-.3v.6" stroke="#9fadb1" stroke-width=".08"/><text x="${sz+2.5}" y="${sx+1.2}" text-anchor="middle" fill="#9fadb1" font-size=".7">5 m</text><path d="M${B[3]-4} ${sx}h4l-1-.6m1 .6l-1 .6" stroke="#ddb178" stroke-width=".08" fill="none"/><text x="${B[3]-2}" y="${sx+1.2}" text-anchor="middle" fill="#ddb178" font-size=".7">BOW</text></g></svg>`;
      $('plan').innerHTML=svg;
      if(phone)document.querySelectorAll('#plan text').forEach(e=>e.setAttribute('transform',`rotate(90 ${e.getAttribute('x')} ${e.getAttribute('y')})`));
      document.querySelectorAll('.room-shape').forEach(e=>e.onclick=()=>roomInfo(e.dataset.room));
    }
    function rebuild(){
      scene.remove(model);model.traverse(o=>{if(o.geometry)o.geometry.dispose();if(o.isLine)o.material.dispose();});
      model=new THREE.Group();scene.add(model);
      const ctx={THREE,K,state,layout:layout(),model,mats,detail};
      if(!detail?.build(ctx)&&window.ShipConcepts.buildModel)window.ShipConcepts.buildModel(ctx);
    }
    function setCamera(view='orbit'){
      if(detail?.setCamera?.(state,view,camera,controls))return;
      const box=new THREE.Box3().setFromObject(model),target=new THREE.Vector3(0,2,0);
      if(box.isEmpty()){camera.position.set(45,40,45);controls.target.copy(target);controls.update();free?.setOff();return;}
      box.getCenter(target);
      const direction=new THREE.Vector3(...(detail?.cameraDirection?.(state,view)||(view==='top'?[0,1,.0001]:view==='side'?[1,.02,0]:[.93,.83,.9]))).normalize();
      const right=new THREE.Vector3().crossVectors(new THREE.Vector3(0,1,0),direction).normalize();
      const up=new THREE.Vector3().crossVectors(direction,right).normalize();
      const tan=Math.tan(camera.fov*Math.PI/360);let distance=1;
      const points=detail?.cameraPoints?.(state)||[box.min.x,box.max.x].flatMap(x=>[box.min.y,box.max.y].flatMap(y=>[box.min.z,box.max.z].map(z=>new THREE.Vector3(x,y,z))));
      for(const point of points){
        const v=point.clone().sub(target),depth=v.dot(direction);
        distance=Math.max(distance,depth+Math.abs(v.dot(right))/(tan*camera.aspect),depth+Math.abs(v.dot(up))/tan);
      }
      controls.maxDistance=Math.max(160,distance*2);
      controls.target.copy(target);camera.position.copy(target).addScaledVector(direction,distance*(innerWidth<=760?1.15:1.05));controls.update();
      free?.setOrbit(controls.target.toArray());
    }
    function renderUI(){
      const s=state.ship,L=layout(),deck=L.decks.find(d=>d.id===state.deck);
      $('study-number').textContent=`CONCEPT ${String(s.number).padStart(2,'0')} / ${data.faction.toUpperCase()}`;
      $('name').textContent=s.name.toUpperCase();$('role').textContent=s.role;$('brief').textContent=s.brief;
      $('stats').innerHTML=[[s.length_m?s.length_m+' m':'Pending','length'],[s.beam_m?s.beam_m+' m':'Pending','beam'],[L.decks.length,'decks'],[s.crew,'crew']].map(([v,n])=>`<div><b>${v}</b><small>${n}</small></div>`).join('');
      $('study-notes').innerHTML=(state.stage==='fit'?s.adjustments:s.notes).map(n=>'<li>'+text(n)+'</li>').join('');
      $('ships').innerHTML=data.ships.map(v=>`<button data-ship="${v.id}" aria-pressed="${s.id===v.id}"><span>0${v.number}</span><small>${data.faction.toUpperCase()} / ${v.role.toUpperCase()}</small><b>${v.name}</b></button>`).join('');
      document.querySelectorAll('[data-ship]').forEach(b=>b.onclick=()=>show(b.dataset.ship,state.stage));
      document.querySelectorAll('[data-stage]').forEach(b=>{b.setAttribute('aria-pressed',b.dataset.stage===state.stage);b.disabled=(data.stage==='initial'&&b.dataset.stage!=='plan')||(data.stage==='hull'&&b.dataset.stage==='fit');b.onclick=()=>show(s.id,b.dataset.stage);});
      $('deckbar').innerHTML=L.decks.map(d=>`<button data-deck="${d.id}" aria-pressed="${d.id===state.deck}">${d.id} / ${d.name}</button>`).join('')+`<span class="eyebrow">FLOOR +${deck.floor_y_m.toFixed(1)} m</span>`;
      $('deckbar').style.display=state.stage==='hull'?'none':'flex';
      document.querySelectorAll('[data-deck]').forEach(b=>b.onclick=()=>{state.deck=b.dataset.deck;state.selected=null;update();setCamera();});
      const is3d=state.stage==='hull'||state.mode!=='plan';
      $('viewtools').innerHTML=(state.stage==='fit'?`<button data-mode="plan" aria-pressed="${state.mode==='plan'}">Plan</button><button data-mode="cutaway" aria-pressed="${state.mode==='cutaway'}">3D cutaway</button>`:'')+(is3d?'<button data-camera="orbit">Orbit</button><button data-camera="top">Top</button><button data-camera="side">Side</button>':'');
      document.querySelectorAll('[data-mode]').forEach(b=>b.onclick=()=>{state.mode=b.dataset.mode;update();setCamera();});
      document.querySelectorAll('[data-camera]').forEach(b=>b.onclick=()=>setCamera(b.dataset.camera));
      $('legend').innerHTML=Object.entries(data.palette).map(([k,c])=>`<span><i style="--tone:${c}"></i>${k}</span>`).join('');$('legend').style.display=is3d?'none':'';
      $('canvas').style.display=is3d?'block':'none';$('plan').style.display=is3d?'none':'block';
      $('viewlabel').innerHTML=`<b>${state.stage==='plan'?'01 / INITIAL PLAN':state.stage==='hull'?'02 / HULL STUDY':'03 / FITTED '+(is3d?'CUTAWAY':'PLAN')}</b><small>${state.stage==='plan'?'Functional rooms before the hull pass':state.stage==='hull'?'Rough armor, machinery and mission fittings':state.compare?'Gold dashes show the original room edges':'Rooms shaped to the occupied hull envelope'}</small>`;
      $('hint').innerHTML=is3d?'Drag to orbit / right-drag to pan<br>Scroll to inspect':'Select a room / choose a deck';
      $('normal').disabled=$('alert').disabled=!is3d||state.stage==='hull';
      $('normal').setAttribute('aria-pressed',state.lighting==='normal');$('alert').setAttribute('aria-pressed',state.lighting==='red_alert');
      $('compare').disabled=state.stage!=='fit'||is3d;
      $('compare').setAttribute('aria-pressed',state.compare);
      roomInfo(state.selected);drawPlan();
      detail?.decorate({state,show,update,setCamera,camera,controls,model,roomInfo});
    }
    function update(){rebuild();renderUI();history.replaceState(null,'','#'+[state.ship.id,state.stage,state.deck,state.mode].join('/'));}
    function show(id,stage='hull',deck=null,mode=null,lighting='normal'){
      state.ship=data.ships.find(s=>s.id===id)||data.ships[0];state.stage=['plan','hull','fit'].includes(stage)?stage:'hull';
      state.deck=state.ship.layout.decks.some(d=>d.id===deck)?deck:state.ship.layout.decks.at(-1).id;state.mode=mode||'plan';state.selected=null;state.lighting=lighting;
      update();setCamera();
    }
    for(const id of ['normal','alert'])$(id).onclick=()=>{state.lighting=id==='alert'?'red_alert':'normal';$('normal').setAttribute('aria-pressed',id==='normal');$('alert').setAttribute('aria-pressed',id==='alert');rebuild();};
    $('compare').onclick=()=>{state.compare=!state.compare;renderUI();};
    function resize(){const e=$('view');renderer.setSize(e.clientWidth,e.clientHeight);camera.aspect=e.clientWidth/e.clientHeight;camera.updateProjectionMatrix();setCamera();drawPlan();}
    new ResizeObserver(resize).observe($('view'));
    const hash=location.hash.slice(1).split('/');show(hash[0],hash[1]||(data.stage==='initial'?'plan':'hull'),hash[2],hash[3]);
    K.registerShots(data.ships.flatMap(s=>[
      {name:'initial-'+s.id,setup:()=>show(s.id,'plan')},
      ...s.layout.decks.map(d=>({name:'initial-'+s.id+'-'+d.id,setup:()=>show(s.id,'plan',d.id)})),
      {name:'hull-'+s.id,setup:()=>show(s.id,'hull')},
      {name:'top-'+s.id,setup:()=>{show(s.id,'hull');setCamera('top');}},
      ...s.layout.decks.flatMap(d=>[
        {name:'fitted-'+s.id+'-'+d.id,setup:()=>{show(s.id,'fit',d.id);state.compare=true;renderUI();}},
        {name:'cutaway-'+s.id+'-'+d.id,setup:()=>show(s.id,'fit',d.id,'cutaway')}
      ]),
      {name:'alert-'+s.id,setup:()=>show(s.id,'fit',s.layout.decks[0].id,'cutaway','red_alert')},
      {name:'reset-'+s.id,setup:()=>{state.lighting='normal';show(s.id,'hull');}}
    ]));
    window.SHIP_VIEW=window.ENEMY_VIEW={show,state,data,scene,camera,controls,renderer,layout,roomInfo,setCamera,update};
    let down=null;renderer.domElement.addEventListener('pointerdown',e=>{down=[e.clientX,e.clientY];});
    renderer.domElement.addEventListener('pointerup',e=>{
      if(!down||Math.hypot(e.clientX-down[0],e.clientY-down[1])>5||state.mode!=='cutaway')return;
      const b=renderer.domElement.getBoundingClientRect(),ray=new THREE.Raycaster();
      ray.setFromCamera(new THREE.Vector2((e.clientX-b.left)/b.width*2-1,-(e.clientY-b.top)/b.height*2+1),camera);
      const hit=ray.intersectObject(model,true).find(h=>h.object.userData.compartment);if(hit)roomInfo(hit.object.userData.compartment);
    });
    let first=true,lastFrame=performance.now();
    renderer.setAnimationLoop(()=>{const now=performance.now(),dt=Math.min(.05,(now-lastFrame)/1000);lastFrame=now;detail?.frame();if(free)free.update(dt);else controls.update();hud.beginFrame(renderer);if(state.stage==='hull'||state.mode!=='plan')renderer.render(scene,camera);hud.endFrame(renderer);if(first){first=false;$('loading').remove();K.markReady();}});
  }
  /** Offset each polygon edge inward. Concave corners retain their authored join. */
  function inset(poly,d){
    return poly.map((p,i)=>{
      const a=poly[(i+poly.length-1)%poly.length],b=poly[(i+1)%poly.length];
      const u=[p[0]-a[0],p[1]-a[1]],v=[b[0]-p[0],b[1]-p[1]],ul=Math.hypot(...u),vl=Math.hypot(...v);
      const n=[-u[1]/ul,u[0]/ul],m=[-v[1]/vl,v[0]/vl],dot=n[0]*m[0]+n[1]*m[1];
      return [p[0]+d*(n[0]+m[0])/(1+dot),p[1]+d*(n[1]+m[1])/(1+dot)];
    });
  }
  /** A bevelled prism; the supplied polygon is the outer wall, not the cap. */
  function prism(T,poly,y0,y1,bevel=0){
    const b=Math.min(bevel,(y1-y0)/3);
    const shape=new T.Shape();inset(poly,b).forEach(([x,z],i)=>i?shape.lineTo(x,-z):shape.moveTo(x,-z));shape.closePath();
    const geo=new T.ExtrudeGeometry(shape,{depth:y1-y0-2*b,steps:1,bevelEnabled:b>0,bevelThickness:b,bevelSize:b,bevelSegments:1,curveSegments:1});
    geo.rotateX(-Math.PI/2);geo.translate(0,y0+b,0);return geo;
  }
  /** Merge textured flat-shaded primitives into a single draw, using the existing array. */
  function batch(T,K,mats,entries){
    // albedo: each vertex's colour before the stand-in key light, for a page that lights the hull itself (spacedock's probes).
    const out={position:[],normal:[],color:[],albedo:[],surfUv:[],surfLayer:[],surfGlow:[]};
    const roles={};
    const sun=new T.Vector3(-.5,.9,.3).normalize();
    for(const e of entries){
      let g=e.geo;if(g.index){const old=g;g=g.toNonIndexed();old.dispose();}
      const p=g.attributes.position,n=g.attributes.normal;
      let vertices=[];
      for(let i=0;i<p.count;i+=3){
        if(e.sideOnly&&Math.abs(n.getY(i))>.5)continue;
        const tri=[0,1,2].map(k=>[p.getX(i+k),p.getY(i+k),p.getZ(i+k)]);
        let pieces=[tri];
        if(e.paint)for(const [axis,value] of [[2,e.paint.z_band_m[0]],[2,e.paint.z_band_m[1]],[0,e.paint.x_min_m],[0,-e.paint.x_min_m]]){
          const next=[];
          for(const poly of pieces){
            const halves=[[],[]];
            for(let k=0;k<poly.length;k++){
              const a=poly[k],b=poly[(k+1)%poly.length],da=a[axis]-value,db=b[axis]-value;
              if(da>=-1e-8)halves[0].push(a);if(da<=1e-8)halves[1].push(a);
              if(da*db < -1e-12){const t=da/(da-db),hit=a.map((q,j)=>q+(b[j]-q)*t);halves[0].push(hit);halves[1].push(hit);}
            }
            if(poly.every(v=>Math.abs(v[axis]-value)<1e-8))next.push(poly);
            else for(const h of halves)if(h.length>=3)next.push(h);
          }pieces=next;
        }
        for(const poly of pieces){
          const mid=poly.reduce((a,v)=>a.map((q,k)=>q+v[k]/poly.length),[0,0,0]);
          const paint=e.paint&&mid[2]>e.paint.z_band_m[0]+1e-6&&mid[2]<e.paint.z_band_m[1]-1e-6&&Math.abs(mid[0])>e.paint.x_min_m+1e-6;
          for(let k=1;k<poly.length-1;k++)for(const v of [poly[0],poly[k],poly[k+1]])vertices.push({v,n:[n.getX(i),n.getY(i),n.getZ(i)],color:paint?e.accent:e.color});
        }
      }
      // Retain component spans after merging so the overlap checker examines
      // surfaces within the same draw as well as surfaces in different meshes.
      roles['part_'+Object.keys(roles).length]=[out.position.length/3,vertices.length];
      for(const v of vertices){
        const [x,y,z]=v.v,[nx,ny,nz]=v.n;
        const c=new T.Color(v.color);
        out.albedo.push(c.r,c.g,c.b);
        const illumination=e.glow?1:.47+.53*Math.max(0,nx*sun.x+ny*sun.y+nz*sun.z);
        c.multiplyScalar(illumination);
        out.position.push(x,y,z);out.normal.push(nx,ny,nz);out.color.push(c.r,c.g,c.b);
        const span=mats.span[e.material||'exterior_pearl'];
        if(Math.abs(ny)>.65)out.surfUv.push(x/span,z/span);else if(Math.abs(nx)>.65)out.surfUv.push(z/span,y/span);else out.surfUv.push(x/span,y/span);
        out.surfLayer.push(mats.layer[e.material||'exterior_pearl']);out.surfGlow.push(e.glow?1:0);
      }
      g.dispose();
    }
    const geo=new T.BufferGeometry();for(const [k,a] of Object.entries(out))geo.setAttribute(k,new T.Float32BufferAttribute(a,['surfLayer','surfGlow'].includes(k)?1:k==='surfUv'?2:3));
    geo.userData.roles=roles;
    geo.computeBoundingSphere();
    const material=mats._conceptMaterial||(mats._conceptMaterial=K.surfaceMaterial(T,mats,{lit:false}));
    return new T.Mesh(geo,material);
  }
  function buildModel(ctx){
    const {THREE:T,K,state,model,mats}=ctx,s=state.ship;
    if(state.stage==='plan')return;
    if(state.stage==='fit'&&window.ShipConcepts.buildCutaway)return window.ShipConcepts.buildCutaway(ctx);
    const e=[],col=s.colors;
    const add=(geo,color,material='exterior_pearl',extra={})=>e.push({geo,color,material,...extra});
    const box=(c,size,color,mat='exterior_pearl',yaw=0,b=.12)=>{
      const [w,h,l]=size;const g=prism(T,[[-w/2,-l/2],[w/2,-l/2],[w/2,l/2],[-w/2,l/2]],-h/2,h/2,b);
      g.rotateY(yaw*Math.PI/180);g.translate(...c);add(g,color,mat);
    };
    for(const z of s.hull_zones){
      add(prism(T,z.poly,...z.y_m,z.bevel_m),col.armor,z.material,{paint:s.paint,accent:col.accent});
      // The lower service band follows the same footprint on every vertical face.
      if(z.service_band===false)continue;
      const band=prism(T,inset(z.poly,-.04),z.y_m[0]+.25,z.y_m[0]+.60,0);
      add(band,col.secondary,'exterior_graphite',{sideOnly:true});
    }
    for(const f of s.fittings){
      const k=f.kind;
      if(f.poly){
        const geo=prism(T,f.poly,...f.y_m,f.bevel_m||0),p=geo.attributes.position;
        // Swept wings use a physical dihedral slope; their ribs use the same plane.
        if(f.slope)for(let i=0;i<p.count;i++)p.setY(i,p.getY(i)+(Math.abs(p.getX(i))-f.slope.root_x_m)*f.slope.rise_per_m);
        geo.computeVertexNormals();
        add(geo,col[f.color_role||'armor'],f.material||'exterior_pearl');continue;
      }
      const c=f.center_m,z=f.size_m;
      if(k==='radiator'){
        box(c,z,col.secondary,'exterior_graphite');
        for(let i=0;i<8;i++)box([c[0],c[1]+z[1]/2+.055,c[2]-z[2]/2+.6+i*(z[2]-1.2)/7],[z[0]*.85,.1,.20],col.edge,'exterior_graphite',0,0);
      }else if(k==='engine'){
        if(f.tapered){
          const [w,h,l]=z;
          const g=prism(T,[[-w*.5,-l*.5],[w*.5,-l*.5],[w*.5,l*.24],[w*.28,l*.5],[-w*.28,l*.5],[-w*.5,l*.24]],-h/2,h/2,.45);
          g.translate(...c);add(g,col.armor,'exterior_pearl');
          for(const side of [-1,1])box([c[0]+side*(w/2+.025),c[1],c[2]-l*.09],[.09,h*.25,l*.61],col.accent,'exterior_pearl',0,.01);
        }else box(c,z,col.secondary,'exterior_graphite',0,.45);
        box([c[0],c[1],c[2]-z[2]/2-.06],[z[0]*.74,z[1]*.68,.14],col.glow,'light_panel',0,.03);
        for(const sign of [-1,1])box([c[0]+sign*z[0]*.34,c[1]+z[1]*.47,c[2]+.3],[.45,.4,z[2]*.7],col.armor);
      }else if(k==='laser'){
        box(c,z,col.secondary,'exterior_graphite',f.yaw_deg,.3);
        box([c[0],c[1]+.38,c[2]+z[2]*.1],[z[0]*1.1,z[1]*.78,z[2]*.69],col.armor,'exterior_pearl',0,.23);
        box([c[0],c[1],c[2]+z[2]/2+.08],[z[0]*.65,z[1]*.5,.18],col.accent,'machinery',0,.01);
      }else if(k==='turret'){
        box(c,z,col.secondary,'exterior_graphite',0,.3);
        box([c[0],c[1]+.31,c[2]-.3],[z[0]*.85,z[1]*.75,z[2]*.65],col.armor,'exterior_pearl',0,.2);
        for(const sign of [-1,1])box([c[0]+sign*.54,c[1]+.12,c[2]+z[2]*.55],[.32,.34,3.4],col.secondary,'exterior_graphite',0,.06);
      }else if(k==='baydoor'){
        box(c,z,col.secondary,'exterior_graphite',0,0);
        for(const sign of [-1,1])box([c[0]+sign*z[0]*.49,c[1],c[2]],[.15,z[1]*1.02,z[2]+.07],col.accent,'exterior_pearl',0,0);
        const frontSign=c[2]<0?-1:1;
        for(let i=0;i<3;i++)box([c[0],c[1]+(i-1)*z[1]*.29,c[2]+frontSign*(z[2]/2+.025)],[z[0]*.93,.06,.03],col.edge,'exterior_graphite',0,0);
      }else if(k==='muzzle')box(c,z,col.secondary,'machinery',f.yaw_deg,0);
      else box(c,z,k==='armor'?col.armor:col.secondary,k==='armor'?'exterior_pearl':'exterior_graphite',f.yaw_deg,.25);
    }
    const mesh=batch(T,K,mats,e);mesh.name=state.ship.layout.ship.name+' rough hull';model.add(mesh);
  }
  /** One furnished room for deck sections, window views and eye-level inspection. */
  function buildRoom(ctx,r,{cutaway=false}={}){
    const {THREE:T,K,state,layout:L,model,mats,detail}=ctx;
    const D=detail?.detailing?.()||structuredClone(K.shipData('detailing'));
    if(cutaway)D.cove.size_m=D.cove.tall_size_m=0;
    if(!detail)for(const finish of Object.values(D.finishes))if(typeof finish==='object')finish.wall='exterior_pearl';
    const built=K.buildCompartment(T,L,r,{detail:!!detail,panels:!!detail,detailing:D});
    const parts={};
    // Ceiling panels face into the room: visible below, culled in the overhead cutaway.
    for(const role of (detail?Object.keys(built.parts).filter(k=>!cutaway||!['beam','cove'].includes(k)):['floor','wall','cove']))if(built.parts[role])parts[role]=built.parts[role];
    if(detail&&r.kind==='trunk')delete parts.floor;
    if(detail)delete parts.baseboard;
    detail?.furnish(ctx,r,parts);
    detail?.dressRoom?.(ctx,r,built,parts);
    // A baked plain room is split to the deck plan's cells (bake.json mockup.cell_m), so its floor shows the lamps' pools.
    if(!detail&&window.ShipConcepts.bakePage)K.subdivideParts(parts,K.lightingData().bake.mockup.cell_m,['floor','wall','ceiling','cove'].filter(k=>parts[k]));
    const geo=K.geometryOf(T,parts,mats,K.finishOf(r,D));
    const light=()=>{if(detail?.lightRoom)detail.lightRoom(ctx,r,built,geo);else K.bakeDirect(T,geo,built.lamps,state.lighting);};
    light();
    // The room's baked light (lib/roombake.js), when the page bakes (ShipConcepts.bakePage): from its cache or live, then lit again.
    const RB=window.ShipConcepts.bakePage;
    if(RB)RB.relight(geo,built.lamps,{id:`${L.ship.id||L.ship.name}:${r.id}${cutaway?':cut':''}`,comp:r,openings:built.openings,L,DET:D,pmats:mats,onBaked:light});
    const material=mats._roomsMaterial||(mats._roomsMaterial=K.surfaceMaterial(T,mats,{lit:false}));
    const mesh=new T.Mesh(geo,material);mesh.userData.compartment=r.id;mesh.name=r.name;model.add(mesh);
    return {mesh,built};
  }
  /** Open-top deck, using the same ShipKit shells and real doorway cuts as the plans. */
  function buildCutaway(ctx){
    const {THREE:T,K,state,layout:L,model,mats,detail}=ctx,s=state.ship;
    if(state.mode!=='cutaway')return;
    const selected=L.compartments.filter(r=>r.decks.includes(state.deck)),deck=L.decks.find(d=>d.id===state.deck);
    const e=[];const add=(geo,color,material='exterior_pearl')=>e.push({geo,color,material});
    const cube=(c,size,color,material='exterior_pearl')=>{const g=new T.BoxGeometry(...size);g.translate(...c);add(g,color,material);};
    for(const r of selected){
      const viewRoom=detail&&r.kind==='trunk'?{...r,brushes:r.brushes.map(b=>({...b,y:[deck.floor_y_m,deck.floor_y_m+2.8]}))}:r;
      const {built}=buildRoom(ctx,viewRoom,{cutaway:true});
      // These are the same lamp points that light the shell, retained above the cut.
      const lamp=state.lighting==='red_alert'?'#c74739':'#e2d7b4';
      if(!detail)for(const light of built.lamps)cube(light.p,[.42,.08,.12],lamp,'light_panel');
    }
    let sectionOffset=0;
    for(const z of s.hull_zones.filter(z=>z.deck===state.deck)){
      sectionOffset+=.015;
      if(!detail)add(prism(T,z.poly,deck.floor_y_m-.30-sectionOffset,deck.floor_y_m-.07-sectionOffset,.04),s.colors.secondary,'exterior_graphite');
      const points=z.poly.concat([z.poly[0]]).map(v=>new T.Vector3(v[0],deck.floor_y_m+.03+sectionOffset,v[1]));
      const line=new T.Line(new T.BufferGeometry().setFromPoints(points),new T.LineBasicMaterial({color:0x849da6}));line.name='Hull section';model.add(line);
    }
    for(const f of s.furnishings.filter(f=>(!detail||f.kind==='person')&&selected.some(r=>r.id===f.room))){
      const [x,y,z]=f.center_m,[w,h,l]=f.size_m;
      if(f.kind==='person'){
        const color='#e9b25e';
        cube([x,y+1.12,z],[.42,.56,.23],color);cube([x,y+1.65,z],[.25,.30,.25],color);
        for(const side of [-1,1]){cube([x+side*.13,y+.42,z],[.14,.84,.20],color);cube([x+side*.29,y+1.06,z],[.12,.64,.18],color);}
      }else if(f.kind==='bunk'){
        for(const height of [.4,1.4])cube([x,y+height,z],[w-.12,.15,l-.12],'#839495','deck_tiles');
        for(const sign of [-1,1])cube([x,y+.8,z+sign*.95],[w,1.6,.07],'#464e53','machinery');
      }else if(f.kind==='console'){
        cube([x,y+.48,z],[w,.95,l],'#63717a','machinery');cube([x,y+1.0,z],[w*.82,.06,l*.6],'#7daaaa','light_panel');
      }else if(f.kind==='seat'){
        cube([x,y+.42,z],[w,.18,l],'#8d6e54');cube([x,y+.71,z-l*.36],[w-.08,.43,.10],'#8d6e54');
      }else{
        const color=f.kind==='machine'?'#7b8280':f.kind==='bed'?'#91a3a3':'#9f9580';
        cube([x,y+h/2,z],[w,h,l],color,f.kind==='machine'?'machinery':'exterior_pearl');
      }
    }
    for(const f of s.craft.filter(f=>!detail&&selected.some(r=>r.id===f.room))){
      const [x,y,z]=f.center_m,sign=f.yaw_deg===180?-1:1;
      const poly=[[0,4],[-4,-3.5],[0,-2],[4,-3.5]].map(([a,b])=>[x+a,z+b*sign]);
      if(sign===1)poly.reverse();
      add(prism(T,poly,y+.35,y+.70,0),'#939c9a');
      cube([x,y+1.1,z-1.0*sign],[1.2,.75,3.0],'#aa7354');
      cube([x,y+.55,z-3.1*sign],[1.2,.3,.15],'#e2ac72','light_panel');
    }
    for(const p of L.portals){
      if(detail)continue;
      if(!selected.some(r=>p.between.includes(r.id)))continue;
      const [x,y,z]=p.center_m;
      if(Math.abs(p.normal[1])>.5){
        // Draw the shaft on its lower deck. The upper deck shows the cut floor
        // opening; stacking both portals there would duplicate the same ladder.
        if(!selected.some(r=>p.between[0]===r.id))continue;
        for(let i=0;i<8;i++)cube([x,deck.floor_y_m+.2+i*.32,z],[.85,.05,.07],'#c4aa7e');
        for(const sign of [-1,1])cube([x+sign*.48,deck.floor_y_m+1.3,z],[.05,2.6,.05],'#c4aa7e');
      }else{
        const alongX=Math.abs(p.normal[2])>.5,w=p.size_m[0],h=p.size_m[1];
        for(const sign of [-1,1])cube([x+(alongX?sign*(w/2+.04):0),y,z+(alongX?0:sign*(w/2+.04))],[alongX?.08:.16,h,alongX?.16:.08],'#b99462');
        cube([x,y+h/2+.05,z],[alongX?w+.16:.16,.1,alongX?.16:w+.16],'#b99462');
      }
    }
    model.add(batch(T,K,mats,e));
    detail?.access(ctx);
  }
  window.ShipConcepts={start,area,bounds,center,inset,prism,batch,buildModel,buildRoom,buildCutaway};
})();
