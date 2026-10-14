/* Review instrument for faction study HTML. Checks actual rendered
   budgets, source freshness, stage/deck controls, camera controls and mobile UI. */
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {pathToFileURL}=require('url');
const {chromium}=require('playwright');
const ROOT=path.resolve(__dirname,'../..');
const faction=process.argv[2]||'grabon';assert(['grabon','federation'].includes(faction));
const out=path.join(ROOT,`docs/screenshots/${faction}-concepts`);
const file=path.join(ROOT,`docs/mockups/${faction==='federation'?'federation-ships':'enemy-ships'}.html`);
const source=JSON.parse(fs.readFileSync(path.join(ROOT,`data/ships/${faction}-concepts/layouts.json`),'utf8'));
const frames=(p,n=5)=>p.evaluate(n=>new Promise(resolve=>{let k=0;function f(){if(++k>=n)resolve();else requestAnimationFrame(f);}requestAnimationFrame(f);}),n);

(async()=>{
  const browser=await chromium.launch({args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  const page=await browser.newPage({viewport:{width:1440,height:900}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await page.route('https://cdn.jsdelivr.net/npm/**',r=>{
    const rel=r.request().url().split('/npm/')[1].split('?')[0];
    return r.fulfill({body:fs.readFileSync(path.join(ROOT,'tools/mockups/.cache',rel)),contentType:'text/javascript',headers:{'access-control-allow-origin':'*'}});
  });
  try {
    await page.goto(pathToFileURL(file).href);await page.waitForFunction(()=>window.MOCKUP_READY);
    assert.deepEqual(await page.evaluate(()=>ENEMY_VIEW.data),source,'The HTML must carry the current catalogue');
    const report={base_commit:source.base_commit,status:'pass',views:[],mobile:[],errors};
    for(const ship of source.ships){
      await page.click(`[data-ship="${ship.id}"]`);await page.click('[data-stage="hull"]');await frames(page);
      const hull=await page.evaluate(()=>{
        const v=ENEMY_VIEW,lo=[Infinity,Infinity,Infinity],hi=[-Infinity,-Infinity,-Infinity];
        v.scene.traverse(o=>{if(!o.isMesh)return;const a=o.geometry.attributes.position;for(let i=0;i<a.count;i++){const p=[a.getX(i),a.getY(i),a.getZ(i)];p.forEach((x,k)=>{lo[k]=Math.min(lo[k],x);hi[k]=Math.max(hi[k],x);});}});
        return {ship:v.state.ship.id,stage:'hull',triangles:v.renderer.info.render.triangles,draw_calls:v.renderer.info.render.calls,texture_mib:MOCKUP_BUDGET().textureMB,extent_m:hi.map((q,k)=>+(q-lo[k]).toFixed(3))};
      });
      assert.equal(hull.ship,ship.id);assert(hull.triangles>500&&hull.triangles<=12000);assert(hull.draw_calls<=120);assert(hull.texture_mib<=4);
      assert(Math.abs(hull.extent_m[0]-ship.beam_m)<.1&&Math.abs(hull.extent_m[2]-ship.length_m)<.1,'Displayed dimensions must match the actual mesh');
      report.views.push(hull);
      const before=await page.evaluate(()=>ENEMY_VIEW.camera.position.toArray());await page.click('[data-camera="side"]');
      assert.notDeepEqual(await page.evaluate(()=>ENEMY_VIEW.camera.position.toArray()),before);
      await page.click('[data-stage="plan"]');await frames(page);
      assert.equal(await page.locator('#plan svg').count(),1);
      assert.equal(await page.evaluate(()=>ENEMY_VIEW.state.stage),'plan');
      for(const deck of ship.layout.decks){
        await page.click('[data-stage="fit"]');await page.click(`[data-deck="${deck.id}"]`);
        const count=ship.layout.compartments.filter(r=>r.decks.includes(deck.id)).length;
        assert.equal(await page.locator('.room-shape').count(),count);
        const first=ship.layout.compartments.find(r=>r.decks.includes(deck.id)&&r.kind!=='corridor');
        await page.click(`[data-room="${first.id}"]`);assert.equal(await page.evaluate(()=>ENEMY_VIEW.state.selected),first.id);
        await page.click('[data-mode="cutaway"]');await frames(page);
        const cost=await page.evaluate(()=>({ship:ENEMY_VIEW.state.ship.id,stage:'cutaway',deck:ENEMY_VIEW.state.deck,triangles:ENEMY_VIEW.renderer.info.render.triangles,draw_calls:ENEMY_VIEW.renderer.info.render.calls,rooms:ENEMY_VIEW.scene.children.flatMap(g=>g.children).filter(o=>o.userData.compartment).length}));
        assert.equal(cost.rooms,count);assert(cost.triangles<=12000&&cost.draw_calls<=120);report.views.push(cost);
      }
      await page.click('#alert');assert.equal(await page.evaluate(()=>ENEMY_VIEW.state.lighting),'red_alert');await page.click('#normal');
      await page.click(`[data-deck="${ship.layout.decks[0].id}"]`);
      const centered=await page.evaluate(()=>{
        const v=SHIP_VIEW,lo=[Infinity,Infinity,Infinity],hi=[-Infinity,-Infinity,-Infinity];
        v.scene.traverse(o=>{const a=o.geometry?.attributes.position;if(!a)return;for(let i=0;i<a.count;i++)[a.getX(i),a.getY(i),a.getZ(i)].forEach((q,k)=>{lo[k]=Math.min(lo[k],q);hi[k]=Math.max(hi[k],q);});});
        return v.controls.target.toArray().every((q,k)=>Math.abs(q-(lo[k]+hi[k])/2)<.01);
      });
      assert(centered,'Switching 3D decks must recenter on the selected deck');
    }
    await page.setViewportSize({width:390,height:844});await frames(page);
    fs.mkdirSync(path.join(out,'mobile'),{recursive:true});
    const [first,second]=source.ships;
    const mobileStates=[...source.ships.map(s=>[s.id,'hull',s.layout.decks.at(-1).id,'plan']),[second.id,'fit',second.layout.decks.at(-1).id,'plan'],[first.id,'fit','A','cutaway']];
    for(const [ship,stage,deck,mode] of mobileStates){
      await page.evaluate(args=>ENEMY_VIEW.show(...args),[ship,stage,deck,mode]);await frames(page);
      const bad=await page.evaluate(()=>{
        const nodes=[...document.querySelectorAll('header button,.steps button,#deckbar button,#viewtools button,footer button')].filter(e=>e.getClientRects().length);
        const rects=nodes.map(e=>({text:e.textContent,r:e.getBoundingClientRect()})),out=[];
        for(const a of rects){const r=a.r;if(r.x<0||r.y<0||r.right>innerWidth+.5||r.bottom>innerHeight+.5)out.push(a.text+' outside screen');}
        for(let i=0;i<rects.length;i++)for(const b of rects.slice(i+1)){const a=rects[i];if(Math.min(a.r.right,b.r.right)-Math.max(a.r.x,b.r.x)>2&&Math.min(a.r.bottom,b.r.bottom)-Math.max(a.r.y,b.r.y)>2)out.push(a.text+' overlaps '+b.text);}
        return out;
      });
      assert.deepEqual(bad,[],`${ship}/${stage}: controls must fit on a phone`);
      if(stage==='hull'){
        const clipped=await page.evaluate(()=>{
          const {scene,camera}=SHIP_VIEW,v=camera.position.clone();let outside=0;
          scene.updateMatrixWorld(true);camera.updateMatrixWorld(true);
          scene.traverse(o=>{if(!o.isMesh)return;const a=o.geometry.attributes.position;for(let i=0;i<a.count;i++){
            v.fromBufferAttribute(a,i).applyMatrix4(o.matrixWorld).project(camera);
            if(Math.abs(v.x)>1||Math.abs(v.y)>1)outside++;
          }});return outside;
        });
        assert.equal(clipped,0,`${ship}: the complete hull must fit in the phone viewport`);
      }
      const name=`${ship}-${stage}-${mode}.png`;await page.screenshot({path:path.join(out,'mobile',name)});report.mobile.push(name);
    }
    // Shared URLs must reopen the actual selected 3D deck, not just its 2D plan.
    await page.reload();await page.waitForFunction(()=>window.MOCKUP_READY);
    assert.equal(await page.evaluate(()=>ENEMY_VIEW.state.mode),'cutaway');
    assert.equal(await page.evaluate(()=>ENEMY_VIEW.state.deck),'A');
    assert.equal(await page.locator('header a').count(),1,'Faction navigation is present');
    assert.deepEqual(errors,[],'The delivered viewer must not log browser errors');
    fs.writeFileSync(path.join(out,'browser-validation.json'),JSON.stringify(report,null,2)+'\n');
    console.log(JSON.stringify(report,null,2));
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
