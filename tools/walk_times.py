#!/usr/bin/env python3
"""Walk times between key places on the Tern, from data/ships/tern/layout.json.

A measurement instrument for openspec/changes/crew-on-deck (CLAUDE.md section 4): it prints the
walk-time table that change's design quotes (task 1.3), then the further routes that
openspec/changes/reference-ship-tern section 6 and damage-control's walkthrough W1 quote. Every
waypoint is a layout point (a seat, a portal centre at floor height, a system, a fixture) or a
stated detour point, and the speeds are the design's proposed values. Standard library only.

Schema starcrew.ship-layout/2: a wall portal's floor point is its centre less half its height, a
floor portal's (a ladder or hatch, normal [0, +-1, 0]) is its centre. The stairs are the layout's
stair fixtures (reference-ship-tern's T2, applied 2026-10-04).

It then checks every route it printed against the plan, with tools/layout_check.py's geometry
(imported): each stated detour point lies inside its compartment, and each straight leg stays in
the air (sampled every 5 cm, 0.3 m above the floor), crosses a wall only through a portal, keeps
a floor under it, and keeps out of the reactor; a ladder climbs through a floor portal. A route
marked T1 needs reference-ship-tern's patch T1 (the hangar's galleries run forward to z = 0 to
meet the landing), which the layout does not hold yet, so it is checked against the plan with
that patch. A leg that crosses a parked craft is reported as a note: the paths ignore furniture
and craft, so real times are a little longer. A failed check exits with status 1.

Usage: python3 tools/walk_times.py [layout.json]
"""
import json, math, os, sys
sys.dont_write_bytecode = True  # importing the checker must not leave a __pycache__ in tools/
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from layout_check import SCHEMA, edges, inside_poly, outward_normal  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAYOUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'data', 'ships', 'tern', 'layout.json')
L = json.load(open(LAYOUT, encoding='utf-8'))
if L.get('schema') != SCHEMA:
    raise SystemExit(f"walk_times: {LAYOUT} is {L.get('schema')!r}, expected {SCHEMA!r}")
WALK, RUN, STAIR = 1.8, 4.0, 0.7
UP, DOWN, MOUNT = 0.8, 1.0, 0.5          # ladder m/s up, down; s to get on and to get off
STAND, SIT = 0.9, 0.4                     # hold 0.5 s + 0.4 s clip; 0.4 s clip
HATCH_SIDE = 1.5                          # climb through a side hatch
PRESSURE_DOOR = 2.0
CASUALTY, CAS_LADDER = 1.2, 0.5           # carrying a downed body: m/s flat; ladder speed factor

def floor_portal(p): return abs(p['normal'][1]) > 0.5

seat = {s['id']: s['seat_m'] for s in L['stations']}
portal = {}
for p in L['portals']:
    c = p['center_m']; sz = p['size_m']
    floor = c[1] if floor_portal(p) else c[1] - sz[1] / 2
    portal[p['id']] = [c[0], floor, c[2]]
system = {s['id']: s['center_m'] for s in L['systems']}
fx = {f['id']: f for f in L['fixtures']}

def P(name):
    if name in seat: return seat[name]
    if name in portal: return portal[name]
    if name in system: return system[name]
    raise KeyError(name)

