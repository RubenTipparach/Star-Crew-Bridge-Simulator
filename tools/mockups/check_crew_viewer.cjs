// Run with Playwright available through NODE_PATH, like shoot.mjs.
const {chromium} = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname,'../..');
const manifest = JSON.parse(fs.readFileSync(path.join(root,'assets/models/crew_review/characters.json'),'utf8'));
const file = pathToFileURL(path.join(root,'docs/mockups/crew-characters.html')).href;
const errors = [], checks = [];
const frames = (page,n=20) => page.evaluate(n => new Promise(resolve => {
  let count=0; const step=() => ++count >= n ? resolve() : requestAnimationFrame(step);
  requestAnimationFrame(step);
}),n);
const state = page => page.evaluate(() => window.MOCKUP_CREW.read());
const vertexPositions = page => page.evaluate(() => {
  const points=[];
  window.MOCKUP_SCENE.traverse(o => {
    if (!o.isSkinnedMesh || o.skeleton.bones.length!==30) return;
    o.skeleton.update();
    const vector=o.position.clone();
    // Use actual skinned vertices, not only a UI time readout or bone names.
    for(let i=0;i<o.geometry.attributes.position.count;i+=11) points.push(...o.getVertexPosition(i,vector).toArray());
  });
  return points;
});
async function open(browser,size) {
  const page = await browser.newPage({viewport:size});
  await page.route('https://cdn.jsdelivr.net/npm/**',route => {
    const rel = route.request().url().replace('https://cdn.jsdelivr.net/npm/','').split('?')[0];
    return route.fulfill({body:fs.readFileSync(path.join(root,'tools/mockups/.cache',rel)),contentType:'application/javascript',headers:{'access-control-allow-origin':'*'}});
  });
  page.on('pageerror',e => errors.push(e.message));
  page.on('console',m => { if(m.type()==='error') errors.push(m.text()); });
  await page.goto(file);
  await page.waitForFunction(() => window.MOCKUP_READY===true);
  await frames(page);
  return page;
}
(async() => {
  const browser = await chromium.launch({args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  try {
    const page = await open(browser,{width:1440,height:900});
    for(const row of manifest.characters) {
      await page.selectOption('#sex',row.sex);
      await page.selectOption('#character',row.id);
      await frames(page);
      const current=await state(page);
      assert.deepEqual(current.visible,[row.id]);
      assert.equal(current.triangles,row.triangles+192);
      assert.equal(current.drawCalls,2);
      assert.match(await page.locator('#asset-info').innerText(),/AWAITING APPROVAL/);
      const download=page.locator('#asset-info a');
      assert.equal(await download.getAttribute('download'),row.file);
      const data=await download.getAttribute('href');
      const bytes=Buffer.from(data.split(',')[1],'base64');
      assert.deepEqual(bytes,fs.readFileSync(path.join(root,'assets/models/crew_review',row.file)));
      checks.push({character:row.id,triangles:current.triangles,drawCalls:current.drawCalls,glbDownload:'exact bytes'});
    }
    await page.selectOption('#character','all'); await frames(page);
    const lineup=await state(page);
    assert.equal(lineup.triangles,manifest.characters.filter(r=>r.sex===lineup.sex).reduce((s,r)=>s+r.triangles,192));
    assert.equal(lineup.drawCalls,7);
    assert.equal(lineup.visible.length,6);
    assert.equal(await page.locator('[data-view="eyes"]').isDisabled(),true);
    checks.push({lineup:lineup.triangles,sex:lineup.sex,drawCalls:lineup.drawCalls});
    await page.selectOption('#sex','male'); await frames(page);
    assert.equal((await state(page)).visible.length,6);
    assert.ok((await state(page)).visible.every(id=>manifest.characters.find(r=>r.id===id).sex==='male'));
    checks.push({pairedVariants:'six departments, both body variants selected and rendered'});
    await page.selectOption('#sex','male');
    await page.selectOption('#character','command_01');
    for(const view of ['front','side','back','eyes','three-quarter']) {
      await page.click(`[data-view="${view}"]`); await frames(page,5);
      assert.equal(await page.locator(`[data-view="${view}"]`).getAttribute('aria-pressed'),'true');
    }
    checks.push({cameraPresets:'front, side, back, eyes, reset'});
    const lightFrames=[];
    for(const light of ['normal','red_alert','emergency']) {
      await page.click(`[data-light="${light}"]`); await frames(page);
      assert.equal((await state(page)).lightState,light);
      lightFrames.push(await page.screenshot());
    }
    assert.notDeepEqual(lightFrames[0],lightFrames[1]);
    assert.notDeepEqual(lightFrames[1],lightFrames[2]);
    checks.push({lighting:'three distinct rendered states'});
    await page.check('#wireframe'); await frames(page);
    assert.equal((await state(page)).wireframe,true);
    assert.equal((await state(page)).triangles,192);
    await page.uncheck('#wireframe');
    await page.click('[data-view="front"]'); await frames(page);
    const beforeOrbit=await state(page);
    await page.mouse.move(780,420); await page.mouse.down();
    await page.mouse.move(900,450,{steps:10}); await page.mouse.up(); await frames(page,35);
    assert.notDeepEqual((await state(page)).camera,beforeOrbit.camera);
    const beforePan=await state(page);
    await page.mouse.move(780,420); await page.mouse.down({button:'right'});
    await page.mouse.move(820,400,{steps:8}); await page.mouse.up({button:'right'}); await frames(page,35);
    assert.notDeepEqual((await state(page)).target,beforePan.target);
    const beforeZoom=await state(page);
    await page.mouse.wheel(0,-200); await frames(page,30);
    assert.notDeepEqual((await state(page)).camera,beforeZoom.camera);
    await page.check('#rotate'); const beforeRotate=await state(page); await frames(page,20);
    assert.notDeepEqual((await state(page)).camera,beforeRotate.camera);
    await page.uncheck('#rotate');
    checks.push({mouse:'orbit, pan, zoom, auto rotate verified'});
    await page.click('[data-view="front"]');
    const rest=await vertexPositions(page);
    for(const clip of manifest.characters[0].animations) {
      await page.selectOption('#animation',clip.name); await frames(page,3);
      assert.equal((await state(page)).clip,clip.name);
      assert.notDeepEqual(await vertexPositions(page),rest);
      assert.equal((await state(page)).triangles,manifest.characters[0].triangles+192);
      assert.equal((await state(page)).drawCalls,2);
    }
    await page.selectOption('#animation','Walk_Loop');
    await page.click('#play'); const initial=await vertexPositions(page); await frames(page,8);
    assert.notDeepEqual(await vertexPositions(page),initial);
    await page.click('#play'); const paused=await vertexPositions(page); await frames(page,8);
    assert.deepEqual(await vertexPositions(page),paused);
    await page.locator('#timeline').fill('1'); await frames(page,3);
    const lastKey=await vertexPositions(page);
    await page.locator('#timeline').fill('0.25'); await frames(page,3);
    assert.equal((await state(page)).time,.25);
    assert.notDeepEqual(await vertexPositions(page),lastKey);
    await page.locator('#timeline').fill('0'); await frames(page,3);
    const startKey=await vertexPositions(page);
    const loopError=Math.max(...startKey.map((value,index)=>Math.abs(value-lastKey[index])));
    assert.ok(loopError<1e-5,`loop vertex error ${loopError}; state ${JSON.stringify(await state(page))}`);
    await page.check('#skeleton'); await frames(page,3);
    assert.equal((await state(page)).drawCalls,3);
    await page.selectOption('#character','all'); await frames(page,3);
    assert.equal((await state(page)).drawCalls,13);
    await page.uncheck('#skeleton');
    await page.selectOption('#sex','male');
    await page.selectOption('#character','command_01');
    await page.selectOption('#animation','rest'); await frames(page,3);
    const restored=await vertexPositions(page);
    assert.ok(restored.every((value,index)=>Math.abs(value-rest[index])<1e-5));
    checks.push({rig:'all five baked clips deform the mesh; playback, pause, reverse scrubbing, loop closure, rest reset and skeleton overlay verified'});
    await page.locator('.mk-x').first().click();
    assert.equal(await page.locator('.controls').isVisible(),false);
    await page.locator('.mk-show').click();
    assert.equal(await page.locator('.controls').isVisible(),true);
    checks.push({panels:'close and restore verified'});
    const phone=await open(browser,{width:390,height:844});
    const controlBox=await phone.locator('.controls').boundingBox();
    assert.ok(controlBox.x>=0 && controlBox.x+controlBox.width<=390);
    assert.ok(controlBox.y>=0 && controlBox.y+controlBox.height<=844);
    for(const element of await phone.locator('.controls button, .controls select').all()) {
      const b=await element.boundingBox();
      assert.ok(b.x>=0 && b.x+b.width<=390);
      assert.ok(b.y>=0 && b.y+b.height<=844);
    }
    assert.equal(await phone.locator('#asset-info').isVisible(),false);
    await phone.screenshot({path:path.join(root,'docs/screenshots/mockups/crew-characters-mobile.png')});
    const phoneShots=path.join(root,'docs/screenshots/crew-characters/mobile-viewer');
    fs.mkdirSync(phoneShots,{recursive:true});
    await phone.selectOption('#character','all'); await frames(phone,3);
    await phone.screenshot({path:path.join(phoneShots,'crew-characters-lineup.png')});
    await phone.selectOption('#character','command_01'); await phone.click('[data-view="eyes"]'); await frames(phone,3);
    await phone.screenshot({path:path.join(phoneShots,'crew-characters-eyes.png')});
    await phone.selectOption('#animation','Wave_Loop'); await phone.click('[data-view="front"]');
    await phone.check('#skeleton'); await frames(phone,3);
    await phone.screenshot({path:path.join(phoneShots,'rig.png')});
    checks.push({mobile:'390x844, controls fit, extra panels closed'});
    assert.deepEqual(errors,[]);
    const report={schema:'starcrew.crew-viewer-qa/1',page:'docs/mockups/crew-characters.html',opened_from_disk:true,renderer:'headless Chromium SwiftShader',pi_measurement:false,owner_approval:'pending',checks,errors};
    fs.writeFileSync(path.join(root,'docs/screenshots/crew-characters/viewer-validation.json'),JSON.stringify(report,null,2)+'\n');
    console.log(JSON.stringify(report,null,2));
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode=1; });
