"""Measure the delivered concept layouts, independent of the fitting operation.

Checks room hull containment, positive convex room polygons, overlap, real portal
alignment, connected routes, fighter and furniture fit. Reports per-room areas
and volumes. This is a documentation instrument, not a runtime layout compiler.
"""
import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from layout_check import inside_poly, is_convex, polys_overlap, signed_area


def distance(p, a, b):
    dx, dz = b[0]-a[0], b[1]-a[1]
    t = max(0, min(1, ((p[0]-a[0])*dx+(p[1]-a[1])*dz)/(dx*dx+dz*dz)))
    return math.dist(p, [a[0]+t*dx, a[1]+t*dz])


def inside_outline(poly, p):
    """Odd-crossing containment for the Splitjaw's concave outer hull."""
    x, z = p
    odd = False
    for i, a in enumerate(poly):
        b = poly[(i+1) % len(poly)]
        if (a[1] > z) != (b[1] > z) and x < (b[0]-a[0])*(z-a[1])/(b[1]-a[1])+a[0]:
            odd = not odd
    return odd


def check(data):
    results = []
    assert len(data['ships']) == 3, 'Three study sets required'
    required = {'bridge','magazine','port_laser','stbd_laser','reactor','drive'}
    for s in data['ships']:
        L = s['layout']; rooms = {r['id']:r for r in L['compartments']}
        assert required <= rooms.keys(), f"{s['id']}: a Hound combat room is missing"
        assert len(rooms) == len(L['compartments']), 'Duplicate room ids'
        report = {'ship':s['id'],'rooms':[],'changed_rooms':0,'minimum_hull_clearance_m':100,'crew_routes':True,'fighter_berths':len(s['craft'])}
        for r in rooms.values():
            b = r['brushes'][0];p=b['poly'];y0,y1=b['y']
            assert signed_area(p)>0 and is_convex(p), f"{s['id']}/{r['id']}: invalid room polygon"
            assert y1-y0 >= 2.8-1e-6, f"{r['id']}: insufficient headroom"
            zones = [z for z in s['hull_zones'] if z['deck']==r['decks'][0]]
            hull = next((z for z in zones if z['id']==r['envelope']), zones[0])
            # Corners alone cannot prove that a room avoids a concave hull notch.
            # Sample every room edge at 0.25 m intervals as well.
            samples=[]
            for i,a in enumerate(p):
                b2=p[(i+1)%len(p)];n=math.ceil(math.dist(a,b2)/.25)
                samples.extend([[a[k]+(b2[k]-a[k])*j/n for k in range(2)] for j in range(n+1)])
            margin=min(y0-hull['y_m'][0],hull['y_m'][1]-y1)
            for v in samples:
                assert inside_outline(hull['poly'],v), f"{s['id']}/{r['id']}: outside hull at {v}"
                margin=min(margin,min(distance(v,a,hull['poly'][(i+1)%len(hull['poly'])]) for i,a in enumerate(hull['poly'])))
            assert margin>=.45-1e-5, f"{s['id']}/{r['id']}: only {margin:.3f} m hull clearance"
            report['minimum_hull_clearance_m']=min(report['minimum_hull_clearance_m'],margin)
            changed=r['brushes']!=r['initial_brushes'];report['changed_rooms']+=changed
            report['rooms'].append({'id':r['id'],'deck':r['decks'][0],'area_m2':round(signed_area(p),3),'volume_m3':round(signed_area(p)*(y1-y0),3),'changed':changed})
        rv=list(rooms.values())
        for i,r in enumerate(rv):
            for q in rv[i+1:]:
                a,b=r['brushes'][0],q['brushes'][0]
                if min(a['y'][1],b['y'][1])-max(a['y'][0],b['y'][0])>1e-5:
                    assert not polys_overlap(a['poly'],b['poly']), f"{s['id']}: overlap {r['id']} / {q['id']}"
        graph={id:set() for id in rooms}
        for p in L['portals']:
            a,b=(rooms[id] for id in p['between']);x,y,z=p['center_m'];n=p['normal']
            graph[a['id']].add(b['id']);graph[b['id']].add(a['id'])
            if abs(n[1])>.5:
                for r in [a,b]:
                    poly=r['brushes'][0]['poly']
                    for dx in [-p['size_m'][0]/2,p['size_m'][0]/2]:
                        for dz in [-p['size_m'][1]/2,p['size_m'][1]/2]:
                            assert inside_poly(poly,x+dx,z+dz), f"{p['id']}: ladder footprint leaves {r['id']}"
                assert abs(a['brushes'][0]['y'][1]-y)<.001, f"{p['id']}: ladder misses lower ceiling"
                assert 0<=b['brushes'][0]['y'][0]-y<=L['conventions']['deck_slab_m']+.001
            else:
                w,h=p['size_m'];t=[n[2],-n[0]]
                for side,r in enumerate([a,b]):
                    br=r['brushes'][0];poly=br['poly']
                    assert br['y'][0]-.001<=y-h/2 and y+h/2<=br['y'][1]+.001
                    edges=[(v,poly[(i+1)%len(poly)]) for i,v in enumerate(poly)]
                    shared=[(u,v2) for u,v2 in edges if all(distance([x+t[0]*v,z+t[1]*v],u,v2)<.001 for v in [-w/2,w/2])]
                    assert shared, f"{s['id']}/{p['id']}: doorway does not fit a shared wall"
                    u,v=shared[0];dx,dz=v[0]-u[0],v[1]-u[1];length=math.hypot(dx,dz)
                    assert (dz*n[0]-dx*n[2])/length*(1 if side==0 else -1)>.999, f"{p['id']}: portal normal faces the wrong way"
        visited={'bridge'};todo=['bridge']
        while todo:
            for id in graph[todo.pop()]-visited:
                visited.add(id);todo.append(id)
        assert visited==set(rooms), f"{s['id']}: unreachable {set(rooms)-visited}"
        assert report['changed_rooms']>=4, f"{s['id']}: no meaningful fitting pass"
        for f in s['furnishings']+s['craft']:
            br=rooms[f['room']]['brushes'][0];x,y,z=f['center_m'];w,h,l=f['size_m']
            for dx in [-w/2,w/2]:
                for dz in [-l/2,l/2]:
                    assert inside_poly(br['poly'],x+dx,z+dz),f"{s['id']}/{f.get('kind',f.get('id'))}: scale object leaves {f['room']}"
            assert y>=br['y'][0]-.001 and y+h<=br['y'][1]+.001, f"{f}: height does not fit"
        assert len(s['craft'])==2 and all(f['size_m']==[8,2.4,8] for f in s['craft'])
        report['area_m2']=round(sum(r['area_m2'] for r in report['rooms']),3)
        results.append(report)
    return {'base_commit':data['base_commit'],'status':'pass','studies':results}


if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--out',type=Path)
    ap.add_argument('--faction', choices=['grabon','federation'], default='grabon');args=ap.parse_args()
    report=check(json.loads((ROOT/f'data/ships/{args.faction}-concepts/layouts.json').read_text()))
    if args.out:args.out.parent.mkdir(parents=True,exist_ok=True);args.out.write_text(json.dumps(report,indent=2)+'\n')
    for s in report['studies']:
        print(f"{s['ship']}: {len(s['rooms'])} connected rooms, {s['changed_rooms']} fitted, {s['area_m2']} m2, {s['minimum_hull_clearance_m']:.2f} m minimum hull clearance, two fighters fit")
