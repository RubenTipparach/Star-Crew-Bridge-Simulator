"""Star Crew's crew bodies (openspec/changes/crew-bodies): cartoon low-poly people from MakeHuman, fitted to
the Pi 5 avatar (crew-on-deck 17: at most 3,000 triangles and 30 bones, palette colours, one material).

The owner, 2026-10-10: "using make human in blender can you make better low poly characters? exagerate
portions a bit to make them look more cartoony". Each row of data/crew/bodies.json becomes
assets/models/crew/<id>.glb:

  1. MPFB 2 (the MakeHuman add-on) builds the human from the pinned CC0 packs: the phenotype sliders, the
     low-poly proxy body, the game_engine rig with its skin weights, the low-poly eyes and the hair. No
     clothes: the uniform is painted on by region.
  2. The hair and the body are decimated to the budget.
  3. Cartoon proportions: bones scaled in pose mode (each about its own head, inheriting no scale), the
     posed mesh applied and the pose made the rest pose, so the skin weights carry it.
  4. Lifted to the floor and scaled to the row's height.
  5. The rig cut to 30 bones (the fingers merged into one finger and one thumb bone a hand, three prop
     bones added) and two influences a vertex.
  6. One colour a face by region (skin, tunic, trousers, boots from the bone that holds it; hair; eyes),
     the tunic flagged in alpha for the game's role colour; flat shaded, one material, no texture.
  7. Exported, and the budget read back from the written glb.

The packs: tools/deps/character_packs.json (copied from Undercity, which builds its people the same way:
the blender-humanoid-characters skill). They are unpacked into a private Blender user folder in the cache
($STARCREW_DEPS, else ~/.cache/starcrew/deps), never into the repository or the user's own Blender.

Run (any Python with the bpy module, 4.5):
  <python with bpy> tools/blender/build_crew_bodies.py [--only id,id]
"""
import hashlib
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import urllib.request
import zipfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
TABLE = os.path.join(ROOT, "data", "crew", "bodies.json")
PACKS = os.path.join(ROOT, "tools", "deps", "character_packs.json")
OUT = os.path.join(ROOT, "assets", "models", "crew")
GENERATOR = "tools/blender/build_crew_bodies.py"
CACHE = os.environ.get("STARCREW_DEPS") or os.path.expanduser("~/.cache/starcrew/deps")
PACK_DIRS = [os.path.join(CACHE, "packs"), os.path.expanduser("~/.cache/undercity/deps/packs")]
MPFB = "bl_ext.user_default.mpfb"
SIDES = ("l", "r")
FINGERS = ("index", "middle", "ring", "pinky")


class BuildError(RuntimeError):
    """A body that can't be built as the table asks; the message names it."""


# ----------------------------------------------------------------------------- the table

TOP_KEYS = {"schema", "status", "_rules", "proxies", "eyes", "style", "eyebrows", "eye_scale", "eye_forward_m", "brow_lift_m", "brow_forward_m", "_comment_eye_scale", "collar_width_m", "_comment_collar",
            "iris_ratio", "pupil_ratio", "eye_segments", "_comment_eye_mesh", "_comment_eye_ratio", "eye_colours_srgb", "clothes", "badge", "cartoon",
            "regions", "region_srgb", "skin_tones_srgb", "hair_colours_srgb", "role_srgb", "budget", "bodies"}
ROW_KEYS = {"id", "sex", "age_years", "muscle", "weight", "height_m", "skin", "hair", "hair_colour", "role", "eye_colour"}


