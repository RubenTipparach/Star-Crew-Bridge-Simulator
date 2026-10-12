"""Revise the authored Grabon assemblies without discarding their painted UVs.

Native CSG owns armor openings. Shared primitives own pipes and impulse hardware.
The original projection remains the reproducible input for each finish build.
"""
import json
import math
from pathlib import Path

import bpy
import bmesh
from mathutils import Vector, Matrix

from hs_kit import Prop, PROP_TO_BLENDER, apply_modifiers, lin, make_materials
from build_ship_exteriors import merge_copies

ROOT = Path(__file__).resolve().parents[2]


def B(v):
    return PROP_TO_BLENDER @ Vector(v)


def key(v):
    return tuple(round(x, 4) for x in v)


def inset(poly, distance):
    """Offset a counterclockwise convex hull footprint inward in metres."""
    out = []
    for i, p in enumerate(poly):
        a, c = Vector(poly[i-1]), Vector(poly[(i+1) % len(poly)])
        b = Vector(p)
        u, v = (b-a).normalized(), (c-b).normalized()
        n, m = Vector((-u.y, u.x)), Vector((-v.y, v.x))
        bisector = n+m
        out.append(tuple(b + bisector * (distance / bisector.dot(n))))
    return out


def texture_material(name, image=None):
    """Unlit exported albedo multiplied by the shared fixed-sun vertex bake."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    m.use_backface_culling = True
    n, links = m.node_tree.nodes, m.node_tree.links
    n.clear()
    out = n.new('ShaderNodeOutputMaterial')
    vc = n.new('ShaderNodeVertexColor')
    vc.layer_name = 'Sun'
    if image:
        im = n.new('ShaderNodeTexImage')
        im.image = image
        mix = n.new('ShaderNodeMixRGB')
        mix.blend_type = 'MULTIPLY'
        mix.inputs[0].default_value = 1
        links.new(im.outputs['Color'], mix.inputs[1])
        links.new(vc.outputs['Color'], mix.inputs[2])
        links.new(mix.outputs[0], out.inputs['Surface'])
    else:
        links.new(vc.outputs['Color'], out.inputs['Surface'])
    return m


def refit(ship, old, armor):
    """Return the revised merged export and facts measured from its assemblies."""
    settings = json.loads((ROOT/'data/ships/grabon-detailed/exterior.json').read_text())
    cfg = settings['ships'][ship['id']]
    make_materials()
    p = Prop(ship['id']+'_refit', 'grabon-detail', 'central impulse and recessed structure')
    tex = bpy.data.images.load(str(ROOT/'assets/textures/exterior_graphite.png'), check_existing=True)
    tex.pack()
    graphite = texture_material('Subhull graphite', tex)
    steel = texture_material('Subhull steel')
    hot = texture_material('Impulse emitters')
    for m in (graphite, steel, hot):
        m['refit_material'] = True
    steel['vertex_tint'] = [.31, .34, .38]
    hot['vertex_tint'] = lin([1.0, .40, .08])
    hot['fullbright'] = True

    # Recover the exact original atlas on each editable source part. Face keys
    # use ship-space coordinates so source transforms cannot change the lookup.
    uv = old.data.uv_layers.active.data
    charts = {}
    for face in old.data.polygons:
        points = [key(old.matrix_world @ old.data.vertices[old.data.loops[i].vertex_index].co) for i in face.loop_indices]
        charts[tuple(sorted(points))] = dict(zip(points, [tuple(uv[i].uv) for i in face.loop_indices]))
    objects = []
    engines = [f for f in ship['fittings'] if f['kind'] == 'engine']
    removed = []
    for src in list(bpy.data.collections['Editable assemblies'].objects):
        points = [PROP_TO_BLENDER.inverted() @ (src.matrix_world @ v.co) for v in src.data.vertices]
        engine_part = 'engine_' in src.name or any(
            all(all(abs(v[k]-f['center_m'][k]) <= f['size_m'][k]/2+.65 for k in range(3)) for v in points)
            for f in engines)
        if cfg['remove_old_engines'] and engine_part:
            removed.append(src.name)
            continue
        ob = src.copy()
        ob.data = src.data.copy()
        p.coll.objects.link(ob)
        ob.hide_set(False)
        ob.hide_render = False
        layer = ob.data.uv_layers.new(name='UVMap')
        ob.data.uv_layers.active=layer
        for face in ob.data.polygons:
            positions = [key(ob.matrix_world @ ob.data.vertices[ob.data.loops[i].vertex_index].co) for i in face.loop_indices]
            chart = charts[tuple(sorted(positions))]
            for i, pos in zip(face.loop_indices, positions):
                layer.data[i].uv = chart[pos]
        ob.data.materials.clear()
        ob.data.materials.append(armor)
        for face in ob.data.polygons:
            face.material_index = 0
        if 'engine_cleat' in src.name:
            cz=sum(v[2] for v in points)/len(points)
            if any(abs(cz-f['center_m'][2]-.25*f['size_m'][2])<.001 for f in engines):
                ob.location.y += .1
        objects.append(ob)

    def assign(ob, mat):
        ob.data.materials.clear()
        ob.data.materials.append(mat)
        for f in ob.data.polygons:
            f.material_index = 0
        return ob

    def box(name, center, size, mat=graphite, bevel=0, keep=True):
        ob = p.box(name, [center[k]-size[k]/2 for k in range(3)], [center[k]+size[k]/2 for k in range(3)])
        assign(ob, mat)
        if bevel:
            mod = ob.modifiers.new('Single edge chamfer', 'BEVEL')
            mod.width, mod.segments = bevel, 1
            apply_modifiers(ob)
        if keep:
            objects.append(ob)
        return ob

    def pipe(name, a, b, radius, mat=steel):
        va, vb = B(a), B(b)
        delta = vb-va
        bpy.ops.mesh.primitive_cylinder_add(vertices=6, radius=radius, depth=delta.length, location=(va+vb)/2)
        ob = bpy.context.object
        ob.name = p.name+'.'+name
        ob.rotation_euler = delta.to_track_quat('Z', 'Y').to_euler()
        assign(ob, mat)
        objects.append(ob)
        return ob

    def elbow(name, points, radius):
        # A single mitered tube, not overlapping capped cylinders at the bend.
        pts=[Vector(v) for v in points]
        directions=[(b-a).normalized() for a,b in zip(pts,pts[1:])]
        normals=[Vector((0,-v.z,v.y)) for v in directions]
        vertices=[]
        for i,point in enumerate(pts):
            normal=normals[0] if i==0 else normals[-1] if i==len(pts)-1 else (normals[i-1]+normals[i])/(1+normals[i-1].dot(normals[i]))
            for j in range(6):
                angle=2*math.pi*j/6
                vertices.append(B(point+Vector((1,0,0))*radius*math.cos(angle)+normal*radius*math.sin(angle)))
        faces=[tuple(range(5,-1,-1)),tuple(range(6*(len(pts)-1),6*len(pts)))]
        for i in range(len(pts)-1):
            for j in range(6):
                k=(j+1)%6
                faces.append((i*6+j,i*6+k,(i+1)*6+k,(i+1)*6+j))
        mesh=bpy.data.meshes.new(name)
        mesh.from_pydata(vertices,[],faces)
        bm=bmesh.new();bm.from_mesh(mesh)
        bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(mesh);bm.free()
        ob=bpy.data.objects.new(p.name+'.'+name,mesh);p.coll.objects.link(ob)
        assign(ob,steel);objects.append(ob)

    bands = []
    for index, zone in enumerate(ship['hull_zones']):
        if zone['id'] == 'neck':
            continue
        ob = next(o for o in objects if o.name.startswith(ship['id']+'.part_'+str(index)+'.'))
        lo, hi = zone['y_m']
        y = (lo+hi)/2
        height, depth = settings['service_belt_height_m'], settings['service_recess_m']
        poly = zone['poly']
        z0, z1 = min(v[1] for v in poly), max(v[1] for v in poly)
        width = max(abs(v[0]) for v in poly)
        cut = box(zone['id']+'_belt_cutter', [0,y,(z0+z1)/2], [width*2+2,height,z1-z0+2], keep=False)
        p.cut(ob, zone['id']+'_continuous_belt', [cut])
        back = p.prism(zone['id']+'_subhull', inset(poly,depth), 'y', y-height/2-.06, y+height/2+.06, 'machinery')
        assign(back, graphite)
        objects.append(back)
        route = inset(poly, depth-.13)
        for i, (a,b) in enumerate(zip(route, route[1:]+route[:1])):
            for dy in (-.17,.17):
                pipe(zone['id']+'_conduit', [a[0],y+dy,a[1]], [b[0],y+dy,b[1]], settings['pipe_radius_m'])
            # One narrow clamp per segment, aligned to the local sidewall.
            mid = (Vector(a)+Vector(b))/2
            clamp = box(zone['id']+'_coupling', [mid.x,y,mid.y], [.25,.58,.16], graphite, .02)
            tangent = Vector(b)-Vector(a)
            pivot=B([mid.x,y,mid.y])
            rotation=Matrix.Rotation(-math.atan2(tangent.y,tangent.x),3,'Z')
            for vertex in clamp.data.vertices:
                vertex.co=pivot+rotation@(vertex.co-pivot)
            if (Vector(b)-Vector(a)).length > 7:
                junction=box(zone['id']+'_junction', [mid.x,y,mid.y], [.85,.43,.23], graphite, .05)
                for vertex in junction.data.vertices:
                    vertex.co=pivot+rotation@(vertex.co-pivot)
        bands.append({'zone':zone['id'],'center_y_m':y,'height_m':height,'recess_m':depth})
        if zone['id'] != cfg['impulse_zone']:
            continue
        # Paired roof troughs return down the aft wall into the continuous belt.
        # Their ends are square and the cutters define roof and wall boundaries.
        end = z0+(z1-z0)*.25
        slot = settings['roof_channel_width_m']
        xrun = max(cfg['impulse_width_m']/2+.72, width*.55)
        for side in (-1,1):
            x = side*xrun
            roof = box('roof_channel_cutter', [x,hi+.1,(z0+end)/2], [slot,.90,end-z0+.12], keep=False)
            wall = box('aft_return_cutter', [x,(y+hi)/2,z0+.12], [slot,hi-y+.06,.78], keep=False)
            p.cut(ob, 'roof_and_aft_return_'+str(side), [roof,wall])
            box('roof_channel_backing', [x,hi-.32,(z0+end)/2], [slot-.06,.07,end-z0], graphite)
            box('aft_return_backing', [x,(y+hi-.4)/2,z0+.47], [slot-.06,hi-.4-y,.07], graphite)
            for dx in (-.33,.33):
                elbow('roof_return_coolant', [[x+dx,y+.05,z0+.20],[x+dx,hi-.19,z0+.20],[x+dx,hi-.19,end-.18]], .09)
                for z in (z0+1.0,end-.65):
                    pipe('roof_coupling', [x+dx,hi-.19,z-.11], [x+dx,hi-.19,z+.11], .13, graphite)
            box('coolant_junction', [x,hi-.22,(z0+end)/2], [.92,.14,.64], graphite, .06)

    # A shallow central impulse cassette clears the hangar roof. The throat is
    # cut from a chamfered ring; emitter cells and vanes are mirrored around x=0.
    zone = next(z for z in ship['hull_zones'] if z['id'] == cfg['impulse_zone'])
    y = sum(zone['y_m'])/2
    z = min(v[1] for v in zone['poly'])-.42
    w, h = cfg['impulse_width_m'], cfg['impulse_height_m']
    frame = box('central_impulse_frame', [0,y,z], [w,h,.88], graphite, .16)
    mouth = box('impulse_aperture_cutter', [0,y,z-.18], [w-.54,h-.48,1.35], keep=False)
    p.cut(frame, 'open_impulse_throat', [mouth])
    box('impulse_throat_back', [0,y,z+.26], [w-.55,h-.49,.06], graphite)
    for side in (-1,1):
        for i in range(8):
            x = side*(.34+i*(w/2-.83)/7)
            box('impulse_emitter', [x,y,z+.10], [.36,h-.75,.08], hot, .035)
        for dy in (-.27,.27):
            box('heat_vane', [side*w*.245,y+dy,z-.22], [w*.43,.055,.25], steel, .015)
    box('impulse_divider', [0,y,z-.09], [.20,h-.52,.56], steel, .035)
    facts = {'impulse_center_m':[0,y,z], 'impulse_size_m':[w,h,.88], 'removed_engine_parts':len(removed), 'service_belts':bands}

    # New equipment uses metre-projected graphite, while surviving armor keeps
    # its authored atlas. Mirror the retained port finish onto the starboard side.
    for ob in objects:
        if not ob.data.uv_layers:
            ob.data.uv_layers.new(name='UVMap')
        uv = ob.data.uv_layers.active.data
        for face in ob.data.polygons:
            if ob.data.materials[face.material_index] == armor:
                continue
            normal = ob.matrix_world.to_3x3() @ face.normal
            axis = max(range(3), key=lambda k:abs(normal[k]))
            axes = [k for k in range(3) if k != axis]
            for i in face.loop_indices:
                v = ob.matrix_world @ ob.data.vertices[ob.data.loops[i].vertex_index].co
                uv[i].uv = [v[k]/2 for k in axes]
    merged = merge_copies(objects, ship['id']+'_revised_hull')
    bpy.context.view_layer.objects.active = merged
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    # Mirror actual faces and their UV loops, including triangles that straddle
    # the keel. Merely matching opposite triangle keys misses different diagonals.
    bm=bmesh.new();bm.from_mesh(merged.data)
    bmesh.ops.bisect_plane(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
                          dist=.00001,plane_co=(0,0,0),plane_no=(1,0,0),
                          clear_outer=True,clear_inner=False,use_snap_center=True)
    facts['mirrored_uv_faces']=len(bm.faces)
    bmesh.ops.mirror(bm,geom=list(bm.verts)+list(bm.edges)+list(bm.faces),
                     matrix=Matrix.Identity(4),merge_dist=.00001,axis='X')
    bmesh.ops.dissolve_degenerate(bm,dist=.00001,edges=list(bm.edges))
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    bm.to_mesh(merged.data);bm.free()
    merged.data.update()
    facts['csg_steps'] = p.steps
    for ob in objects+[old]:
        ob.hide_render = True
        ob.hide_set(True)
    merged.hide_render = False
    merged.hide_set(False)
    return merged, facts
