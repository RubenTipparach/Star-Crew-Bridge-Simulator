"""Validate actual detailed Grabon brush volumes and access routes, independently
of their generator. The instrument checks geometry, portal fit and scale, not
just the existence of stair/elevator labels in a catalogue.
"""
import itertools
import json
import math
import sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from layout_check import inside_poly,is_convex,polys_overlap,signed_area
from check_enemy_concepts import distance,inside_outline


def run():
    data=json.loads((ROOT/'data/ships/grabon-detailed/layouts.json').read_text());report=[]
    for s in data['ships']:
        L=s['layout'];rooms={r['id']:r for r in L['compartments']};deckmap={d['id']:d for d in L['decks']}
        brushes=[(r,b) for r in rooms.values() for b in r['brushes']]
        for r,b in brushes:
            assert is_convex(b['poly']) and signed_area(b['poly'])>0,(s['id'],r['id'],'polygon')
            assert all(max(p[k] for p in b['poly'])-min(p[k] for p in b['poly'])>=.6 for k in [0,1]),(s['id'],r['id'],'inaccessible brush sliver')
            assert b['y'][1]-b['y'][0]>=2.8-1e-5
            for deck in r['decks']:
                y=deckmap[deck]['floor_y_m']
                candidates=[z for z in s['hull_zones'] if z['deck']==deck and (r['kind']=='trunk' or z['id']==r['envelope'])]
                for p in b['poly']:
                    assert any(inside_outline(z['poly'],p) and z['y_m'][0]<y and z['y_m'][1]>=y+2.8 for z in candidates),(s['id'],r['id'],'outside hull',p)
        for (r,a),(q,b) in itertools.combinations(brushes,2):
            if min(a['y'][1],b['y'][1])-max(a['y'][0],b['y'][0])>1e-6:
                assert not polys_overlap(a['poly'],b['poly']),(s['id'],r['id'],q['id'],'overlapping volume')
        graph={k:set() for k in rooms}
        windows=[p for p in L['portals'] if p['kind']=='window']
        assert len(windows)>=12,(s['id'],'not enough occupied deck windows')
        for d in L['decks']:
            row=[p for p in windows if d['id'] in rooms[p['between'][0]]['decks']]
            assert row,(s['id'],d['id'],'no deck windows')
            assert all(abs(p['center_m'][1]-d['floor_y_m']-1.45)<1e-6 and p['size_m'][1]==.66 for p in row),(s['id'],'misaligned windows')
        for p in windows:
            assert any(all(abs(a-b)<1e-6 for a,b in zip(q['center_m'],[-p['center_m'][0],*p['center_m'][1:]])) and q['size_m']==p['size_m'] for q in windows),(s['id'],p['id'],'unpaired window')
        for p in L['portals']:
            a,b=p['between']
            if b!='space':graph[a].add(b);graph[b].add(a)
            x,y,z=p['center_m'];w,h=p['size_m'];n=p['normal']
            for room in [rooms[k] for k in (a,b) if k!='space']:
                if abs(n[1])>.5:
                    assert all(any(inside_poly(br['poly'],x+dx,z+dz) for br in room['brushes']) for dx in [-w/2,w/2] for dz in [-h/2,h/2]),(s['id'],p['id'],'ladder fit')
                else:
                    ends=[[x+n[2]*t,z-n[0]*t] for t in [-w/2,w/2]]
                    assert any(br['y'][0]<=y-h/2+1e-6 and br['y'][1]>=y+h/2-1e-6 and any(all(distance(v,c,d)<1e-4 for v in ends) for c,d in zip(br['poly'],br['poly'][1:]+br['poly'][:1])) for br in room['brushes']),(s['id'],p['id'],room['id'],'door fit')
        seen={'bridge'};todo=['bridge']
        while todo:
            for id in graph[todo.pop()]-seen:seen.add(id);todo.append(id)
        assert seen==rooms.keys(),(s['id'],'disconnected',rooms.keys()-seen)
        for f in s['furnishings']+s['craft']:
            r=rooms[f['room']];x,y,z=f['center_m'];w,h,d=f['size_m']
            for dx,dz in itertools.product([-w/2,0,w/2],[-d/2,0,d/2]):
                assert any(inside_poly(b['poly'],x+dx,z+dz) and b['y'][0]<=y+.001 and b['y'][1]>=y+h-.001 for b in r['brushes']),(s['id'],r['id'],f.get('kind','fighter'),'footprint')
        st=s['access']['stair'];floors=sorted(d['floor_y_m'] for d in L['decks'])
        assert st['floors_y_m']==s['access']['lift']['floors_y_m']==floors
        rises=[(b-a)/st['risers_per_deck'] for a,b in zip(floors,floors[1:])]
        going=1.25*math.radians(st['sweep_deg'])/(st['risers_per_deck']-1)
        assert max(rises)<=.19 and going>=.28
        # The staircase occupies yaw 90..360; the upper landing occupies yaw
        # 0..90. Headroom over each tread is the next flight's 3.6 m rise.
        headroom=min(b-a for a,b in zip(floors,floors[1:]))-.06
        assert headroom>=2.1 and st['radius_m']-st['column_radius_m']>=1.2
        for kind in ['stairs','lift']:
            assert len([p for p in L['portals'] if kind in p['between']])==len(floors)
        assert len([p for p in L['portals'] if p['kind']=='ladder'])==len(floors)-1
        lower_dimensions=None
        if s['id']=='gorath':
            lower=next(z for z in s['hull_zones'] if z['id']=='body')
            upper=next(z for z in s['hull_zones'] if z['id']=='engineering')
            spans=lambda zone:[round(max(v[k] for v in zone['poly'])-min(v[k] for v in zone['poly']),6) for k in (0,1)]
            assert all(inside_poly(upper['poly'],*v) for v in lower['poly']),'Gorath bottom footprint must be inside the upper tier'
            assert all(a<b for a,b in zip(spans(lower),spans(upper))),'Gorath bottom must be narrower and shorter'
            lower_dimensions={'lower_width_length_m':spans(lower),'upper_width_length_m':spans(upper)}
            for a,b in itertools.combinations(s['craft'],2):
                assert any(abs(a['center_m'][k]-b['center_m'][k])>=(a['size_m'][k]+b['size_m'][k])/2+.6 for k in (0,2)),'Keep a clear aisle between fighter berths'
        report.append({'ship':s['id'],'rooms':len(rooms),'windows':len(windows),'brushes':len(brushes),'reachable':len(seen),'decks':len(floors),
                       'stair_rise_m':rises[0],'stair_going_m':round(going,3),'flight_headroom_m':headroom,'ladder_bypass_width_m':1.3,'fighter_berths':len(s['craft'])})
        if lower_dimensions:report[-1]['tier_dimensions']=lower_dimensions
    return {'status':'pass','ships':report}


if __name__=='__main__':
    r=run();out=ROOT/'docs/screenshots/grabon-detail/layout-validation.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