def load_table():
    """The body table, checked: an unknown or a missing key stops the build with its path (CLAUDE.md 6.5)."""
    with open(TABLE, encoding="utf-8") as f:
        t = json.load(f)

    def exact(d, keys, where):
        if set(d) != keys:
            raise BuildError(f"{TABLE}: {where}: unknown {sorted(set(d) - keys)} missing {sorted(keys - set(d))}")
    exact(t, TOP_KEYS, "top level")
    exact(t["clothes"], {"_comment", "suit", "shoes"}, "clothes")
    for i, b in enumerate(t["bodies"]):
        exact(b, ROW_KEYS, f"bodies[{i}]")
        if b["sex"] not in t["proxies"]:
            raise BuildError(f"{TABLE}: {b['id']}: sex {b['sex']!r}")
        if not 18 <= b["age_years"] <= 70 or not 0 <= b["muscle"] <= 1 or not 0 <= b["weight"] <= 1:
            raise BuildError(f"{TABLE}: {b['id']}: age, muscle or weight out of range")
        if not 1.4 <= b["height_m"] <= 1.80:
            raise BuildError(f"{TABLE}: {b['id']}: height_m {b['height_m']} outside 1.40-1.80 m (the walk body is 1.80 m)")
        for key, pal in (("skin", "skin_tones_srgb"), ("hair_colour", "hair_colours_srgb"), ("role", "role_srgb"),
                         ("eye_colour", "eye_colours_srgb")):
            if b[key] not in t[pal]:
                raise BuildError(f"{TABLE}: {b['id']}: {key} {b[key]!r} is not in {pal}")
    return t


# ----------------------------------------------------------------------------- packs and MPFB

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pack_path(pack):
    """The pinned pack's zip, from a cache that has the exact file, else downloaded from its url."""
    for d in PACK_DIRS:
        p = os.path.join(d, pack["file"])
        if os.path.isfile(p) and os.path.getsize(p) == pack["size_bytes"] and sha256_file(p) == pack["sha256"]:
            return p
    os.makedirs(PACK_DIRS[0], exist_ok=True)
    p = os.path.join(PACK_DIRS[0], pack["file"])
    print(f"[crew] fetching {pack['id']}: {pack['url']}", flush=True)
    urllib.request.urlretrieve(pack["url"], p + ".part")
    if sha256_file(p + ".part") != pack["sha256"]:
        os.remove(p + ".part")
        raise BuildError(f"pack {pack['id']}: the download does not match its pinned SHA-256")
    os.replace(p + ".part", p)
    return p


def private_user_folder():
    return os.path.join(CACHE, "blender")


def ensure_private_blender():
    """Run again with the private Blender user folder and a fixed hash seed, so MPFB never lands in the
    user's own Blender settings and the build is reproducible."""
    want = private_user_folder()
    if os.environ.get("BLENDER_USER_RESOURCES") == want and os.environ.get("PYTHONHASHSEED") == "0":
        return
    os.makedirs(want, exist_ok=True)
    env = dict(os.environ, BLENDER_USER_RESOURCES=want, PYTHONHASHSEED="0")
    sys.stdout.flush()
    os._exit(subprocess.run([sys.executable, "-u", os.path.abspath(__file__)] + sys.argv[1:], env=env).returncode)


def install_mpfb():
    """Unpack MPFB and the CC0 assets into the private folder (once), enable MPFB, return its services."""
    import bpy  # first: addon_utils exists only once bpy is imported
    import addon_utils
    with open(PACKS, encoding="utf-8") as f:
        packs = {p["id"]: p for p in json.load(f)["packs"]}
    repo = next(r for r in bpy.context.preferences.extensions.repos if r.module == "user_default")
    mpfb_dir = os.path.join(repo.directory, "mpfb")
    stamp = os.path.join(mpfb_dir, ".starcrew_pack")
    if not os.path.isfile(stamp) or open(stamp).read() != packs["mpfb"]["sha256"]:
        shutil.rmtree(mpfb_dir, ignore_errors=True)
        zipfile.ZipFile(pack_path(packs["mpfb"])).extractall(mpfb_dir)
        open(stamp, "w").write(packs["mpfb"]["sha256"])
    addon_utils.enable(MPFB, default_set=True)
    from bl_ext.user_default.mpfb.services import AssetService, HumanService, LocationService
    data = LocationService.get_user_data()
    stamp = os.path.join(data, ".starcrew_pack")
    if not os.path.isfile(stamp) or open(stamp).read() != packs["makehuman_system_assets_cc0"]["sha256"]:
        zipfile.ZipFile(pack_path(packs["makehuman_system_assets_cc0"])).extractall(data)
        open(stamp, "w").write(packs["makehuman_system_assets_cc0"]["sha256"])
    AssetService.rescan_pack_metadata()
    AssetService.update_all_asset_lists()
    return HumanService


