"""Derive fitted Grabon review layouts and access shafts from approved concepts.

One source owns the silhouettes. This builder reserves real shaft volumes, splits
convex room brushes and relocates obstructed portals and furnishings in metres.
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from layout_check import inside_poly, signed_area
from fit_enemy_concepts import clip
from grabon_windows import fit_windows


def half(poly, axis, value, sign):
    """Clip a convex polygon to one axis-aligned half plane."""
    out=[]
    for a,b in zip(poly,poly[1:]+poly[:1]):
        da=(a[axis]-value)*sign;db=(b[axis]-value)*sign
        if da>=-1e-7:out.append(a)
        if da*db<0:
            t=da/(da-db);out.append([round(a[k]+(b[k]-a[k])*t,6) for k in range(2)])
    clean=[]
    for p in out:
        if not clean or math.dist(clean[-1],p)>1e-6:clean.append(p)
    if len(clean)>1 and math.dist(clean[0],clean[-1])<1e-6:clean.pop()
    return clean if len(clean)>=3 and abs(signed_area(clean))>1e-6 else []


def subtract(poly, rect):
    """Disjoint convex pieces outside a rectangle, without overlapping faces."""
    overlap=poly
    for axis,value,sign in [(0,rect[0],1),(0,rect[2],-1),(1,rect[1],1),(1,rect[3],-1)]:
        overlap=half(overlap,axis,value,sign)
        if not overlap:return [poly]
    inside=poly;out=[]
    for axis,value,sign in [(0,rect[0],1),(0,rect[2],-1),(1,rect[1],1),(1,rect[3],-1)]:
        if not inside:break
        part=half(inside,axis,value,-sign)
        if part:out.append(part)
        inside=half(inside,axis,value,sign)
    return out


def rect_poly(r):
    a,b,c,d=r;return [[a,b],[c,b],[c,d],[a,d]]


def build():
    data=json.loads((ROOT/'data/ships/grabon-concepts/layouts.json').read_text())
    exterior=json.loads((ROOT/'data/ships/grabon-detailed/exterior.json').read_text())
    interior=json.loads((ROOT/'data/ships/grabon-detailed/interior.json').read_text())
    data['collection']='Grabon detailed ships';data['detail_level']='detailed'
    for s in data['ships']:
        L=s['layout'];decks=sorted(L['decks'],key=lambda d:d['floor_y_m'])
        floors=[d['floor_y_m'] for d in decks];top=floors[-1]+2.8
        lower=exterior['ships'][s['id']].get('lower_hull')
        if lower:
            zone=next(z for z in s['hull_zones'] if z['id']==lower['zone'])
            bay=next(f for f in s['fittings'] if f['kind']=='baydoor')
            # Retain the original bounds for identifying the native painted parts.
            s['lower_hull_refit']={'source_zone':copy.deepcopy(zone),'source_bay':copy.deepcopy(bay)}
            sx,sz=lower['scale_xz'];dz=lower['offset_z_m']
            zone['poly']=[[round(x*sx,6),round(z*sz+dz,6)] for x,z in zone['poly']]
            s['fit_envelopes'][zone['id']]=copy.deepcopy(zone['poly'])
            bay['center_m'][0]=round(bay['center_m'][0]*sx,6);bay['center_m'][2]=round(bay['center_m'][2]*sz+dz,6)
            bay['size_m'][0]=round(bay['size_m'][0]*sx,6);bay['size_m'][2]=round(bay['size_m'][2]*sz,6)
            for room in L['compartments']:
                if room['envelope']==zone['id']:
                    for brush in room['brushes']:
                        brush['poly']=clip(brush['poly'],zone['poly'],s['fit_margin_m'])
                        assert len(brush['poly'])>=3,(s['id'],room['id'],'lower hull erased room')
            assert len(s['craft'])==len(lower['fighter_centers_xz_m'])
            for craft,(x,z) in zip(s['craft'],lower['fighter_centers_xz_m']):
                craft['center_m'][0]=x;craft['center_m'][2]=z
        # Meet the existing bulkhead at z=-2, avoiding inaccessible slivers of
        # the adjacent reactor and life-support rooms behind their wall ribs.
        shafts=[('stairs',[-5.7,-6.3,-1.2,-2.0]),('lift',[1.2,-2.0,4.4,1.0])]
        for r in L['compartments']:
            if r['kind']=='corridor':continue
            for _,rect in shafts:
                r['brushes']=[{'poly':p,'y':b['y']} for b in r['brushes'] for p in subtract(b['poly'],rect)]
            assert r['brushes'],r['id']
        for id,rect in shafts:
            r={'id':id,'name':'Stair tower' if id=='stairs' else 'Elevator shaft','kind':'trunk','group':'route',
               'purpose':'All-deck access with a clear landing at every corridor.','decks':[d['id'] for d in decks],
               'poi':len(L['compartments'])+1,'envelope':'access','finish':'working',
               'brushes':[{'poly':rect_poly(rect),'y':[0,top]}]}
            r['initial_brushes']=copy.deepcopy(r['brushes']);L['compartments'].append(r)
            x=-1.2 if id=='stairs' else 1.2;z=-3.2 if id=='stairs' else -.4
            for d in decks:
                L['portals'].append({'id':id+'_'+d['id'],'kind':'door','between':[d['id']+'_route',id],
                  'center_m':[x,d['floor_y_m']+1.05,z],'normal':[-1 if x<0 else 1,0,0],'size_m':[1.3,2.1]})
        rooms={r['id']:r for r in L['compartments']}
        # Existing room doors whose wall was consumed by a shaft move to a surviving
        # corridor boundary. Keep a 15 cm jamb at each end of the candidate segment.
        for p in L['portals']:
            if abs(p['normal'][0])<.9 or p['id'].startswith(('stairs_','lift_')):continue
            a,b=[rooms[id] for id in p['between']]
            if a['kind']!='corridor':continue
            x=p['center_m'][0];ranges=[]
            for br in b['brushes']:
                for v,w in zip(br['poly'],br['poly'][1:]+br['poly'][:1]):
                    if abs(v[0]-x)<1e-5 and abs(w[0]-x)<1e-5:
                        lo,hi=sorted([v[1],w[1]])
                        if hi-lo>=p['size_m'][0]+.3:ranges.append((lo,hi))
            assert ranges,(s['id'],p['id'])
            if not any(lo+.15<=p['center_m'][2]-p['size_m'][0]/2 and p['center_m'][2]+p['size_m'][0]/2<=hi-.15 for lo,hi in ranges):
                lo,hi=max(ranges,key=lambda q:q[1]-q[0]);p['center_m'][2]=(lo+hi)/2
        stair={'id':'main_stair','kind':'spiral_stair','compartment':'stairs','center_m':[-3.7,0,-4.2],
          'floors_y_m':floors,'radius_m':1.7,'column_radius_m':.12,'risers_per_deck':20,
          'start_yaw_deg':90,'sweep_deg':270,'landing_poly':rect_poly([-3.7,-4.2,-1.2,-2.1])}
        lift={'id':'main_lift','kind':'lift','compartment':'lift','center_m':[2.8,0,-.4],
          'car_m':[2.6,2.2,2.3],'floors_y_m':floors,'facing_yaw_deg':-90,'roof':False,
          'panels':{'wall':'panel:working:plate','back':'panel:working:plate','floor':'panel:working:floor:plate'}}
        L['fixtures']=[stair,lift]
        for portal in L['portals']:
            if portal['kind']=='ladder':
                portal['center_m'][0]=-.55;portal['center_m'][2]=.9;portal['size_m']=[.9,1.2]
                # Center the aperture in the slab so both adjoining room shells
                # recognize its opening, including these 0.8 m deck structures.
                a,b=(rooms[id] for id in portal['between'])
                lower_room,upper_room=sorted([a,b],key=lambda r:r['brushes'][0]['y'][0])
                portal['center_m'][1]=(max(br['y'][1] for br in lower_room['brushes'])+min(br['y'][0] for br in upper_room['brushes']))/2
        s['access']={'stair':stair,'lift':lift,'shaft_rects':dict(shafts)}
        fit_windows(s,interior)
        # Refit scale furniture into the remaining room union. Keep berth and crew
        # counts; reject a room when no placement exists rather than drop its prop.
        placed={}
        for f in s['furnishings']:
            if f['kind']=='seat':f['size_m']=[.70,1.22,.64]
            if f['kind']=='bunk':f['size_m']=[1.0,2.0,2.2]
            r=rooms[f['room']];w,h,l=f['size_m'];x,y,z=f['center_m']
            def fits(xx,zz):
                if not all(any(inside_poly(b['poly'],xx+dx,zz+dz) for b in r['brushes']) for dx in [-w/2-.08,w/2+.08] for dz in [-l/2-.08,l/2+.08]):return False
                for qx,qz,qw,ql in placed.get(r['id'],[]):
                    if abs(xx-qx)<(w+qw)/2+.12 and abs(zz-qz)<(l+ql)/2+.12:return False
                for p in L['portals']:
                    if r['id'] in p['between'] and abs(p['normal'][1])<.5 and abs(xx-p['center_m'][0])<w/2+.8 and abs(zz-p['center_m'][2])<l/2+.8:return False
                return True
            if not fits(x,z):
                points=[v for b in r['brushes'] for v in b['poly']]
                candidates=[(i*.3,j*.3) for i in range(math.ceil(min(v[0] for v in points)/.3),math.floor(max(v[0] for v in points)/.3)+1)
                            for j in range(math.ceil(min(v[1] for v in points)/.3),math.floor(max(v[1] for v in points)/.3)+1)]
                candidates.sort(key=lambda p:(p[0]-x)**2+(p[1]-z)**2)
                hit=next((p for p in candidates if fits(*p)),None);assert hit,(s['id'],r['id'],f)
                f['center_m']=[hit[0],y,hit[1]];x,z=hit
            placed.setdefault(r['id'],[]).append((x,z,w,l))
        s['notes']=['Purple armor and orange identification bands.',
                    'Stairs, elevator and independent ladder connect every main deck.',
                    'Two fighter berths and the approved hull proportions are retained.']
        s['adjustments']=['Dedicated shaft volumes are cut out of adjacent rooms.',
                          'Relocated doors preserve clear corridor approaches.',
                          'Furnishings are refitted around the access towers.']
        if lower:
            s['notes'][-1]='Two full-size fighter berths fit inside the inset bottom deck.'
            s['adjustments'].append('The bottom hull is narrower and shorter than the engineering deck; its rooms and hangar are refitted.')
    return data


if __name__=='__main__':
    out=ROOT/'data/ships/grabon-detailed/layouts.json';out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(build(),indent=2)+'\n')
    print('Derived three detailed Grabon layouts with stairs, elevators and ladders.')
