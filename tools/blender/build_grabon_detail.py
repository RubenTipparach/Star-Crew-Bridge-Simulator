"""Build detailed Grabon hull assets from the shared viewer's exported geometry.

--guide authors physical fittings and hull-space linework, unwraps and bakes locked
UV guides. --finish applies the enhanced atlas and exports editable sources and
GLBs. No rough silhouette is reimplemented here.
"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
import bmesh
from mathutils import Vector

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(Path(__file__).parent))
from hs_kit import Prop, PROP_TO_BLENDER, lin, make_materials

def B(v):
    return PROP_TO_BLENDER @ Vector(v)
from build_ship_exteriors import merge_copies

OUT=ROOT/'assets/models/grabon';TEX=ROOT/'assets/textures/grabon'


def material(name,color,linework=False):
    m=bpy.data.materials.new(name);m.use_nodes=True;n=m.node_tree.nodes;l=m.node_tree.links;n.clear()
    out=n.new('ShaderNodeOutputMaterial');e=n.new('ShaderNodeEmission');e.inputs['Color'].default_value=(*lin(color),1);l.new(e.outputs[0],out.inputs['Surface'])
    if linework:
        # Author long transverse joints directly in the ship frame. A much wider
        # longitudinal spacing keeps broad plates quiet instead of making a grid.
        g=n.new('ShaderNodeNewGeometry');sep=n.new('ShaderNodeSeparateXYZ');l.new(g.outputs['Position'],sep.inputs[0])
        def seam(axis,period,width):
            ping=n.new('ShaderNodeMath');ping.operation='PINGPONG';ping.inputs[1].default_value=period/2;l.new(sep.outputs[axis],ping.inputs[0])
            less=n.new('ShaderNodeMath');less.operation='LESS_THAN';less.inputs[1].default_value=width;l.new(ping.outputs[0],less.inputs[0]);return less.outputs[0]
        a=seam('Y',7.2,.045);b=seam('X',18,.045)
        mx=n.new('ShaderNodeMath');mx.operation='MAXIMUM';l.new(a,mx.inputs[0]);l.new(b,mx.inputs[1])
        mix=n.new('ShaderNodeMixRGB');l.new(mx.outputs[0],mix.inputs[0]);mix.inputs[1].default_value=(*lin(color),1);mix.inputs[2].default_value=(*lin([.18,.14,.23]),1);l.new(mix.outputs[0],e.inputs['Color'])
    return m


def color(ob,mat):
    ob.data.materials.clear();ob.data.materials.append(mat)
    for p in ob.data.polygons:p.material_index=0
    return ob


def guide(s,raw):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    make_materials()
    p=Prop(s['id'],'grabon-detail','approved hull with fitted exterior equipment')
    srgb=lambda h:[int(h[i:i+2],16)/255 for i in (1,3,5)]
    mats={k:material(k,srgb(v),k=='armor') for k,v in s['colors'].items()}
    mats['steel']=material('steel',[.34,.35,.38]);mats['dark']=material('dark',[.11,.105,.14]);mats['lens']=material('lens',[.12,.28,.32])
    objects=[];a=raw['attributes'];sun=Vector((-.5,.9,.3)).normalized()
    for part,(start,count) in raw['roles'].items():
        vertices=[B(a['position'][i*3:i*3+3]) for i in range(start,start+count)]
        mesh=bpy.data.meshes.new(part);mesh.from_pydata(vertices,[],[tuple(range(i,i+3)) for i in range(0,count,3)]);mesh.update()
        ob=bpy.data.objects.new(s['id']+'.'+part,mesh);bpy.context.scene.collection.objects.link(ob)
        n=Vector(a['normal'][start*3:start*3+3]);shade=.47+.53*max(0,n.dot(sun))
        c=[v/shade for v in a['color'][start*3:start*3+3]]
        # The original palette is recovered from vertex lighting, then the guide
        # uses uniform albedo. The final mesh receives its own fixed sun colors.
        def err(key):return sum((c[k]-lin(srgb(s['colors'][key]))[k])**2 for k in range(3))
        role=min(s['colors'],key=err)
        color(ob,mats[role]);bm=bmesh.new();bm.from_mesh(mesh);bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.0001);bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(mesh);bm.free()
        objects.append(ob)
    def box(name,c,size,role='dark',bevel=.03):
        ob=p.box(name,[c[k]-size[k]/2 for k in range(3)],[c[k]+size[k]/2 for k in range(3)])
        color(ob,mats[role])
        if bevel:
            mod=ob.modifiers.new('Edge chamfer','BEVEL');mod.width=bevel;mod.segments=1
            bpy.context.view_layer.objects.active=ob;bpy.ops.object.modifier_apply(modifier=mod.name)
        objects.append(ob);return ob
    def pipe(name,a,b,r=.075,role='steel'):
        va,vb=B(a),B(b);d=vb-va
        bpy.ops.mesh.primitive_cylinder_add(vertices=6,radius=r,depth=d.length,location=(va+vb)/2)
        ob=bpy.context.object;ob.name=s['id']+'.'+name;ob.rotation_euler=d.to_track_quat('Z','Y').to_euler();color(ob,mats[role]);objects.append(ob)
    # Roof maintenance panels follow the occupied hull modules. Their footprint
    # is inside each bevel and all hardware is above the authored roof.
    for zone in s['hull_zones']:
        if zone['id'] not in ('body','engineering','command','head'):continue
        poly=zone['poly'];z0=min(v[1] for v in poly);z1=max(v[1] for v in poly);top=zone['y_m'][1]
        width=max(abs(v[0]) for v in poly)
        for side in (-1,1):
            x=side*width*.47
            for i,z in enumerate([z0+(z1-z0)*.29,z0+(z1-z0)*.64]):
                # Command vents and turrets retain a clear central maintenance run.
                if zone['id']=='command':x=side*width*.72
                box('hatch_frame',[x,top+.095,z],[1.65,.14,2.25],'dark',.09)
                box('hatch_lid',[x,top+.195,z],[1.39,.08,1.95],'secondary',.08)
                box('hatch_handle',[x,top+.27,z+.46],[.46,.055,.12],'steel',.01)
                for dz in (-.85,.85):box('hatch_clamp',[x,top+.30,z+dz],[.34,.10,.12],'steel',.01)
        # Recessed service spine on the head and open aft roof, with two real pipes.
        if zone['id'] in ('body','engineering'):
            end=z0+(z1-z0)*.24
            box('service_trough',[0,top+.045,(z0+.7+end)/2],[1.45,.07,end-z0-.7],'dark',.01)
            for x in (-.4,.4):pipe('coolant_run',[x,top+.16,z0+1],[x,top+.16,end-.1])
    for f in s['fittings']:
        if f['kind']=='engine':
            x,y,z=f['center_m'];w,h,l=f['size_m']
            # Two inset louver arrays, attachment cleats and exposed feed pipes.
            for dx in (-w*.25,w*.25):
                box('engine_grille',[x+dx,y+h/2+.045,z],[w*.22,.06,l*.40],'dark',.02)
                for i in range(8):box('engine_louver',[x+dx,y+h/2+.12,z-l*.16+i*l*.32/7],[w*.20,.08,.18],'steel',.015)
            for side in (-1,1):
                for zz in (-.28,.25):box('engine_cleat',[x+side*(w/2+.055),y+h*.28,z+l*zz],[.10,.46,.7],'steel',.025)
                pipe('engine_feed',[x+side*w*.38,y+h/2+.22,z-l*.34],[x+side*w*.38,y+h/2+.22,z+l*.27],.07)
        elif f['kind']=='shell' and f.get('id','').startswith('wing_plate') and f.get('color_role')=='armor':
            poly=f['poly'];cx=sum(v[0] for v in poly)/len(poly);cz=sum(v[1] for v in poly)/len(poly)
            yy=f['y_m'][1]+(abs(cx)-f['slope']['root_x_m'])*f['slope']['rise_per_m']
            box('wing_inspection',[cx,yy+.1,cz],[1.15,.12,1.9],'secondary',.08)
            for dz in (-.65,.65):box('wing_fastener',[cx,yy+.19,cz+dz],[.32,.04,.12],'steel',.01)
    source=bpy.data.collections.new('Editable assemblies');bpy.context.scene.collection.children.link(source)
    for ob in objects:
        for coll in list(ob.users_collection):coll.objects.unlink(ob)
        source.objects.link(ob)
    merged=merge_copies(objects,s['id']+'_hull')
    for ob in objects:ob.hide_render=True;ob.hide_set(True)
    bpy.ops.object.select_all(action='DESELECT');merged.hide_set(False);merged.select_set(True);bpy.context.view_layer.objects.active=merged
    bpy.ops.object.transform_apply(location=False,rotation=True,scale=True)
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT');bpy.ops.uv.smart_project(angle_limit=math.radians(60),island_margin=.006);bpy.ops.object.mode_set(mode='OBJECT')
    scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=1;scene.cycles.use_denoising=False
    scene.render.bake.margin=8
    image=bpy.data.images.new(s['id']+'_projection',width=2048,height=2048,alpha=False)
    for m in merged.data.materials:
        n=m.node_tree.nodes.new('ShaderNodeTexImage');n.image=image;m.node_tree.nodes.active=n
    bpy.ops.object.bake(type='EMIT');image.filepath_raw=str(TEX/(s['id']+'-projection.png'));image.file_format='PNG';image.save()
    # UV wireframe is a supporting guide. Blender's UV exporter draws actual UVs.
    try:bpy.ops.uv.export_layout(filepath=str(TEX/(s['id']+'-uv.svg')),export_all=True,mode='SVG',size=(2048,2048),opacity=.15)
    except Exception as ex:print('UV export:',ex)
    # Keep editable projected materials and source fittings before enhancement.
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/(s['id']+'-projection.blend')))
    print('GUIDE',s['id'],len(merged.data.polygons),flush=True)


def review(s):
    bpy.ops.wm.open_mainfile(filepath=str(OUT/(s['id']+'-projection.blend')))
    ob=bpy.data.objects[s['id']+'_hull'];scene=bpy.context.scene
    for m in ob.data.materials:
        nodes=m.node_tree.nodes;links=m.node_tree.links;e=next(n for n in nodes if n.type=='EMISSION')
        base=e.inputs['Color'].links[0].from_socket if e.inputs['Color'].is_linked else None
        rgb=nodes.new('ShaderNodeRGB');rgb.outputs[0].default_value=e.inputs['Color'].default_value
        normal=nodes.new('ShaderNodeNewGeometry');dot=nodes.new('ShaderNodeVectorMath');dot.operation='DOT_PRODUCT';dot.inputs[1].default_value=B([-.5,.9,.3]).normalized();links.new(normal.outputs['Normal'],dot.inputs[0])
        clamp=nodes.new('ShaderNodeMath');clamp.operation='MAXIMUM';clamp.inputs[1].default_value=0;links.new(dot.outputs['Value'],clamp.inputs[0])
        level=nodes.new('ShaderNodeMath');level.operation='MULTIPLY_ADD';level.inputs[1].default_value=.56;level.inputs[2].default_value=.44;links.new(clamp.outputs[0],level.inputs[0])
        mix=nodes.new('ShaderNodeMixRGB');mix.blend_type='MULTIPLY';mix.inputs[0].default_value=1;links.new(base or rgb.outputs[0],mix.inputs[1]);links.new(level.outputs[0],mix.inputs[2]);links.new(mix.outputs[0],e.inputs['Color'])
    camera=bpy.data.objects.new('Review camera',bpy.data.cameras.new('Review camera'));scene.collection.objects.link(camera);scene.camera=camera
    camera.location=B([75,68,85]);camera.rotation_euler=(B([0,4,3])-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=102
    scene.world=bpy.data.worlds.new('Space');scene.world.color=(.018,.024,.03);scene.view_settings.view_transform='Standard';scene.render.resolution_x=1200;scene.render.resolution_y=900;scene.render.resolution_percentage=100
    dest=ROOT/'docs/screenshots/grabon-detail/projection';dest.mkdir(parents=True,exist_ok=True)
    scene.render.filepath=str(dest/(s['id']+'.png'));bpy.ops.render.render(write_still=True)
    uv=ob.data.uv_layers.active.data
    (TEX/(s['id']+'-uv.json')).write_text(json.dumps([[[round(v,6) for v in uv[i].uv] for i in f.loop_indices] for f in ob.data.polygons]))


def maps(s):
    """Save actual UV edges and material regions from the locked projection mesh."""
    bpy.ops.wm.open_mainfile(filepath=str(OUT/(s['id']+'-projection.blend')))
    ob=bpy.data.objects[s['id']+'_hull'];bpy.context.view_layer.objects.active=ob
    bpy.ops.object.select_all(action='DESELECT');ob.hide_set(False);ob.select_set(True)
    bpy.ops.uv.export_layout(filepath=str(TEX/(s['id']+'-uv.svg')),export_all=True,mode='SVG',size=(2048,2048),opacity=0)
    wire=TEX/(s['id']+'-uv.svg');wire.write_text('\n'.join(line.rstrip() for line in wire.read_text().splitlines())+'\n')
    colors={'armor':[.5,.12,.8],'accent':[1,.3,.02],'secondary':[.18,.22,.28],
            'edge':[.42,.48,.56],'glow':[.15,1,1],'steel':[.6,.6,.6],'dark':[.07,.07,.09],'lens':[.02,.25,.5]}
    im=bpy.data.images.new(s['id']+'_regions',width=2048,height=2048,alpha=False)
    for m in ob.data.materials:
        nodes=m.node_tree.nodes;links=m.node_tree.links;nodes.clear()
        out=nodes.new('ShaderNodeOutputMaterial');em=nodes.new('ShaderNodeEmission')
        em.inputs['Color'].default_value=(*colors.get(m.name.split('.')[0],[1,1,1]),1)
        links.new(em.outputs[0],out.inputs['Surface']);tex=nodes.new('ShaderNodeTexImage');tex.image=im;nodes.active=tex
    bpy.context.scene.render.bake.margin=0;bpy.ops.object.bake(type='EMIT')
    im.filepath_raw=str(TEX/(s['id']+'-regions.png'));im.file_format='PNG';im.save()
    (TEX/'regions.json').write_text(json.dumps(colors,indent=2)+'\n')


def finish(s):
    bpy.ops.wm.open_mainfile(filepath=str(OUT/(s['id']+'-projection.blend')))
    ob=bpy.data.objects[s['id']+'_hull'];bpy.context.view_layer.objects.active=ob
    path=TEX/(s['id']+'-enhanced.png');assert path.exists(),path
    tex=bpy.data.images.load(str(path));tex.pack()
    m=bpy.data.materials.new('Grabon painted hull');m.use_nodes=True;m.use_backface_culling=True;n=m.node_tree.nodes;l=m.node_tree.links;n.clear()
    out=n.new('ShaderNodeOutputMaterial');emit=n.new('ShaderNodeEmission');im=n.new('ShaderNodeTexImage');im.image=tex;im.interpolation='Linear'
    vc=n.new('ShaderNodeVertexColor');vc.layer_name='Sun';mix=n.new('ShaderNodeMixRGB');mix.blend_type='MULTIPLY';mix.inputs[0].default_value=1
    l.new(im.outputs['Color'],mix.inputs[1]);l.new(vc.outputs['Color'],mix.inputs[2]);l.new(mix.outputs[0],emit.inputs['Color']);l.new(emit.outputs[0],out.inputs['Surface'])
    # Refit the editable assemblies while preserving their enhanced paint UVs.
    from grabon_exterior_refit import refit
    ob,refit_facts=refit(s,ob,m)
    bpy.context.view_layer.objects.active=ob
    # COLOR_0 must contain the light bake, not a primitive's old white tint.
    for attr in list(ob.data.color_attributes):ob.data.color_attributes.remove(attr)
    sun=B([-.45,.82,.34]).normalized();col=ob.data.color_attributes.new(name='Sun',type='FLOAT_COLOR',domain='CORNER')
    for poly in ob.data.polygons:
        mat=ob.data.materials[poly.material_index]
        shade=1 if mat.get('fullbright') else .44+.56*max(0,poly.normal.dot(sun))
        tint=mat.get('vertex_tint',[1,1,1])
        for i in poly.loop_indices:col.data[i].color=(*(shade*v for v in tint),1)
    bpy.ops.object.select_all(action='DESELECT');ob.hide_set(False);ob.select_set(True)
    ob.data.calc_loop_triangles();tris=len(ob.data.loop_triangles);assert tris<=16000,(s['id'],tris)
    vertices=[PROP_TO_BLENDER.inverted()@v.co for v in ob.data.vertices]
    bounds=[[round(min(v[k] for v in vertices),4),round(max(v[k] for v in vertices),4)] for k in range(3)]
    # The direct color output exports as KHR_materials_unlit with COLOR_0 lighting.
    l.new(mix.outputs[0],out.inputs['Surface'])
    bpy.ops.export_scene.gltf(filepath=str(OUT/(s['id']+'.glb')),use_selection=True,export_format='GLB',export_yup=True,export_animations=False,export_vertex_color='ACTIVE',export_all_vertex_colors=False)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT/(s['id']+'.blend')))
    return {'file':s['id']+'.glb','triangles':tris,'budget_triangles':16000,'texture':str(path.relative_to(ROOT)).replace('\\','/'),
            'texture_px':list(tex.size),'texture_bytes_with_mips':int((tex.size[0]*tex.size[1]+128*128)*4*4/3),
            'exterior_refit':refit_facts,
            'bounds_m':bounds,'dimensions_m':[round(b-a,4) for a,b in bounds],
            'sha256':hashlib.sha256((OUT/(s['id']+'.glb')).read_bytes()).hexdigest()}


if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--guide',action='store_true');ap.add_argument('--finish',action='store_true');ap.add_argument('--review',action='store_true');ap.add_argument('--maps',action='store_true');ap.add_argument('--only');args=ap.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else sys.argv[1:])
    OUT.mkdir(parents=True,exist_ok=True);TEX.mkdir(parents=True,exist_ok=True)
    data=json.loads((ROOT/'data/ships/grabon-detailed/layouts.json').read_text())
    raw={s['id']:s for s in json.loads((ROOT/'build/grabon-detail/hulls.json').read_text())} if args.guide else {}
    manifest=json.loads((OUT/'props.json').read_text()) if args.only and (OUT/'props.json').exists() else {'schema':'starcrew.props/1','generator':'tools/blender/build_grabon_detail.py','props':{}}
    for s in data['ships']:
        if args.only and s['id']!=args.only:continue
        if args.guide:guide(s,raw[s['id']])
        if args.review:review(s)
        if args.maps:maps(s)
        if args.finish:manifest['props'][s['id']]=finish(s)
    if args.finish:(OUT/'props.json').write_text(json.dumps(manifest,indent=2)+'\n')
