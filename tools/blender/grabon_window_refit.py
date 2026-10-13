"""Open Grabon pressure shells and windows from the actual interior portals.

Native CSG cuts the armor, machinery backing and any pipes in the aperture.
The manifest records measured surface intersections for the shared window liner
and browser ray validation used by the Tern preview.
"""
import json
from pathlib import Path
import bpy
from mathutils import Vector
from hs_kit import PROP_TO_BLENDER
from build_machinery_props import facet

ROOT=Path(__file__).resolve().parents[2]


def window_frame(portal):
    return facet(Vector(portal['center_m']),Vector(portal['normal']),(0,1,0))


def reserve_armor(p,cutter,zone,ship,assign,graphite):
    """Leave an armored surround where a continuous belt meets a window."""
    keep=[]
    for w in ship.get('windows',[]):
        if w['zone']!=zone['id']:continue
        width,height=w['size_m']
        ob=p.box(w['id']+'_armor_keepout',(-width/2-.22,-height/2-.22,-.1),
                 (width/2+.22,height/2+.22,w['max_depth_m']+2),m=window_frame(w))
        assign(ob,graphite);keep.append(ob)
    if keep:p.cut(cutter,zone['id']+'_window_armor_bridges',keep)


def open_windows(ship,p,objects,assign,graphite,steel,inset):
    cfg=json.loads((ROOT/'data/ships/grabon-detailed/interior.json').read_text())
    B=lambda v:PROP_TO_BLENDER@Vector(v)
    inverse=PROP_TO_BLENDER.inverted()
    windows=[]
    def join(label,parts):
        # Sequential unions also resolve overlaps between operands. A single
        # collection Boolean can leave those intersections inside its operand.
        target=parts[0]
        for i,ob in enumerate(parts[1:]):p.union(target,label+'_'+str(i),[ob])
        return target
    # The backing and armor enclose one shared air volume. Its inner surface
    # stays outside the room walls, with explicit floor/ceiling clearance.
    shells=[ob for ob in objects if '.part_' in ob.name and any(ob.name.startswith(ship['id']+'.part_'+str(i)+'.') for i in range(len(ship['hull_zones']))) or '_subhull' in ob.name]
    # Overlapping hull tiers must become one pressure skin before hollowing.
    # Otherwise each tier leaves coincident cavity caps inside its neighbor.
    copies=[]
    for shell in shells:
        ob=shell.copy();ob.data=shell.data.copy();p.coll.objects.link(ob)
        copies.append(ob);shell.hide_render=True;shell.hide_set(True)
    pressure=copies[0];pressure.name=ship['id']+'_joined_pressure_skin'
    join('unified_pressure_skin',copies)
    objects[:]=[ob for ob in objects if ob not in shells]+[pressure]
    cavities=[]
    for zone in ship['hull_zones']:
        rooms=[r for r in ship['layout']['compartments'] if r['envelope']==zone['id']]
        if not rooms:continue
        lo=min(b['y'][0] for r in rooms for b in r['brushes'])-.06
        hi=max(b['y'][1] for r in rooms for b in r['brushes'])+.06
        cavity=p.prism(zone['id']+'_pressure_volume',inset(zone['poly'],.26+cfg['pressure_skin_m']), 'y',lo,hi,'machinery')
        assign(cavity,graphite);cavities.append(cavity)
    air=join('connected_room_air_volumes',cavities)
    p.cut(pressure,'hollow_pressure_shell',[air])
    shells=[pressure]
    for w in ship.get('windows',[]):
        armor=pressure
        c=Vector(w['center_m']);n=Vector(w['normal']).normalized();t=Vector((n.z,0,-n.x))
        width,height=w['size_m'];rim=cfg['window_rim_m'];outer=[];inner=[]
        def cast(origin,direction):
            local=armor.matrix_world.inverted()
            hit,point,normal,_=armor.ray_cast(local@B(origin),local.to_3x3()@B(direction))
            assert hit,(ship['id'],w['id'],'ray missed pressure shell',tuple(origin))
            return inverse@(armor.matrix_world@point)
        center=cast(c+n*8,-n);depth=(center-c).dot(n)
        assert 0<depth<=w['max_depth_m'],(w['id'],'depth',depth)
        for u,v in [(-1,-1),(1,-1),(1,1),(-1,1)]:
            q=c+t*u*width/2+Vector((0,v*height/2,0))
            outer.append(list(cast(q+n*8,-n)))
            inner.append(list(cast(q,n)))
        frame_outer=[cast(c+t*u*(width/2+rim)+Vector((0,v*(height/2+rim),0))+n*8,-n)
                     for u,v in [(-1,-1),(1,-1),(1,1),(-1,1)]]
        # Cut every intersecting assembly, including pipe runs and sensor strips.
        frame=window_frame(w);rot=frame.inverted()
        targets=[]
        for ob in objects:
            pts=[rot@(inverse@(ob.matrix_world@v.co)) for v in ob.data.vertices]
            if not pts:continue
            pad=0 if ob in shells else rim+.04
            low=(-width/2-pad,-height/2-pad,-.08);high=(width/2+pad,height/2+pad,depth+cfg['window_service_clearance_m'])
            if all(max(v[k] for v in pts)>low[k]+.0001 and min(v[k] for v in pts)<high[k]-.0001 for k in range(3)):
                targets.append(ob)
        # A pipe, its collar and a junction can overlap in solid space. Join
        # that hardware before cutting it, so the aperture has one end cap.
        service=[ob for ob in targets if ob not in shells]
        if len(service)>1:
            join(w['id']+'_joined_service_hardware',service)
            objects[:]=[ob for ob in objects if ob not in service[1:]]
            targets=[ob for ob in targets if ob not in service[1:]]
        for ob in targets:
            pad=0 if ob in shells else rim+.04
            cut=p.box(w['id']+'_through_cut',(-width/2-pad,-height/2-pad,-.08),(width/2+pad,height/2+pad,depth+cfg['window_service_clearance_m']),'machinery',frame)
            assign(cut,graphite);p.cut(ob,w['id'],[cut])
        # Follow the actual hull at all eight corners. A room's flat window
        # plane can meet a tapered hull, so a constant-depth rim intersects
        # its jamb. This closed ring stays 15 mm clear of the measured skin.
        points=[B(Vector(q)+n*d) for d in (.015,.085) for q in frame_outer+outer]
        faces=[]
        for i in range(4):
            j=(i+1)%4
            faces.extend([(8+i,8+j,12+j,12+i),(i,4+i,4+j,j),
                          (i,j,8+j,8+i),(4+i,12+i,12+j,4+j)])
        mesh=bpy.data.meshes.new(w['id']+'_rim');mesh.from_pydata(points,[],faces);mesh.update()
        ring=bpy.data.objects.new(w['id']+'_rim',mesh);p.coll.objects.link(ring)
        assign(ring,steel);objects.append(ring)
        windows.append({'id':w['id'],'room':w['between'][0],'center_m':w['center_m'],'normal':w['normal'],
                        'size_m':w['size_m'],'outer_center_m':list(center),'outer_corners_m':outer,'inner_corners_m':inner,
                        'rim_width_m':rim,'max_depth_m':w['max_depth_m'],'depth_m':depth,
                        'rim_material':steel.name,'deck':w['deck']})
    # Difference can consume an old pipe or panel completely.
    objects[:]=[ob for ob in objects if len(ob.data.polygons)]
    return windows
