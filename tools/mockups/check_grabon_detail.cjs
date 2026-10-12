/* Verify detailed Grabon review controls, actual geometry, access and budgets. */
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
  await page.goto(pathToFileURL(path.join(root,'docs/mockups/grabon-ships.html')).href);await page.waitForFunction(()=>window.MOCKUP_READY);
  assert.deepEqual(await page.evaluate(()=>SHIP_VIEW.data),JSON.parse(fs.readFileSync(path.join(root,'data/ships/grabon-detailed/layouts.json'),'utf8')));
  const report={status:'pass',views:[],errors};
  for(const ship of await page.evaluate(()=>SHIP_VIEW.data.ships.map(s=>({id:s.id,decks:s.layout.decks})))){
   const states=[['hull','A','plan'],...ship.decks.map(d=>['fit',d.id,'cutaway']),['fit',ship.decks.at(-1).id,'access']];
   for(const [stage,deck,mode] of states){
    await page.evaluate(a=>SHIP_VIEW.show(...a),[ship.id,stage,deck,mode]);await frames(page);
    const facts=await page.evaluate(()=>{
     const v=SHIP_VIEW,roles={};let bad=0;
     v.scene.traverse(o=>{if(!o.isMesh)return;for(const a of Object.values(o.geometry.attributes))for(const n of a.array)if(!Number.isFinite(n))bad++;
      for(const [role,[start,count]] of Object.entries(o.geometry.userData.roles||{}))roles[role]=(roles[role]||0)+count;
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
     let lightRange=null;if(v.state.stage==='hull')v.scene.traverse(o=>{if(o.isMesh&&o.material.map){const c=o.geometry.attributes.color;let lo=1,hi=0;for(let i=0;i<c.count;i++){const n=c.getX(i);lo=Math.min(lo,n);hi=Math.max(hi,n);}lightRange=[lo,hi];}});
     return {ship:v.state.ship.id,stage:v.state.stage,deck:v.state.deck,mode:v.state.mode,triangles:v.renderer.info.render.triangles,draws:v.renderer.info.render.calls,texture_mib:MOCKUP_BUDGET().textureMB,nonfinite:bad,stair_vertices:roles.stair||0,ladder_openings:holes,sun_range:lightRange};
    });
    assert.equal(facts.nonfinite,0,JSON.stringify(facts));assert(facts.triangles<=60000);assert(facts.draws<=120);assert(facts.texture_mib<=96);
    for(const hole of facts.ladder_openings)assert(hole.open&&hole.bypass,'Actual upper floor must have a ladder hole and solid bypass: '+JSON.stringify(hole));
    if(facts.sun_range)assert(facts.sun_range[0]<.6&&facts.sun_range[1]>.85,'Exported hull must carry its fixed sun lighting in COLOR_0');
    if(mode==='access')assert(facts.stair_vertices>=200,'Stairs must have actual tread geometry');report.views.push(facts);
   }
   await page.click('[data-lift="A"]');await page.waitForTimeout(5200);
   const lift=await page.evaluate(()=>{const l=GRABON_DETAIL.lift;return [l.car.position.y+l.car.userData.floorOrigin,l.goal];});
   assert(Math.abs(lift[0]-lift[1])<.02,'Lift must reach the selected floor');
   await page.screenshot({path:path.join(out,`lift-${ship.id}-upper.png`)});
  }
  await page.setViewportSize({width:390,height:844});await frames(page);
  for(const mode of ['hull','cutaway','access']){
   await page.evaluate(mode=>SHIP_VIEW.show('gorath',mode==='hull'?'hull':'fit','B',mode==='hull'?'plan':mode),mode);await frames(page);
   const overflow=await page.evaluate(()=>Array.from(document.querySelectorAll('#viewtools button,#deckbar button,#lift-controls button')).filter(e=>{const r=e.getBoundingClientRect();return r.width&&r.right>innerWidth+1;}).map(e=>e.textContent));assert.deepEqual(overflow,[],'Phone controls fit');
   await page.screenshot({path:path.join(out,`phone-${mode}.png`)});
  }
  assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'browser-validation.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
