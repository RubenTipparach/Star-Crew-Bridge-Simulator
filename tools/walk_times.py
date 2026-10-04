#!/usr/bin/env python3
"""Walk times between key places on the Tern, from data/ships/tern/layout.json.

A measurement instrument for openspec/changes/crew-on-deck (CLAUDE.md section 4): it prints the
walk-time table that change's design quotes (task 1.3). Every waypoint is a layout point (a seat,
a portal centre at floor height, a system, a fixture) or a stated detour point, and the speeds are
the design's proposed values. Standard library only.

Usage: python3 tools/walk_times.py
"""
import json, math, os, sys
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
L = json.load(open(os.path.join(ROOT, 'data', 'ships', 'tern', 'layout.json'), encoding='utf-8'))
WALK, RUN, STAIR = 1.8, 4.0, 0.7
UP, DOWN, MOUNT = 0.8, 1.0, 0.5          # ladder m/s up, down; s to get on and to get off
STAND, SIT = 0.9, 0.4                     # hold 0.5 s + 0.4 s clip; 0.4 s clip
HATCH_SIDE = 1.5                          # climb through a side hatch
PRESSURE_DOOR = 2.0
CASUALTY, CAS_LADDER = 1.2, 0.5           # carrying a downed body: m/s flat; ladder speed factor

seat = {s['id']: s['seat_m'] for s in L['stations']}
portal = {}
for p in L['portals']:
    c = p['center_m']; ax = p['axis']; sz = p['size_m']
    floor = c[1] - sz[1] / 2 if ax in ('x', 'z') else c[1]
    portal[p['id']] = [c[0], floor, c[2]]
system = {s['id']: s['center_m'] for s in L['systems']}
fx = {f['id']: f for f in L['fixtures']}

def P(name):
    if name in seat: return seat[name]
    if name in portal: return portal[name]
    if name in system: return system[name]
    raise KeyError(name)

def route(steps, start_seated=False, end_seated=False, casualty=False):
    """steps: list of ('pt', xyz) | ('stair', top, foot) | ('ladder', dy_m) | ('fixed', s, why)."""
    flat = stair = 0.0; fixed = 0.0; lad = 0.0; lad_c = 0.0; cur = None
    for st in steps:
        if st[0] == 'pt':
            p = st[1]
            if cur is not None: flat += math.hypot(p[0] - cur[0], p[2] - cur[2])
            cur = p
        elif st[0] == 'stair':
            top, foot = st[1], st[2]
            a = top if cur is None or math.dist(cur, top) < math.dist(cur, foot) else foot
            b = foot if a is top else top
            if cur is not None: flat += math.hypot(a[0] - cur[0], a[2] - cur[2])
            stair += math.dist(a, b); cur = b
        elif st[0] == 'ladder':
            dy = st[1]; v = UP if dy > 0 else DOWN
            t = abs(dy) / v + 2 * MOUNT
            lad += t; lad_c += abs(dy) / (v * CAS_LADDER) + 2 * MOUNT
            cur = [cur[0], cur[1] + dy, cur[2]]
        elif st[0] == 'fixed':
            fixed += st[1]
    extra = (STAND if start_seated else 0) + (SIT if end_seated else 0) + fixed
    walk = flat / WALK + stair / (STAIR * WALK) + lad + extra
    run = flat / RUN + stair / (STAIR * RUN) + lad + extra
    cas = flat / CASUALTY + stair / (STAIR * CASUALTY) + lad_c + extra
    return dict(path=flat + stair + sum(abs(s[1]) for s in steps if s[0] == 'ladder'), walk=walk, run=run, cas=cas)

def stair(fid):
    f = fx[fid]; return ('stair', f['top_m'], f['foot_m'])

# T2 stairs are proposed (reference-ship-tern); not in the layout yet.
T2 = {
 'eng_stair_upper': ([2.0, 3.5, -18.6], [6.0, 0.0, -18.6]),
 'eng_stair_lower': ([-2.0, 0.0, -18.6], [-6.0, -3.5, -18.6]),
 'hangar_stair_p': ([5.0, 0.0, -17.4], [1.3, -3.5, -17.4]),
 'hangar_stair_s': ([-5.0, 0.0, -17.4], [-1.3, -3.5, -17.4]),
}
def S(fid):
    t, f = T2[fid]; return ('stair', t, f)
pt = lambda name: ('pt', P(name))
xyz = lambda x, y, z: ('pt', [x, y, z])
TRUNK_A, TRUNK_B, TRUNK_C = [0, 3.5, 11.0], [0, 0.0, 11.0], [0, -3.5, 11.0]

def bridge_to_trunk(st):
    return [pt(st), pt('p_bridge_aft'), ('pt', TRUNK_A)]

