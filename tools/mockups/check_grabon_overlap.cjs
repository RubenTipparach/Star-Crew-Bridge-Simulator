/* Run the repository overlap check over every detailed hull, deck and access view. */
const fs=require('fs'),cp=require('child_process'),path=require('path');
process.chdir(path.resolve(__dirname,'../..'));
fs.mkdirSync('docs/screenshots/grabon-detail',{recursive:true});
const data=JSON.parse(fs.readFileSync('data/ships/grabon-detailed/layouts.json','utf8')),log=[];
const nativeOnly=process.argv.includes('--native-only');
for(const ship of data.ships)for(const [stage,deck,mode] of [['hull','A','plan'],['hull','A','rooms'],...ship.layout.decks.map(d=>['fit',d.id,'cutaway']),['fit',ship.layout.decks.at(-1).id,'access']]){
 if(nativeOnly&&(stage!=='hull'||mode!=='plan'))continue;
 let setup=`SHIP_VIEW.show(${JSON.stringify(ship.id)},${JSON.stringify(stage)},${JSON.stringify(deck)},${JSON.stringify(mode)})`;
 // Inspect native hull faces and furnished pressure rooms separately. The
 // window-ray checks verify their join. Keep ceilings in the full-room pass.
 if(stage==='hull')setup+=`;SHIP_VIEW.scene.traverse(o=>{if(o.isMesh){o.visible=${mode==='rooms'?'!':'!!'}o.userData.exterior;if(o.visible&&o.userData.exterior){const g=o.geometry.index?o.geometry.toNonIndexed():o.geometry;g.userData.roles={};for(let i=0;i<g.attributes.position.count;i+=3)g.userData.roles['face_'+i]=[i,3];o.geometry=g;}}})`;
 const r=cp.spawnSync(process.execPath,['tools/mockups/zfight.mjs','docs/mockups/grabon-ships.html','--setup',setup,'--max-m2','.001'],{encoding:'utf8',timeout:150000});
 log.push(`${ship.id} / ${stage} / ${deck} / ${mode}\n${r.stdout}${r.stderr}${r.error?'\n'+r.error.message+'\n':''}`);console.log(log.at(-1));
 fs.writeFileSync('docs/screenshots/grabon-detail/'+(nativeOnly?'native-':'')+'overlap-validation.txt',log.join('\n'));
 if(r.status!==0)process.exitCode=1;
}