def age_slider(years):
    """MakeHuman's age slider for an age in years: 1 y is 0, 11 y 0.1875, 25 y 0.5, 90 y 1 (MPFB's anchors)."""
    anchors = [(1.0, 0.0), (11.0, 0.1875), (25.0, 0.5), (90.0, 1.0)]
    years = min(max(years, 1.0), 90.0)
    for (y0, s0), (y1, s1) in zip(anchors, anchors[1:]):
        if years <= y1:
            return s0 + (years - y0) / (y1 - y0) * (s1 - s0)
    return 1.0


# ----------------------------------------------------------------------------- Blender helpers

def ctx(obj, fn, **kw):
    import bpy
    with bpy.context.temp_override(object=obj, active_object=obj, selected_objects=[obj], selected_editable_objects=[obj]):
        return fn(**kw)


def tris(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def decimate_to(obj, target):
    """Collapse-decimate a mesh to about target triangles (symmetric, so left and right match)."""
    import bpy
    have = tris(obj)
    if have <= target:
        return
    m = obj.modifiers.new("Decimate", 'DECIMATE')
    m.ratio = target / have
    m.use_symmetry = True
    ctx(obj, bpy.ops.object.modifier_move_to_index, modifier=m.name, index=0)
    ctx(obj, bpy.ops.object.modifier_apply, modifier=m.name)


def bone_kind(name):
    """A bone's family for the table's cartoon and region keys: hand_l -> hand, index_02_r -> index."""
    base = name
    for s in ("_l", "_r"):
        if base.endswith(s):
            base = base[:-2]
    for f in FINGERS + ("thumb",):
        if base.startswith(f + "_"):
            return f
    return base


# ----------------------------------------------------------------------------- the body

def build_body(row, t, HumanService):
    import bpy
    from mathutils import Matrix, Vector
    bpy.ops.wm.read_factory_settings(use_empty=True)
    import addon_utils
    addon_utils.enable(MPFB, default_set=True)
    info = HumanService._create_default_human_info_dict()
    info["name"] = row["id"]
    info["phenotype"].update(gender=1.0 if row["sex"] == "male" else 0.0, age=age_slider(row["age_years"]),
                             muscle=row["muscle"], weight=row["weight"], height=0.5, proportions=0.5)
    info["rig"] = "game_engine"
    info["proxy"] = "{0}/{0}.proxy".format(t["proxies"][row["sex"]])
    info["eyes"] = "{0}/{0}.mhclo".format(t["eyes"])
    info["hair"] = "{0}/{0}.mhclo".format(row["hair"]) if row["hair"] else ""
    info["eyebrows"] = "{0}/{0}.mhclo".format(t["eyebrows"])
    info["eyelashes"] = ""
    info["clothes"] = ["{0}/{0}.mhclo".format(t["clothes"][k]) for k in ("suit", "shoes")]
    info["skin_material_type"] = "GAMEENGINE"
    st = HumanService.get_default_deserialization_settings()
    st["subdiv_levels"] = 0
    st["override_eyes_model"] = "GAMEENGINE"
    HumanService.deserialize_from_dict(info, st)

    rig = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
    parts = {}
    for o in list(bpy.data.objects):
        if o.type != 'MESH':
            continue
        if o.name.endswith(".body"):            # MPFB's full base mesh, hidden under the proxy: not ours
            bpy.data.objects.remove(o, do_unlink=True)
        elif o.name.endswith(t["proxies"][row["sex"]]):
            parts["body"] = o
        elif o.name.endswith("." + t["eyes"]):
            parts["eyes"] = o
        elif row["hair"] and o.name.endswith("." + row["hair"]):
            parts["hair"] = o
        elif o.name.endswith("." + t["eyebrows"]):
            parts["brows"] = o
        elif o.name.endswith("." + t["clothes"]["suit"]):
            parts["suit"] = o
        elif o.name.endswith("." + t["clothes"]["shoes"]):
            parts["boots"] = o
    for k in ("body", "eyes", "brows", "suit", "boots"):
        if k not in parts:
            raise BuildError(f"{row['id']}: MPFB made no {k} mesh")
    for o in parts.values():
        for m in list(o.modifiers):
            if m.type == 'MASK':           # the garments' delete groups: the skin under the clothes goes
                ctx(o, bpy.ops.object.modifier_apply, modifier=m.name)
            elif m.type != 'ARMATURE':
                o.modifiers.remove(m)
        o.data.materials.clear()
    hide_covered_skin(parts["body"], rig, t)
    # The suit's two loose pieces: the upper is the tunic, the lower the trousers.
    suit = parts.pop("suit")
    for o in bpy.context.view_layer.objects:
        o.select_set(o == suit)
    bpy.context.view_layer.objects.active = suit
    bpy.ops.object.mode_set(mode='EDIT')
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.separate(type='LOOSE')
    bpy.ops.object.mode_set(mode='OBJECT')
    pieces = [o for o in bpy.data.objects if o.type == 'MESH' and o.name.startswith(suit.name)]
    if len(pieces) != 2:
        raise BuildError(f"{row['id']}: suit {t['clothes']['suit']} is {len(pieces)} loose pieces, not 2 (tunic and trousers)")
    mean_z = {o.name: sum(v.co.z for v in o.data.vertices) / len(o.data.vertices) for o in pieces}
    parts["tunic"], parts["trousers"] = sorted(pieces, key=lambda o: -mean_z[o.name])

    # 2. Decimate to the budget: the hair to its share, the body to the rest.
    if "hair" in parts:
        decimate_to(parts["hair"], t["budget"]["hair_triangles"])
    for k in ("tunic", "trousers", "boots"):
        decimate_to(parts[k], t["budget"][k + "_triangles"])
    rest = t["budget"]["triangles"] - sum(tris(o) for k, o in parts.items() if k != "body") - 40
    decimate_to(parts["body"], rest)

    # 3. Cartoon proportions: pose scales, each bone inheriting no scale, applied as the rest pose.
    for b in rig.data.bones:
        b.inherit_scale = 'NONE'
    families = {"hands": ("hand",) + FINGERS + ("thumb",), "feet": ("foot", "ball"), "clavicle": ("clavicle",),
                "thigh": ("thigh",), "calf": ("calf",), "head": ("head",), "neck_01": ("neck_01",),
                "spine_01": ("spine_01",), "spine_02": ("spine_02",), "spine_03": ("spine_03",), "upperarm": ("upperarm",),
                "lowerarm": ("lowerarm",)}
    for key, spec in t["cartoon"].items():
        kinds = families[key]
        if set(spec) - {"uniform", "along", "across"} or not spec:
            raise BuildError(f"{TABLE}: cartoon.{key}: keys are uniform, along and across")
        u = spec.get("uniform", 1.0)
        scale = (u * spec.get("across", 1.0), u * spec.get("along", 1.0), u * spec.get("across", 1.0))
        for pb in rig.pose.bones:
            if bone_kind(pb.name) in kinds:
                pb.scale = scale
    bpy.context.view_layer.update()
    for o in parts.values():
        mod = next(m for m in o.modifiers if m.type == 'ARMATURE')
        ctx(o, bpy.ops.object.modifier_apply, modifier=mod.name)
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode='POSE')
    bpy.ops.pose.armature_apply(selected=False)
    bpy.ops.object.mode_set(mode='OBJECT')
    for b in rig.data.bones:
        b.inherit_scale = 'FULL'

    # Big cartoon eyes: each eyeball scaled about its own centre (left and right by the sign of x).
    eyes = parts["eyes"]
    for side in (-1, 1):
        vs = [v for v in eyes.data.vertices if (v.co.x > 0) == (side > 0)]
        c = sum((v.co for v in vs), Vector()) / len(vs)
        for v in vs:
            v.co = c + (v.co - c) * t["eye_scale"] + Vector((0.0, -t["eye_forward_m"], 0.0))
    rebuild_eyes(eyes, t)
    # The brows rise above the bigger eyes and come forward over them.
    for v in parts["brows"].data.vertices:
        v.co += Vector((0.0, -t["brow_forward_m"], t["brow_lift_m"]))

    # 4. On the floor, at the row's height (the whole body scaled, rig and mesh together).
    meshes = list(parts.values())
    for o in meshes:
        mw = o.matrix_world.copy()
        o.parent = None
        o.matrix_world = mw
    world = [o.matrix_world @ v.co for o in meshes for v in o.data.vertices]
    lo = min(v.z for v in world)
    hi = max(v.z for v in world)
    k = row["height_m"] / (hi - lo)
    fix = Matrix.Scale(k, 4) @ Matrix.Translation((0.0, 0.0, -lo))
    for o in meshes + [rig]:
        o.matrix_world = fix @ o.matrix_world
    for o in meshes + [rig]:
        ctx(o, bpy.ops.object.transform_apply, location=True, rotation=True, scale=True)

    # 5. One mesh, two influences a vertex, the rig cut to 30 bones.
    body = parts["body"]
    for name, o in parts.items():
        a = o.data.attributes.new("part", 'INT', 'POINT')
        code = {"body": 0, "eyes": 1, "hair": 2, "brows": 3, "tunic": 4, "trousers": 5, "boots": 6}[name]
        for i in range(len(o.data.vertices)):
            a.data[i].value = code
    with bpy.context.temp_override(active_object=body, selected_editable_objects=meshes, object=body):
        bpy.ops.object.join()
    merge_groups(body)
    trim_rig(rig, body)
    ctx(body, bpy.ops.object.vertex_group_limit_total, group_select_mode='ALL', limit=2)
    ctx(body, bpy.ops.object.vertex_group_normalize_all, group_select_mode='ALL', lock_active=False)
    body.parent = rig
    mod = body.modifiers.new("Armature", 'ARMATURE')
    mod.object = rig

    # 6. One colour a face, by region.
    colour_regions(body, rig, row, t)
    for p in body.data.polygons:      # smooth, after the owner's Pixar references; the colours stay one a face
        p.use_smooth = True
    mat = bpy.data.materials.new(row["id"])
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    col = nodes.new("ShaderNodeVertexColor")
    col.layer_name = "Col"
    mat.node_tree.links.new(col.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 1.0
    body.data.materials.append(mat)
    body.name = row["id"]
    rig.name = row["id"] + "_rig"
    return rig, body, head_height(body, rig, t)


def merge_groups(obj):
    """Each hand's fingers into one finger group and its thumb into one thumb group (weights summed)."""
    vg = obj.vertex_groups
    for s in SIDES:
        plan = {f"fingers_{s}": [f"{f}_0{i}_{s}" for f in FINGERS for i in (1, 2, 3)],
                f"thumb_{s}": [f"thumb_0{i}_{s}" for i in (1, 2, 3)]}
        for target, sources in plan.items():
            idx = {vg[n].index: n for n in sources if n in vg}
            acc = {}
            for v in obj.data.vertices:
                w = sum(g.weight for g in v.groups if g.group in idx)
                if w > 0:
                    acc[v.index] = w
            for n in idx.values():
                vg.remove(vg[n])
            g = vg.new(name=target)
            for i, w in sorted(acc.items()):
                g.add([i], min(1.0, w), 'REPLACE')


def trim_rig(rig, body):
    """The game_engine rig's 53 bones to 30: every finger bone but the first of the index and of the thumb
    goes, those two are renamed fingers_<s> and thumb_<s>, and three prop bones (no weight) are added."""
    import bpy
    from mathutils import Vector
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode='EDIT')
    eb = rig.data.edit_bones
    for s in SIDES:
        for f in FINGERS + ("thumb",):
            for i in (1, 2, 3):
                n = f"{f}_0{i}_{s}"
                if n in eb and not (i == 1 and f in ("index", "thumb")):
                    eb.remove(eb[n])
        eb[f"index_01_{s}"].name = f"fingers_{s}"
        eb[f"thumb_01_{s}"].name = f"thumb_{s}"
    for name, parent, off in (("prop_hand_r", "hand_r", None), ("prop_hand_l", "hand_l", None),
                              ("prop_back", "spine_03", Vector((0.0, 0.18, 0.0)))):
        p = eb[parent]
        b = eb.new(name)
        b.head = p.tail.copy() if off is None else p.head + off
        b.tail = b.head + Vector((0.0, 0.0, 0.08))
        b.parent = p
        b.use_deform = False
    bpy.ops.object.mode_set(mode='OBJECT')
    keep = {b.name for b in rig.data.bones}
    for g in list(body.vertex_groups):
        if g.name not in keep:
            body.vertex_groups.remove(g)


