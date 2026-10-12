"""Fit the Grabon bridge walls and derive mirrored windows from real room edges.

Window centers, normals and sizes are the portal source for both the room builder
and the exterior CSG. Inward walls and long hull-to-room passages are rejected.
"""
import math
from fit_enemy_concepts import clip


def ray_depth(point,normal,poly):
    hits=[]
    for a,b in zip(poly,poly[1:]+poly[:1]):
        edge=[b[k]-a[k] for k in (0,1)]
        det=normal[0]*edge[1]-normal[1]*edge[0]
        if abs(det)<1e-8:continue
        delta=[a[k]-point[k] for k in (0,1)]
        t=(delta[0]*edge[1]-delta[1]*edge[0])/det
        u=(delta[0]*normal[1]-delta[1]*normal[0])/det
        if t>=0 and -.0001<=u<=1.0001:hits.append(t)
    return min(hits) if hits else math.inf


def fit_windows(ship,cfg):
    layout=ship['layout'];rooms={r['id']:r for r in layout['compartments']}
    zones={z['id']:z for z in ship['hull_zones']}
    bridge=rooms['bridge'];aft=min(v[1] for b in bridge['brushes'] for v in b['poly'])
    bridge['brushes'][0]['poly']=clip([[-100,aft],[100,aft],[100,100],[-100,100]],zones['head']['poly'],cfg['bridge_wall_margin_m'])
    windows=[]
    def fitting_blocks(point,normal,y,depth):
        """Keep observation ports clear of the nearby weapon/engine assemblies."""
        for f in ship['fittings']:
            if f['kind'] not in ('laser','engine'):continue
            angle=math.radians(f.get('yaw_deg',0));co,si=math.cos(angle),math.sin(angle)
            dx,dz=point[0]-f['center_m'][0],point[1]-f['center_m'][2]
            origin=[dx*co-dz*si,y-f['center_m'][1],dx*si+dz*co]
            direction=[normal[0]*co-normal[1]*si,0,normal[0]*si+normal[1]*co]
            lo,hi=0,depth+4
            for a,n,size in zip(origin,direction,f['size_m']):
                extent=size/2+.15
                if abs(n)<1e-9:
                    if abs(a)>extent:hi=-1;break
                else:
                    t0,t1=sorted(((-extent-a)/n,(extent-a)/n));lo=max(lo,t0);hi=min(hi,t1)
            if hi>=lo:return True
        return False
    def add(room,point,normal,label,width):
        zone=zones[room['envelope']]
        depth=ray_depth(point,normal,zone['poly'])
        assert depth<=cfg['window_max_depth_m'],(ship['id'],label,'deep window',depth)
        center=[round(point[0],6),round(room['brushes'][0]['y'][0]+cfg['window_center_above_floor_m'],6),round(point[1],6)]
        w={'id':'window_'+label,'kind':'window','between':[room['id'],'space'],
           'center_m':center,'normal':[round(normal[0],9),0,round(normal[1],9)],
           'size_m':[width,cfg['window_height_m']],'max_depth_m':cfg['window_max_depth_m']}
        layout['portals'].append(w)
        windows.append(dict(w,zone=zone['id'],deck=room['decks'][0]))
    def candidates(room,side):
        out=[]
        for brush in room['brushes']:
            for a,b in zip(brush['poly'],brush['poly'][1:]+brush['poly'][:1]):
                dx,dz=b[0]-a[0],b[1]-a[1];length=math.hypot(dx,dz)
                n=[dz/length,-dx/length]
                if n[0]*side<.65 or length<cfg['window_width_m']+.5:continue
                center=[(a[k]+b[k])/2 for k in (0,1)]
                depths=[ray_depth([center[k]+f*(b[k]-a[k]) for k in (0,1)],n,zones[room['envelope']]['poly']) for f in [-.35,0,.35]]
                if max(depths)<=cfg['window_max_depth_m']:out.append((length,a,b,n))
        return sorted(out,reverse=True)
    # Each bridge has two forward observation slits and paired side windows.
    bow=max(bridge['brushes'][0]['poly'],key=lambda v:v[1])[1]
    bow_half=min(abs(v[0]) for v in bridge['brushes'][0]['poly'] if abs(v[1]-bow)<.001)
    for side in (-1,1):add(bridge,[side*min(2.0,bow_half-1.045),bow],[0,1],'bridge_bow_'+str(side),1.65)
    pairs=[['bridge','bridge']]+cfg['window_rooms'][ship['id']]
    for i,(left,right) in enumerate(pairs):
        lc=candidates(rooms[left],-1);rc=candidates(rooms[right],1)
        assert lc and rc,(ship['id'],left,right,'no exterior window wall')
        _,a,b,n=lc[0]
        # Use mirrored wall samples, rather than separately spacing the two sides.
        length=math.dist(a,b);edge=[(b[k]-a[k])/length for k in (0,1)]
        margin=cfg['window_width_m']/2+.25
        samples=[margin+(length-2*margin)*j/20 for j in range(21)]
        valid=[]
        for distance in samples:
            point=[a[k]+distance*edge[k] for k in (0,1)]
            samples_xz=[[point[k]+f*edge[k] for k in (0,1)] for f in [-margin,0,margin]]
            y=rooms[left]['brushes'][0]['y'][0]+cfg['window_center_above_floor_m']
            depths=[ray_depth(q,n,zones[rooms[left]['envelope']]['poly']) for q in samples_xz]
            if max(depths)<=cfg['window_max_depth_m'] and not any(fitting_blocks(q,n,y+dy,depth) for q,depth in zip(samples_xz,depths) for dy in [-cfg['window_height_m']/2,0,cfg['window_height_m']/2]):valid.append((distance,point))
        assert valid,(ship['id'],left,'window width does not fit')
        positions=[min(valid,key=lambda q:abs(q[0]-length*.35))]
        other=[q for q in valid if abs(q[0]-positions[0][0])>=cfg['window_width_m']+.5]
        if other:positions.append(min(other,key=lambda q:abs(q[0]-length*.65)))
        for j,(_,point) in enumerate(positions):
            add(rooms[left],point,n,f'{i}_{j}_port',cfg['window_width_m'])
            add(rooms[right],[-point[0],point[1]],[-n[0],n[1]],f'{i}_{j}_starboard',cfg['window_width_m'])
    ship['windows']=windows
