"""Read back revised Grabon GLBs and inspect the actual editable mesh topology.

Run with Blender headless after build_grabon_detail.py --finish. Browser review
separately checks mirrored UVs, central impulse placement and coplanar overlap.
"""
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
import bmesh

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(Path(__file__).parent))
from hs_kit import read_glb, PROP_TO_BLENDER

manifest=json.loads((ROOT/'assets/models/grabon/props.json').read_text())
report={'status':'pass','ships':[]}
for ship,entry in manifest['props'].items():
    path=ROOT/'assets/models/grabon'/entry['file']
    assert hashlib.sha256(path.read_bytes()).hexdigest()==entry['sha256']
    js,primitives=read_glb(path)
    triangles=sum(len(p['idx'])//3 for p in primitives.values())
    assert triangles==entry['triangles'] and triangles<=entry['budget_triangles']
    assert len(primitives)==4
    for material in js['materials']:
        assert not material.get('doubleSided',False)
        assert 'KHR_materials_unlit' in material.get('extensions',{})
    for primitive in primitives.values():
        assert all(math.isfinite(v) for key in ('pos','nrm','uv') for row in primitive[key] for v in row)
    bpy.ops.wm.open_mainfile(filepath=str(path.with_suffix('.blend')))
    mesh=bpy.data.objects[ship+'_revised_hull'].data
    bm=bmesh.new();bm.from_mesh(mesh)
    nonmanifold=sum(not e.is_manifold for e in bm.edges)
    degenerate=sum(f.calc_area()<1e-10 for f in bm.faces)
    bm.free()
    assert nonmanifold==0,(ship,'nonmanifold edges',nonmanifold)
    assert degenerate==0,(ship,'degenerate faces',degenerate)
    lower_bounds=None
    if ship=='gorath':
        catalogue=json.loads((ROOT/'data/ships/grabon-detailed/layouts.json').read_text())
        design=next(s for s in catalogue['ships'] if s['id']==ship)
        index,zone=next((i,z) for i,z in enumerate(design['hull_zones']) if z['id']=='body')
        armor=next(o for o in bpy.data.objects if o.name.startswith(ship+'.part_'+str(index)+'.'))
        points=[PROP_TO_BLENDER.inverted()@(armor.matrix_world@v.co) for v in armor.data.vertices]
        lower_bounds=[[round(min(v[k] for v in points),5),round(max(v[k] for v in points),5)] for k in (0,2)]
        expected=[[min(v[k] for v in zone['poly']),max(v[k] for v in zone['poly'])] for k in (0,1)]
        assert all(abs(a-b)<.0001 for pair,want in zip(lower_bounds,expected) for a,b in zip(pair,want)),(ship,'native lower hull differs from layout',lower_bounds,expected)
    report['ships'].append({'id':ship,'triangles':triangles,'materials':len(primitives),
                           'nonmanifold_edges':nonmanifold,'degenerate_faces':degenerate,
                           'sha256':entry['sha256']})
    if lower_bounds:report['ships'][-1]['lower_hull_bounds_xz_m']=lower_bounds
out=ROOT/'docs/screenshots/grabon-detail/exterior-validation.json'
out.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