def route(steps, start_seated=False, end_seated=False, casualty=False):
    """steps: list of ('pt', xyz) | ('stair', top, foot) | ('ladder', dy_m) | ('fixed', s, why).
    Returns the path, the times and the legs walked, ('flat' | 'stair' | 'ladder', from, to)."""
    flat = stair = 0.0; fixed = 0.0; lad = 0.0; lad_c = 0.0; cur = None; legs = []
    for st in steps:
        if st[0] == 'pt':
            p = st[1]
            if cur is not None: flat += math.hypot(p[0] - cur[0], p[2] - cur[2]); legs.append(('flat', cur, p))
            cur = p
        elif st[0] == 'stair':
            top, foot = st[1], st[2]
            a = top if cur is None or math.dist(cur, top) < math.dist(cur, foot) else foot
            b = foot if a is top else top
            if cur is not None: flat += math.hypot(a[0] - cur[0], a[2] - cur[2]); legs.append(('flat', cur, a))
            stair += math.dist(a, b); legs.append(('stair', a, b)); cur = b
        elif st[0] == 'ladder':
            dy = st[1]; v = UP if dy > 0 else DOWN
            t = abs(dy) / v + 2 * MOUNT
            lad += t; lad_c += abs(dy) / (v * CAS_LADDER) + 2 * MOUNT
            nxt = [cur[0], cur[1] + dy, cur[2]]; legs.append(('ladder', cur, nxt)); cur = nxt
        elif st[0] == 'fixed':
            fixed += st[1]
    extra = (STAND if start_seated else 0) + (SIT if end_seated else 0) + fixed
    walk = flat / WALK + stair / (STAIR * WALK) + lad + extra
    run = flat / RUN + stair / (STAIR * RUN) + lad + extra
    cas = flat / CASUALTY + stair / (STAIR * CASUALTY) + lad_c + extra
    return dict(path=flat + stair + sum(abs(s[1]) for s in steps if s[0] == 'ladder'), walk=walk, run=run, cas=cas, legs=legs)

def stair(fid):
    f = fx[fid]; return ('stair', f['top_m'], f['foot_m'])

# T2's stairs are in the layout (reference-ship-tern, applied 2026-10-04): S reads them there.
S = stair
pt = lambda name: ('pt', P(name))
# Stated detour points: (x, floor y, z), the compartment each must lie in, and the patch it needs.
DETOUR = {
 'bunk': ([5.1, 0.0, 13.0], 'quarters', ''),
 'mess_table': ([-5.1, 0.0, 13.0], 'mess', ''),
 'trunk_a': ([0, 3.5, 11.0], 'a_corridor', ''),            # the ladder trunk, at each deck's floor
 'trunk_b': ([0, 0.0, 11.0], 'b_spine', ''),
 'trunk_c': ([0, -3.5, 11.0], 'c_spine', ''),
 'gallery_fwd_p': ([5.6, 0, -1.5], 'hangar', 'T1'),        # the port gallery's forward end, once T1 runs it to z = 0
 'beside_petrel': ([2.8, -3.5, -10.0], 'hangar', ''),
 'reactor_floor': ([0, -3.5, -21.6], 'engineering', ''),   # engineering's lower floor, forward of the reactor
 'tube1_breech': ([2.0, 0.0, 30.2], 'torpedo_room', ''),
 'bridge_centre': ([0, 3.5, 25.0], 'bridge', ''),
 'under_dorsal_pod': ([0, 3.5, 2.0], 'dorsal_turret', ''), # below the dorsal pod's hatch
 'over_ventral_pod': ([0, -3.5, 3.0], 'c_spine', ''),      # above the ventral pod's hatch
}
D = lambda name: ('pt', DETOUR[name][0])
TRUNK_A, TRUNK_B, TRUNK_C = (DETOUR[k][0] for k in ('trunk_a', 'trunk_b', 'trunk_c'))
CHECK = []                                 # (label, route): every route printed, for the check below

def bridge_to_trunk(st):
    return [pt(st), pt('p_bridge_aft'), ('pt', TRUNK_A)]