R = {}
# Bridge to engineering, two ways
R['Helm to engineering bay console, by the aft passage (T2)'] = route([pt('helm'), pt('p_bridge_aft'), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), pt('eng_main')], True, True)
R['Helm to engineering bay console, through the hangar (T1)'] = route(bridge_to_trunk('helm') + [('ladder', -3.5), pt('p_spine_hangar'), xyz(5.6, 0, -1.5), pt('p_gallery_eng_p'), pt('eng_main')], True, True)
R['Helm to the reactor lower floor (today)'] = route(bridge_to_trunk('helm') + [('ladder', -7.0), pt('p_hangar_c'), pt('p_hangar_eng'), xyz(0, -3.5, -21.6)], True, False)
# Quarters
Q = [5.1, 0.0, 13.0]
R['Quarters to helm (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt('helm')], False, True)
R['Mess to helm (today)'] = route([('pt', [-5.1, 0, 13.0]), pt('p_mess'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt('helm')], False, True)
R['Quarters to bay control on the hangar landing (today)'] = route([('pt', Q), pt('p_quarters'), pt('p_spine_hangar'), pt('bay_control')], False, True)
R['Quarters to the hangar floor beside the Petrel (today)'] = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c'), xyz(2.8, -3.5, -10.0)], False, False)
# Damage control to the switchboards
R['Damage control board to the forward switchboard (today)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_switchboard'), pt('aux_switchboard')], True, False)
R['Damage control board to the battery bank (today)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_switchboard'), pt('battery_bank')], True, False)
R['Damage control board to the main switchboard, through the hangar (T1)'] = route([pt('damage_board'), pt('p_damage_control'), pt('p_spine_hangar'), xyz(5.6, 0, -1.5), pt('p_gallery_eng_p'), pt('main_switchboard')], True, False)
R['Damage control board to the main switchboard, by the aft passage (T2)'] = route([pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_dorsal_fwd'), pt('p_dorsal_aft'), pt('p_aft_eng'), S('eng_stair_upper'), pt('main_switchboard')], True, False)
# Magazine
R['Magazine racks to tube 1 breech, on foot (today)'] = route([pt('magazine_racks'), pt('p_magazine'), ('pt', TRUNK_C), ('ladder', 3.5), pt('p_torpedo'), xyz(2.0, 0.0, 30.2)], False, False)
# Medbay with a casualty
R['Bridge centre to a medbay bed (today)'] = route([xyz(0, 3.5, 25.0), pt('p_bridge_aft'), ('pt', TRUNK_A), ('ladder', -3.5), pt('p_medbay'), pt('medbay_beds')], False, False)
R['Engineering bay console to a medbay bed (T1)'] = route([pt('eng_main'), pt('p_gallery_eng_p'), xyz(5.6, 0, -1.5), pt('p_spine_hangar'), pt('p_medbay'), pt('medbay_beds')], True, False)

for k, v in R.items():
    print(f"{k:75s} path {v['path']:5.1f} m  walk {v['walk']:5.1f} s  run {v['run']:5.1f} s  casualty {v['cas']:5.1f} s")

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
rows['eng_main (T2)'] = to_bay([[pt('eng_main'), pt('p_gallery_eng_p'), S('hangar_stair_p')], [pt('eng_main'), S('eng_stair_lower'), pt('p_hangar_eng')], [pt('eng_main'), pt('p_gallery_eng_p'), xyz(-0.0, 0, -18.0)][:2] + [S('hangar_stair_p')]])
rows['damage_board'] = to_bay([[pt('damage_board'), pt('p_damage_control'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['bay_control (today, by the trunk)'] = to_bay([[pt('bay_control'), pt('p_spine_hangar'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['bay_control (T1, T2, by the gallery stair)'] = to_bay([[pt('bay_control'), xyz(5.6, 0, -1.5), S('hangar_stair_p')]])
rows['gunner_dorsal'] = to_bay([[pt('gunner_dorsal'), ('ladder', -3.5), xyz(0, 3.5, 2.0), pt('p_dorsal_fwd'), ('pt', TRUNK_A), ('ladder', -7.0), pt('p_hangar_c')]])
rows['gunner_ventral'] = to_bay([[pt('gunner_ventral'), ('ladder', 3.0), xyz(0, -3.5, 3.0), pt('p_hangar_c')]])
rows['gunner_port'] = to_bay([[pt('gunner_port'), ('fixed', HATCH_SIDE, 'hatch'), pt('p_pod_port'), pt('p_port_turret'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['gunner_stbd'] = to_bay([[pt('gunner_stbd'), ('fixed', HATCH_SIDE, 'hatch'), pt('p_pod_stbd'), pt('p_stbd_turret'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]])
rows['quarters'] = to_bay([[('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', -3.5), pt('p_hangar_c')]], False)
for k, (p_, s_) in rows.items():
    print(f"{k:45s} port: {p_['path']:5.1f} m walk {p_['walk']:5.1f} run {p_['run']:5.1f} | stbd: {s_['path']:5.1f} m walk {s_['walk']:5.1f} run {s_['run']:5.1f}")

print()
for st in ['helm','tactical','engineering','science','captain']:
    v = route([('pt', Q), pt('p_quarters'), ('pt', TRUNK_B), ('ladder', 3.5), pt('p_bridge_aft'), pt(st)], False, True)
    print('quarters to', st, round(v['walk'],1), round(v['run'],1))
