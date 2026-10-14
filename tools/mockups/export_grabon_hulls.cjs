/* Export the shared concept interpreter's actual primitive meshes for Blender.
   Geometry and palette stay owned by the catalogue and ShipConcepts. */
const fs=require('fs'),path=require('path');
const {pathToFileURL}=require('url');
const {chromium}=require('playwright');
const root=path.resolve(__dirname,'../..');
(async()=>{
 const browser=await chromium.launch({args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
  const page=await browser.newPage();
  await page.route('https://cdn.jsdelivr.net/npm/**',r=>r.fulfill({body:fs.readFileSync(path.join(root,'tools/mockups/.cache',r.request().url().split('/npm/')[1].split('?')[0])),contentType:'text/javascript',headers:{'access-control-allow-origin':'*'}}));
  await page.goto(pathToFileURL(path.join(root,'docs/mockups/enemy-ships.html')).href);
  await page.waitForFunction(()=>window.MOCKUP_READY);
  const hulls=await page.evaluate(()=>SHIP_VIEW.data.ships.map(s=>{
   SHIP_VIEW.show(s.id,'hull');let mesh;
   SHIP_VIEW.scene.traverse(o=>{if(o.isMesh)mesh=o;});
   const g=mesh.geometry;
   return {id:s.id,attributes:Object.fromEntries(['position','normal','color','surfLayer'].map(k=>[k,Array.from(g.attributes[k].array)])),roles:g.userData.roles};
  }));
  const out=path.join(root,'build/grabon-detail');fs.mkdirSync(out,{recursive:true});
  fs.writeFileSync(path.join(out,'hulls.json'),JSON.stringify(hulls));
  console.log('Exported three catalogue hulls with primitive spans and vertex colors.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