R = {}
# Bridge to engineering, two ways
R['Helm to engineering bay console, by the aft passage (T2)'] = route([pt('helm'), pt('p_bridge_aft'), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), pt('eng_main')], True, True)
R['Helm to engineering bay console, through the hangar (T1)'] = route(bridge_to_trunk('helm') + [('ladder', -3.5), pt('p_spine_hangar'), D('gallery_fwd_p'), pt('p_gallery_eng_p'), pt('eng_main')], True, True)
R['Helm to the reactor lower floor (today)'] = route(bridge_to_trunk('helm') + [('ladder', -7.0), pt('p_hangar_c'), pt('p_hangar_eng'), D('reactor_floor')], True, False)
# Quarters
Q = DETOUR['bunk'][0]
R['Quarters to helm (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt('helm')], False, True)
R['Mess to helm (today)'] = route([D('mess_table'), pt('p_mess'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt('helm')], False, True)
R['Quarters to bay control on the hangar landing (today)'] = route([('pt', Q), pt('p_quarters'), pt('p_spine_hangar'), pt('bay_control')], False, True)
R['Quarters to the hangar floor beside the Petrel (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c'), D('beside_petrel')], False, False)
# Damage control to the switchboards
R['Damage control board to the forward switchboard (today)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_switchboard'), pt('aux_switchboard')], True, False)
R['Damage control board to the battery bank (today)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_switchboard'), pt('battery_bank')], True, False)
R['Damage control board to the main switchboard, through the hangar (T1)'] = route([pt('damage_board'), pt('p_damage_control'), pt('p_spine_hangar'), D('gallery_fwd_p'), pt('p_gallery_eng_p'), pt('main_switchboard')], True, False)
R['Damage control board to the main switchboard, by the aft passage (T2)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), pt('main_switchboard')], True, False)
# Magazine
R['Magazine racks to tube 1 breech, on foot (today)'] = route([pt('magazine_racks'), pt('p_magazine'), ('pt', TRUNK_C), ('ladder', 3.5), pt('p_torpedo'), D('tube1_breech')], False, False)
# Medbay with a casualty
R['Bridge centre to a medbay bed (today)'] = route([D('bridge_centre'), pt('p_bridge_aft'), ('pt', TRUNK_A), ('ladder', -3.5), pt('p_medbay'), pt('medbay_beds')], False, False)
R['Engineering bay console to a medbay bed (T1)'] = route([pt('eng_main'), pt('p_gallery_eng_p'), D('gallery_fwd_p'), pt('p_spine_hangar'), pt('p_medbay'), pt('medbay_beds')], True, False)

for k, v in R.items():
    print(f"{k:75s} path {v['path']:5.1f} m  walk {v['walk']:5.1f} s  run {v['run']:5.1f} s  casualty {v['cas']:5.1f} s")
    CHECK.append((k, v))

print()
# Any station to the launch bays: hangar side of each bay's pressure door. Best of the listed routes.
BAYP, BAYS = pt('p_bay_p'), pt('p_bay_s')
def to_bay(prefix, seated=True):
    out = []
    for bay in (BAYP, BAYS):
        best = None
        for r in prefix:
            v = route(r + [bay], seated, False)
            if best is None or v['walk'] < best['walk']: best = v
        out.append(best)
    return out
def via_c_from_bridge(st): return [bridge_to_trunk(st) + [('ladder', -7.0), pt('p_hangar_c')]]
rows = {}
for st in ['captain', 'helm', 'tactical', 'engineering', 'science', 'comms', 'flight_ops']:
    rows[st] = to_bay(via_c_from_bridge(st))
rows['eng_main (T2)'] = to_bay([[pt('eng_main'), pt('p_gallery_eng_p'), S('hangar_stair_p')], [pt('eng_main'), S('eng_stair_lower'), pt('p_hangar_eng')]])
rows['damage_board'] = to_bay([[pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['bay_control (today, by the trunk)'] = to_bay([[pt('bay_control'), pt('p_spine_hangar'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['bay_control (T1, T2, by the gallery stair)'] = to_bay([[pt('bay_control'), D('gallery_fwd_p'), S('hangar_stair_p')]])
rows['gunner_dorsal'] = to_bay([[pt('gunner_dorsal'), ('ladder', -3.5), D('under_dorsal_pod'), pt('p_dorsal_fwd'), ('pt', TRUNK_A), ('ladder', -7.0), pt('p_hangar_c')]])
rows['gunner_ventral'] = to_bay([[pt('gunner_ventral'), ('ladder', 3.0), D('over_ventral_pod'), pt('p_hangar_c')]])
rows['gunner_port'] = to_bay([[pt('gunner_port'), ('fixed', HATCH_SIDE, 'hatch'), pt('p_pod_port'), pt('p_port_turret'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['gunner_stbd'] = to_bay([[pt('gunner_stbd'), ('fixed', HATCH_SIDE, 'hatch'), pt('p_pod_stbd'), pt('p_stbd_turret'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['quarters'] = to_bay([[('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]], False)
for k, (p_, s_) in rows.items():
    print(f"{k:45s} port: {p_['path']:5.1f} m walk {p_['walk']:5.1f} run {p_['run']:5.1f} | stbd: {s_['path']:5.1f} m walk {s_['walk']:5.1f} run {s_['run']:5.1f}")
    CHECK += [(k + ', to the port bay', p_), (k + ', to the starboard bay', s_)]

print()
for st in ['helm','tactical','engineering','science','captain']:
    v = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt(st)], False, True)
    print('quarters to', st, round(v['walk'],1), round(v['run'],1))
    CHECK.append(('quarters to ' + st, v))

print()
# Routes reference-ship-tern section 6 quotes beyond crew-on-deck's table (added 2026-10-04 when
# reference-ship-tern adopted these speeds, its question T5). Same waypoints and rules as above.
RP = fx['reactor_panel']['center_m']
E = {}
E['Quarters to captain (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt('captain')], False, True)
E['Quarters to a medbay bed (today)'] = route([('pt', Q), pt('p_quarters'), pt('p_medbay'), pt('medbay_beds')], False, False)
E['Helm to the reactor panel, by the aft passage (T2)'] = route([pt('helm'), pt('p_bridge_aft'), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), ('pt', RP)], True, False)
E["Helm to engineering's catwalk door (today)"] = route([pt('helm'), pt('p_bridge_aft'), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng')], True, False)
E['Helm to engineering bay console by the hangar floor and gallery stair (T2)'] = route(bridge_to_trunk('helm') + [('ladder', -7.0), pt('p_hangar_c'), S('hangar_stair_p'), pt('p_gallery_eng_p'), pt('eng_main')], True, True)
E['Quarters to the port pod seat (today)'] = route([('pt', Q), pt('p_quarters'), pt('p_port_turret'), pt('p_pod_port'), ('fixed', HATCH_SIDE, 'hatch'), pt('gunner_port')], False, True)
E['Quarters to the starboard pod seat (today)'] = route([('pt', Q), pt('p_quarters'), pt('p_stbd_turret'), pt('p_pod_stbd'), ('fixed', HATCH_SIDE, 'hatch'), pt('gunner_stbd')], False, True)
E['Quarters to the dorsal pod seat (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_dorsal_fwd'), D('under_dorsal_pod'), ('ladder', 3.5), pt('gunner_dorsal')], False, True)
E['Quarters to the ventral pod seat (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', -3.5), D('over_ventral_pod'), ('ladder', -3.0), pt('gunner_ventral')], False, True)
E['Helm to the dorsal pod seat (today)'] = route([pt('helm'), pt('p_bridge_aft'), pt('p_dorsal_fwd'), D('under_dorsal_pod'), ('ladder', 3.5), pt('gunner_dorsal')], True, True)
E['Helm to the port or starboard pod seat (today)'] = route(bridge_to_trunk('helm') + [('ladder', -3.5), pt('p_port_turret'), pt('p_pod_port'), ('fixed', HATCH_SIDE, 'hatch'), pt('gunner_port')], True, True)
E['Helm to the ventral pod seat (today)'] = route(bridge_to_trunk('helm') + [('ladder', -7.0), D('over_ventral_pod'), ('ladder', -3.0), pt('gunner_ventral')], True, True)
for k, v in E.items():
    print(f"{k:75s} path {v['path']:5.1f} m  walk {v['walk']:5.1f} s  run {v['run']:5.1f} s")
    CHECK.append((k, v))
# A suited damage control team (crew-on-deck section 10: 1.5 m/s, no running) from the suit lockers to
# the port main switchboard section by the aft passage: damage-control's walkthrough W1.
SUITED = 1.5
_walk, WALK = WALK, SUITED
v = route([('pt', fx['eva_suits']['center_m']), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), pt('main_switchboard')], False, False)
WALK = _walk
print(f"{'Suited team, suit lockers to the port main switchboard, aft passage (T2)':75s} path {v['path']:5.1f} m  suited {v['walk']:5.1f} s")
CHECK.append(('Suited team, suit lockers to the port main switchboard, aft passage (T2)', v))

# ------------------------------------------------------------------ the route check
STEP_M, ABOVE_M, NEAR_M = 0.05, 0.3, 0.02   # sample spacing; height above the floor; a crossing's reach
SLAB = float(L.get('conventions', {}).get('deck_slab_m', 0.5))
T1_GALLERY_FWD_Z = 0.0                      # T1: the galleries' forward ends move to z = 0

def plan(t1):
    """Every brush as (compartment, polygon, floor, ceiling); with T1, each hangar gallery (a hangar
    brush on deck B's floor) has its forward corners moved forward to T1_GALLERY_FWD_Z."""
    out = []
    for c in L['compartments']:
        for b in c['brushes']:
            poly = [tuple(p) for p in b['poly']]
            if t1 and c['id'] == 'hangar' and b['y'][0] > -1e-6:
                zs = [z for _, z in poly]; mid = (min(zs) + max(zs)) / 2; dz = T1_GALLERY_FWD_Z - max(zs)
                poly = [(x, z + dz if z > mid else z) for x, z in poly]
            out.append((c['id'], poly, b['y'][0], b['y'][1]))
    return out

def rect_has(cx, cz, sx, sz, x, z, tol=1e-3):
    return abs(x - cx) <= sx / 2 + tol and abs(z - cz) <= sz / 2 + tol

def comps_at(br, x, y, z, strict_y=False):
    """Compartments whose air holds (x, y, z); strict_y leaves out a floor or ceiling it lies on."""
    e = 1e-6 if strict_y else -1e-6
    return {cid for cid, poly, y0, y1 in br if y0 + e <= y <= y1 - e and inside_poly(poly, x, z)}

def has_floor(br, x, y, z):
    """A floor within 0.6 m below a walking point (a seat stands up to 0.5 m above its floor):
    a brush's floor, or a floor a fixture provides (landing, catwalk, dais, the mezzanine outside its ring)."""
    lo, hi = y - 0.6, y + 0.05
    if any(lo <= y0 <= hi and inside_poly(poly, x, z) for _, poly, y0, _ in br):
        return True
    for f in L['fixtures']:
        c = f['center_m']; top = c[1] + f.get('height_m', 0.0)
        if f['kind'] not in ('landing', 'catwalk', 'dais', 'mezzanine') or not lo <= top <= hi:
            continue
        if f.get('poly'):
            ring = f.get('ring_inner_radius_m', 0.0)
            if inside_poly([tuple(p) for p in f['poly']], x, z) and math.hypot(x - c[0], z - c[2]) >= ring:
                return True
        elif rect_has(c[0], c[2], f['size_m'][0], f['size_m'][1], x, z):
            return True
    return False

def through_portal(a_set, b_set, x, y, z, vertical):
    """A portal joins a compartment of a_set to one of b_set at (x, z): a floor portal whose opening
    holds it (vertical), or a wall portal whose opening holds it at walking height y."""
    for p in L['portals']:
        bt = set(p['between'])
        if not (bt & a_set and bt & b_set) or floor_portal(p) != vertical:
            continue
        c = p['center_m']; w, h = p['size_m']
        if vertical:
            if rect_has(c[0], c[2], w, h, x, z): return True
            continue
        n = p['normal']; tx, tz = -n[2], n[0]
        if abs((x - c[0]) * n[0] + (z - c[2]) * n[2]) <= 0.05 and abs((x - c[0]) * tx + (z - c[2]) * tz) <= w / 2 + NEAR_M \
           and c[1] - h / 2 - 0.05 <= y - ABOVE_M <= c[1] + h / 2:
            return True
    return False

def check_leg(br, kind, a, b):
    """Problems (failures) and notes on one leg."""
    bad, notes = [], []
    n = max(1, int(math.dist(a, b) / STEP_M))
    pts = [[a[k] + (b[k] - a[k]) * i / n for k in range(3)] for i in range(n + 1)]
    where = lambda p: f"({p[0]:.2f}, {p[1]:.2f}, {p[2]:.2f})"
    if kind == 'ladder':
        prev = None
        for p in pts:
            y = p[1] + ABOVE_M
            here = comps_at(br, p[0], y, p[2], strict_y=True)
            for q in L['portals']:
                if floor_portal(q) and abs(y - q['center_m'][1]) <= SLAB + 1e-6 and rect_has(q['center_m'][0], q['center_m'][2], *q['size_m'], p[0], p[2]):
                    here |= set(q['between']) - {'space'}
            if not here:
                bad.append(f"the ladder leaves the air at {where(p)}"); break
            if prev and not (prev & here) and not through_portal(prev, here, p[0], y, p[2], True):
                bad.append(f"the ladder climbs through a solid floor at {where(p)}"); break
            prev = here
        return bad, notes
    solids = [s for s in L['systems'] if s.get('radius_m')]   # the reactor: a cylinder no one walks through
    for p in pts:
        y = p[1] + ABOVE_M
        if not comps_at(br, p[0], y, p[2]):
            bad.append(f"leaves the air at {where(p)}"); break
        if kind == 'flat' and not has_floor(br, p[0], p[1], p[2]):
            bad.append(f"has no floor under it at {where(p)}"); break
        hit = next((s for s in solids if abs(y - s['center_m'][1]) <= s.get('height_m', 0) / 2
                    and math.hypot(p[0] - s['center_m'][0], p[2] - s['center_m'][2]) < s['radius_m']), None)
        if hit:
            bad.append(f"passes through the {hit['name'].lower()} at {where(p)}"); break
    for cr in L.get('craft', []):
        c = cr['center_m']
        hit = [p for p in pts if rect_has(c[0], c[2], cr['span_m'], cr['length_m'], p[0], p[2], 0.0)
               and c[1] - cr['height_m'] / 2 - 0.1 <= p[1] + ABOVE_M <= c[1] + cr['height_m'] / 2]
        if hit:
            notes.append(f"crosses the parked {cr['name']}'s footprint for {math.dist(hit[0], hit[-1]):.1f} m")
    # Walls: wherever the leg crosses a brush's edge, the air either side must share a compartment
    # (an open face) or be joined there by a wall portal.
    dx, dz = b[0] - a[0], b[2] - a[2]; ln = math.hypot(dx, dz)
    for cid, poly, y0, y1 in br:
        for e0, e1 in edges(poly):
            fx_, fz_ = e1[0] - e0[0], e1[1] - e0[1]
            den = dx * fz_ - dz * fx_
            if abs(den) < 1e-12: continue
            t = ((e0[0] - a[0]) * fz_ - (e0[1] - a[2]) * fx_) / den
            u = ((e0[0] - a[0]) * dz - (e0[1] - a[2]) * dx) / den
            if not (1e-6 < t < 1 - 1e-6 and -1e-6 <= u <= 1 + 1e-6): continue
            x, z = a[0] + dx * t, a[2] + dz * t; y = a[1] + (b[1] - a[1]) * t + ABOVE_M
            if not y0 <= y <= y1: continue
            ux, uz = dx / ln * NEAR_M, dz / ln * NEAR_M
            before, after = comps_at(br, x - ux, y, z - uz), comps_at(br, x + ux, y, z + uz)
            if before & after or not before or not after: continue
            if not through_portal(before, after, x, y, z, False):
                bad.append(f"cuts the wall between {'/'.join(sorted(before))} and {'/'.join(sorted(after))} at ({x:.2f}, {y - ABOVE_M:.2f}, {z:.2f})")
    return bad, notes

PLANS = {False: plan(False), True: plan(True)}
fails, notes = [], []
for name, (xyz, cid, needs) in DETOUR.items():
    br = PLANS[needs == 'T1']
    if cid not in comps_at(br, xyz[0], xyz[1] + ABOVE_M, xyz[2]):
        fails.append(f"detour point {name} {xyz} is not inside {cid}{' (with T1)' if needs else ''}")
for label, v in CHECK:
    br = PLANS['T1' in label]
    for kind, a, b in v['legs']:
        if math.dist(a, b) < 1e-9: continue
        bad, nt = check_leg(br, kind, a, b)
        fails += [f"{label}: {kind} leg {[round(c, 2) for c in a]} to {[round(c, 2) for c in b]} {m}" for m in bad]
        notes += [f"{label}: leg {[round(c, 2) for c in a]} to {[round(c, 2) for c in b]} {m}" for m in nt]
print()
print(f"Route check: {len(DETOUR)} detour points, {len(CHECK)} routes, {sum(len(v['legs']) for _, v in CHECK)} legs"
      f" (routes marked T1 checked with the galleries run forward to z = {T1_GALLERY_FWD_Z:g})")
for m in dict.fromkeys(notes): print('  note', m)
for m in dict.fromkeys(fails): print('  FAIL', m)
if not fails: print('  ok')
sys.exit(1 if fails else 0)
