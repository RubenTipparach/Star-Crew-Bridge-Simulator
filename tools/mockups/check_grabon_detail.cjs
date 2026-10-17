/* Verify detailed Grabon review controls, actual geometry, access and draw counts. */
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {pathToFileURL}=require('url'),{chromium}=require('playwright');
const root=path.resolve(__dirname,'../..'),out=path.join(root,'docs/screenshots/grabon-detail');
const frames=(page,n=12)=>page.evaluate(n=>new Promise(resolve=>{let k=0;const frame=()=>++k===n?resolve():requestAnimationFrame(frame);requestAnimationFrame(frame);}),n);
(async()=>{
 const browser=await chromium.launch({args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.route('https://cdn.jsdelivr.net/npm/**',r=>r.fulfill({body:fs.readFileSync(path.join(root,'tools/mockups/.cache',r.request().url().split('/npm/')[1].split('?')[0])),contentType:'text/javascript',headers:{'access-control-allow-origin':'*'}}));
  await page.goto(pathToFileURL(path.join(root,process.argv[2]||'docs/mockups/grabon-ships.html')).href);await page.waitForFunction(()=>window.MOCKUP_READY);
  assert.deepEqual(await page.evaluate(()=>SHIP_VIEW.data),JSON.parse(fs.readFileSync(path.join(root,'data/ships/grabon-detailed/layouts.json'),'utf8')));
  const report={status:'pass',views:[],room_navigation:[],errors};
  for(const ship of await page.evaluate(()=>SHIP_VIEW.data.ships.map(s=>({id:s.id,decks:s.layout.decks})))){
   const states=[['hull','A','plan'],...ship.decks.map(d=>['fit',d.id,'cutaway']),['fit','A','interior'],['fit',ship.decks.at(-1).id,'access']];
   for(const [stage,deck,mode] of states){
    await page.evaluate(a=>SHIP_VIEW.show(...a),[ship.id,stage,deck,mode]);await frames(page);
    const facts=await page.evaluate(()=>{
     const v=SHIP_VIEW,roles={},ceilings=[];let bad=0;
     v.scene.traverse(o=>{if(!o.isMesh)return;for(const a of Object.values(o.geometry.attributes))for(const n of a.array)if(!Number.isFinite(n))bad++;
      for(const [role,[start,count]] of Object.entries(o.geometry.userData.roles||{}))roles[role]=(roles[role]||0)+count;
      if(v.state.stage==='fit'&&v.state.mode==='cutaway'&&o.userData.compartment){
       const [start,count]=o.geometry.userData.roles.ceiling||[0,0],normal=o.geometry.attributes.normal;
       let inward=true;for(let i=start;i<start+count;i++)if(normal.getY(i)>-.99)inward=false;
       ceilings.push({room:o.userData.compartment,vertices:count,inward,front_side:o.material.side===0});
      }
     });
     const holes=[];
     if(v.state.stage==='fit'&&v.state.mode==='cutaway')for(const portal of v.layout().portals.filter(p=>p.kind==='ladder')){
      const upper=v.layout().compartments.find(r=>r.id===portal.between[1]),y=upper.brushes[0].y[0];
      if(!upper.decks.includes(v.state.deck))continue;
      let room;v.scene.traverse(o=>{if(o.userData.compartment===upper.id)room=o;});
      const a=room.geometry.attributes.position.array;
      const covered=(x,z)=>{for(let i=0;i<a.length;i+=9){
       if([a[i+1],a[i+4],a[i+7]].some(h=>Math.abs(h-y)>.06))continue;
       const p=[[a[i],a[i+2]],[a[i+3],a[i+5]],[a[i+6],a[i+8]]],cross=[];
       for(let j=0;j<3;j++){const q=p[j],r=p[(j+1)%3];cross.push((r[0]-q[0])*(z-q[1])-(r[1]-q[1])*(x-q[0]));}
       if(Math.abs(cross.reduce((a,b)=>a+b,0))>1e-6&&(cross.every(n=>n>=-1e-6)||cross.every(n=>n<=1e-6)))return true;
      }return false;};
      holes.push({id:portal.id,open:!covered(portal.center_m[0],portal.center_m[2]),bypass:covered(.6,portal.center_m[2])});
     }
     let lightRange=null,exterior=null;
     if(v.state.stage==='hull'){
      const vertices=[],emitter=[],positions=[];v.scene.updateMatrixWorld(true);
      v.scene.traverse(o=>{if(!o.isMesh||!o.userData.exterior)return;const g=o.geometry,p=g.attributes.position,uv=g.attributes.uv;
       if(o.material.map){const c=g.attributes.color;let lo=1,hi=0;for(let i=0;i<c.count;i++){const n=c.getX(i);lo=Math.min(lo,n);hi=Math.max(hi,n);}lightRange=[lo,hi];}
       for(let i=0;i<p.count;i++){
        const q=v.camera.position.clone().set(p.getX(i),p.getY(i),p.getZ(i)).applyMatrix4(o.matrixWorld);
        const point=[q.x,q.y,q.z];positions.push(point);
        if(o.material.name==='Impulse emitters')emitter.push(point);
        vertices.push([...point,uv?uv.getX(i):0,uv?uv.getY(i):0,o.material.name]);
       }
      });
      const key=a=>a.map(n=>typeof n==='number'?Number(n.toFixed(3)):n).join('|'),set=new Set(vertices.map(key));
      const bounds=a=>[0,1,2].map(k=>[Math.min(...a.map(p=>p[k])),Math.max(...a.map(p=>p[k]))]);
      exterior={bounds_m:bounds(positions),impulse_bounds_m:bounds(emitter),impulse_vertices:emitter.length,windows:GRABON_DETAIL.windows.length,window_rays:GRABON_DETAIL.validateWindows(),
       unmatched_mirror_vertices:vertices.filter(a=>!set.has(key([-a[0],...a.slice(1)]))).length};
     }
     const interior=v.state.mode==='interior'?{ceiling_vertices:roles.ceiling||0,eye_height_m:v.camera.position.y-GRABON_DETAIL.floorAt(),clear_spawn:GRABON_DETAIL.walkable(v.camera.position.x,v.camera.position.z)}:null;
     return {ship:v.state.ship.id,stage:v.state.stage,deck:v.state.deck,mode:v.state.mode,triangles:v.renderer.info.render.triangles,draws:v.renderer.info.render.calls,texture_mib:MOCKUP_DRAWS().textureMB,nonfinite:bad,stair_vertices:roles.stair||0,ladder_openings:holes,cutaway_ceilings:ceilings,sun_range:lightRange,exterior,interior};
    });
    assert.equal(facts.nonfinite,0,JSON.stringify(facts));assert(facts.triangles<=(stage==='hull'?200000:60000),JSON.stringify(facts));assert(facts.draws<=120);assert(facts.texture_mib<=96);
    for(const hole of facts.ladder_openings)assert(hole.open&&hole.bypass,'Actual upper floor must have a ladder hole and solid bypass: '+JSON.stringify(hole));
    for(const ceiling of facts.cutaway_ceilings)assert(ceiling.vertices>0&&ceiling.inward&&ceiling.front_side,'Cutaway ceilings must remain visible from inside and culled from above: '+JSON.stringify(ceiling));
    if(facts.interior){assert(facts.interior.ceiling_vertices>0);assert(Math.abs(facts.interior.eye_height_m-1.65)<.001);assert(facts.interior.clear_spawn,'Interior camera must start clear of walls and furniture');}
    if(facts.sun_range)assert(facts.sun_range[0]<.6&&facts.sun_range[1]>.85,'Exported hull must carry its fixed sun lighting in COLOR_0');
    if(facts.exterior){
     const e=facts.exterior;assert(e.impulse_vertices>100,'Each hull must have a modeled central impulse aperture');
     const center=e.impulse_bounds_m.map(([a,b])=>(a+b)/2);assert(Math.abs(center[0])<.001,'Impulse must sit on the centerline');
     assert(Math.abs(center[1]-(ship.id==='gorath'?5.2:5.15))<.001,'Impulse must sit at the middle tier height');
     assert.equal(e.unmatched_mirror_vertices,0,'Exported geometry and UVs must mirror exactly');
     if(ship.id==='shrike')assert(e.bounds_m[2][0]>-22.5,'Shrike must not retain its projecting aft engine cubes');
    }
    if(mode==='access')assert(facts.stair_vertices>=200,'Stairs must have actual tread geometry');report.views.push(facts);
   }
   await page.click('[data-lift="A"]');await page.waitForTimeout(5200);
   const lift=await page.evaluate(()=>{const l=GRABON_DETAIL.lift;return [l.car.position.y+l.car.userData.floorOrigin,l.goal];});
   assert(Math.abs(lift[0]-lift[1])<.02,'Lift must reach the selected floor');
   await page.screenshot({path:path.join(out,`lift-${ship.id}-upper.png`)});
   const rooms=await page.evaluate(id=>{
    const ship=SHIP_VIEW.data.ships.find(s=>s.id===id),checked=[];
    for(const room of ship.layout.compartments.filter(r=>r.kind!=='trunk')){
     SHIP_VIEW.show(id,'fit',room.decks[0],'interior');
     const picker=document.getElementById('room-select');picker.value=room.id;picker.dispatchEvent(new Event('change'));
     const c=SHIP_VIEW.camera.position;
     if(!GRABON_DETAIL.walkable(c.x,c.z)||Math.abs(c.y-GRABON_DETAIL.floorAt()-1.65)>.001)throw new Error(id+'/'+room.id+': invalid standing view');
     checked.push(room.id);
    }
    return checked;
   },ship.id);
   await page.evaluate(id=>{SHIP_VIEW.show(id,'fit','A','interior');const s=document.getElementById('room-select');s.value='mess';s.dispatchEvent(new Event('change'));s.blur();},ship.id);
   const before=await page.evaluate(()=>SHIP_VIEW.camera.position.toArray());
   await page.keyboard.down('w');await frames(page,10);await page.keyboard.up('w');
   const walked=await page.evaluate(()=>({position:SHIP_VIEW.camera.position.toArray(),clear:GRABON_DETAIL.walkable(SHIP_VIEW.camera.position.x,SHIP_VIEW.camera.position.z)}));
   assert(walked.clear);assert(Math.hypot(...walked.position.map((n,i)=>n-before[i]))>.02,'W must move the interior camera');
   report.room_navigation.push({ship:ship.id,rooms,keyboard_walk:true});
  }
  await page.setViewportSize({width:390,height:844});await frames(page);
  for(const mode of ['hull','cutaway','interior','access']){
   await page.evaluate(mode=>SHIP_VIEW.show('gorath',mode==='hull'?'hull':'fit','B',mode==='hull'?'plan':mode),mode);await frames(page);
   const overflow=await page.evaluate(()=>Array.from(document.querySelectorAll('#viewtools button,#deckbar button,#lift-controls button')).filter(e=>{const r=e.getBoundingClientRect();return r.width&&r.right>innerWidth+1;}).map(e=>e.textContent));assert.deepEqual(overflow,[],'Phone controls fit');
   await page.screenshot({path:path.join(out,`phone-${mode}.png`)});
  }
  assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'browser-validation.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