def rebuild_eyes(eyes, t):
    """Replace each eyeball with a sphere whose rings lie on the pupil's and the iris's edges (the table's
    ratios), at the old eyeball's centre and radius, facing along its front. Each new vertex takes the weights
    of the old eyeball's vertices (an eye rides on the head)."""
    import bmesh
    from mathutils import Vector
    n = t["eye_segments"]
    plans = []
    for side in (-1, 1):
        vs = [v for v in eyes.data.vertices if (v.co.x > 0) == (side > 0)]
        c = sum((v.co for v in vs), Vector()) / len(vs)
        r = sum((v.co - c).length for v in vs) / len(vs)
        tip = sorted(vs, key=lambda v: v.co.y)[:max(1, len(vs) // 10)]
        axis = (sum((v.co for v in tip), Vector()) / len(tip) - c).normalized()
        groups = [(g.group, g.weight) for g in vs[0].groups]
        plans.append((c, r, axis, groups))
    bm = bmesh.new()
    deform = bm.verts.layers.deform.verify()
    zone = bm.faces.layers.int.new("eye_zone")     # 1 pupil, 2 iris, 3 white; 0 on every face that is no eye
    rings = (math.asin(t["iris_ratio"] * t["pupil_ratio"]), math.asin(t["iris_ratio"]), math.pi / 2)
    for c, r, axis, groups in plans:
        q = Vector((0.0, 0.0, 1.0)).rotation_difference(axis)

        def at(theta, phi):
            v = bm.verts.new(c + q @ Vector((math.sin(theta) * math.cos(phi), math.sin(theta) * math.sin(phi), math.cos(theta))) * r)
            for g, w in groups:
                v[deform][g] = w
            return v
        front, back = at(0.0, 0.0), at(math.pi, 0.0)
        loops = [[at(th, 2 * math.pi * k / n) for k in range(n)] for th in rings]
        for k in range(n):
            j = (k + 1) % n
            bm.faces.new((front, loops[0][k], loops[0][j]))[zone] = 1
            for z, (a, b) in enumerate(zip(loops, loops[1:])):
                for f in (bm.faces.new((a[k], b[k], b[j])), bm.faces.new((a[k], b[j], a[j]))):
                    f[zone] = 2 + z
            bm.faces.new((loops[-1][k], back, loops[-1][j]))[zone] = 3
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(eyes.data)
    bm.free()


def hide_covered_skin(body, rig, t):
    """Delete the body faces the garments cover: every face whose strongest bone is in the tunic, trousers or
    boots region (the arms, the torso, the legs, the feet), so no triangle of the budget is spent under the
    clothes and no skin pokes through them. The neck, head and hands stay."""
    import bmesh
    region_of = {}
    for b in rig.data.bones:
        for region, kinds in t["regions"].items():
            if bone_kind(b.name) in kinds:
                region_of[b.name] = region
    names = {g.index: g.name for g in body.vertex_groups}
    covered = []
    for p in body.data.polygons:
        weight = {}
        for vi in p.vertices:
            for g in body.data.vertices[vi].groups:
                r = region_of.get(names[g.group])
                if r:
                    weight[r] = weight.get(r, 0.0) + g.weight
        if weight and max(sorted(weight), key=lambda r: weight[r]) in ("tunic", "trousers", "boots"):
            covered.append(p.index)
    bm = bmesh.new()
    bm.from_mesh(body.data)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm, geom=[bm.faces[i] for i in covered], context='FACES')
    bm.to_mesh(body.data)
    bm.free()


def neck_opening(obj, part):
    """Points along the tunic's neck opening: the open edge loop of the tunic (part 4) highest on the body (the
    others are its hem and its cuffs), sampled every centimetre."""
    tunic = {p.index for p in obj.data.polygons if max(part[v].value for v in p.vertices) == 4}
    count = {}
    for p in obj.data.polygons:
        if p.index in tunic:
            for e in p.edge_keys:
                count[e] = count.get(e, 0) + 1
    edges = [e for e, n in count.items() if n == 1]
    link = {}
    for a, b in edges:
        link.setdefault(a, set()).add(b)
        link.setdefault(b, set()).add(a)
    loops, seen = [], set()
    for v in sorted(link):
        if v in seen:
            continue
        stack, comp = [v], []
        while stack:
            u = stack.pop()
            if u not in seen:
                seen.add(u)
                comp.append(u)
                stack.extend(link[u] - seen)
        loops.append(comp)
    co = obj.data.vertices
    top = max(loops, key=lambda c: sum(co[v].co.z for v in c) / len(c))
    top_set = set(top)
    pts = []
    for a, b in edges:
        if a in top_set and b in top_set:
            pa, pb = co[a].co, co[b].co
            n = max(1, int((pb - pa).length / 0.01))
            pts += [pa.lerp(pb, i / n) for i in range(n + 1)]
    return pts


def colour_regions(obj, rig, row, t):
    """One colour a face. Body faces take the region of the bone holding most of their vertices' weight
    (skin, tunic, trousers, boots); a tunic face within collar_radius_m of the neck's base is the collar band, and the tunic
    faces round a point on the left chest the badge. Eye faces facing forward (within iris_facing) are the
    iris, the rest the sclera; hair and brows take the hair colour. The tunic carries alpha 1 (the game's
    role-colour flag), every other face 0."""
    from mathutils import Vector
    region_of = {}
    for b in rig.data.bones:
        for region, kinds in t["regions"].items():
            if bone_kind(b.name) in kinds:
                region_of[b.name] = region
    missing = [b.name for b in rig.data.bones if b.use_deform and b.name not in region_of]
    if missing:
        raise BuildError(f"{row['id']}: bones in no region: {missing}")
    names = {g.index: g.name for g in obj.vertex_groups}
    colours = {"skin": t["skin_tones_srgb"][row["skin"]], "tunic": t["role_srgb"][row["role"]],
               "hair": t["hair_colours_srgb"][row["hair_colour"]], "iris": t["eye_colours_srgb"][row["eye_colour"]],
               **t["region_srgb"]}
    front = Vector((0.0, -1.0, 0.0))
    part = obj.data.attributes["part"].data
    zone = obj.data.attributes["eye_zone"].data     # marked by rebuild_eyes as it made the eye
    neck_edge = neck_opening(obj, part)
    face_region = {}
    for p in obj.data.polygons:
        kind = max(part[v].value for v in p.vertices)
        if kind == 1:
            region = {1: "pupil", 2: "iris"}.get(zone[p.index].value, "sclera")
        elif kind in (2, 3):
            region = "hair"
        elif kind in (4, 5, 6):
            region = {4: "tunic", 5: "trousers", 6: "boots"}[kind]
            if kind == 4 and min((p.center - v).length for v in neck_edge) < t["collar_width_m"]:
                region = "collar"
        else:
            weight = {}
            for vi in p.vertices:
                for g in obj.data.vertices[vi].groups:
                    r = region_of.get(names[g.group])
                    if r:
                        weight[r] = weight.get(r, 0.0) + g.weight
            region = max(sorted(weight), key=lambda r: weight[r]) if weight else "skin"
        face_region[p.index] = region
    # The badge: the tunic faces nearest a point on the front of the left chest.
    chest_z = rig.data.bones["spine_03"].head_local.z + rig.data.bones["spine_03"].length - t["badge"]["down_m"]
    tunic = [p for p in obj.data.polygons if face_region[p.index] == "tunic"]
    band = [p for p in tunic if abs(p.center.z - chest_z) < 0.06 and p.center.x > 0 and p.normal.dot(front) > 0.5]
    if band:
        x_at = t["badge"]["out_m"]
        anchor = min(band, key=lambda p: abs(p.center.x - x_at) + 0.5 * abs(p.center.z - chest_z)).center
        for p in tunic:
            if (p.center - anchor).length < t["badge"]["radius_m"] and p.normal.dot(front) > 0.3:
                face_region[p.index] = "badge"
    # What was painted, measured: widths across x of the right eye's faces by region.
    def width(regions):
        xs = [obj.data.vertices[v].co.x for p in obj.data.polygons if face_region[p.index] in regions and p.center.x > 0
              for v in p.vertices]
        return max(xs) - min(xs) if xs else 0.0
    ball, iris, pupil = width({"sclera", "iris", "pupil"}), width({"iris", "pupil"}), width({"pupil"})
    print(f"[crew] {row['id']}: eye {ball * 1000:.0f} mm, iris {iris / ball:.2f} of it, pupil {pupil / max(iris, 1e-9):.2f} "
          f"of the iris (vertex extents)", flush=True)
    attr = obj.data.color_attributes.new("Col", 'BYTE_COLOR', 'CORNER')
    for p in obj.data.polygons:
        region = face_region[p.index]
        c = colours[region]
        rgba = (c[0], c[1], c[2], 1.0 if region == "tunic" else 0.0)
        for li in p.loop_indices:
            attr.data[li].color_srgb = rgba
    obj.data.attributes.remove(obj.data.attributes["part"])
    obj.data.attributes.remove(obj.data.attributes["eye_zone"])
    obj.data.color_attributes.active_color = attr


def head_height(obj, rig, t):
    """The head's height (chin to crown): the z range of the vertices the head bone holds most."""
    gi = obj.vertex_groups["head"].index
    zs = []
    for v in obj.data.vertices:
        best = max(v.groups, key=lambda g: g.weight, default=None)
        if best is not None and best.group == gi:
            zs.append(v.co.z)
    return max(zs) - min(zs) if zs else 0.0


# ----------------------------------------------------------------------------- export and the budget

def glb_stats(path):
    """Triangles, bones, materials and textures, read back from the written glb."""
    with open(path, "rb") as f:
        data = f.read()
    n = struct.unpack_from("<I", data, 12)[0]
    g = json.loads(data[20:20 + n])
    tri = 0
    for m in g["meshes"]:
        for p in m["primitives"]:
            acc = g["accessors"][p["indices"]] if "indices" in p else g["accessors"][p["attributes"]["POSITION"]]
            tri += acc["count"] // 3
    return {"triangles": tri, "bones": sum(len(s["joints"]) for s in g.get("skins", [])),
            "materials": len(g.get("materials", [])), "textures": len(g.get("textures", [])),
            "colours": any("COLOR_0" in p["attributes"] for m in g["meshes"] for p in m["primitives"]),
            "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}


def export(row, rig, body, t, head_m):
    import bpy
    for o in bpy.data.objects:
        o.select_set(o in (rig, body))
    bpy.context.view_layer.objects.active = rig
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, row["id"] + ".glb")
    bpy.ops.export_scene.gltf(filepath=path, export_format='GLB', use_selection=True, export_yup=True,
                              export_skins=True, export_animations=False, export_morph=False,
                              export_materials='EXPORT', export_rest_position_armature=True, export_def_bones=False)
    s = glb_stats(path)
    b = t["budget"]
    bad = [f"{s[k]} {k}, over {b[k]}" for k in ("triangles", "bones", "materials") if s[k] > b[k]]
    if s["textures"] or not s["colours"]:
        bad.append("a texture or no vertex colours")
    if bad:
        os.remove(path)
        raise BuildError(f"{row['id']}: refused: " + "; ".join(bad))
    s.update(id=row["id"], height_m=row["height_m"], head_m=round(head_m, 3),
             heads_tall=round(row["height_m"] / head_m, 2) if head_m else None, role=row["role"])
    return s


def main():
    ensure_private_blender()
    t = load_table()
    only = None
    if "--only" in sys.argv:
        only = set(sys.argv[sys.argv.index("--only") + 1].split(","))
    HumanService = install_mpfb()
    man_path = os.path.join(OUT, "bodies.json")
    rows = {}
    if os.path.isfile(man_path):
        with open(man_path, encoding="utf-8") as f:
            rows = json.load(f).get("bodies", {})
    for row in t["bodies"]:
        if only and row["id"] not in only:
            continue
        rig, body, head_m = build_body(row, t, HumanService)
        s = export(row, rig, body, t, head_m)
        rows[row["id"]] = s
        print(f"[crew] {row['id']}: {s['triangles']} triangles, {s['bones']} bones, {s['height_m']} m, "
              f"{s['heads_tall']} heads tall, {s['bytes']} bytes", flush=True)
    keep = {r["id"] for r in t["bodies"]}
    with open(man_path, "w", encoding="utf-8") as f:
        json.dump({"schema": "starcrew.crew-bodies-manifest/1", "built_by": GENERATOR,
                   "table": "data/crew/bodies.json", "budget": t["budget"],
                   "bodies": {k: rows[k] for k in sorted(rows) if k in keep}}, f, indent=2)
        f.write("\n")
    print(f"[crew] wrote {os.path.relpath(man_path, ROOT)}", flush=True)


if __name__ == "__main__":
    try:
        main()
    except BuildError as e:
        print(f"[crew] {e}", flush=True)
        sys.stdout.flush()
        os._exit(1)
    sys.stdout.flush()
    os._exit(0)   # Blender's teardown hangs in the bpy module after MPFB: leave without it
