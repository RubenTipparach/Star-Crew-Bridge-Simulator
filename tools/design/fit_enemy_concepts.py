"""Fit the retained initial Hound study rooms to their authored hull envelopes.

The catalogue owns the initial polygons, hulls and room roles. This documentation
tool clips only outer edges, records the fitted polygons, and locates simple scale
furnishings in data. --check compares the delivered catalogue with that derivation.
No engine data, mesh or texture is changed.
"""
import argparse
import copy
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / 'data/ships/enemy-concepts/layouts.json'


def clip(poly, envelope, margin):
    """Intersect a convex room with each inward-offset hull edge, in metres."""
    result = copy.deepcopy(poly)
    for i, a in enumerate(envelope):
        b = envelope[(i + 1) % len(envelope)]
        dx, dz = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dz)
        def distance(p):
            return (dx * (p[1] - a[1]) - dz * (p[0] - a[0])) / length - margin
        output = []
        for j, p in enumerate(result):
            q = result[(j + 1) % len(result)]
            dp, dq = distance(p), distance(q)
            if dp >= -1e-8:
                output.append(p)
            if (dp < 0) != (dq < 0):
                t = dp / (dp - dq)
                output.append([p[k] + t * (q[k] - p[k]) for k in range(2)])
        result = output
    clean = []
    for p in result:
        p = [round(v, 6) for v in p]
        if not clean or math.dist(clean[-1], p) > 1e-5:
            clean.append(p)
    if len(clean) > 1 and math.dist(clean[0], clean[-1]) < 1e-5:
        clean.pop()
    return clean


def fit(data):
    """Return the fitted study catalogue, leaving the first-pass plans intact."""
    d = copy.deepcopy(data)
    d['stage'] = 'fitted'
    for s in d['ships']:
        s['fit_margin_m'] = 0.85
        s['layout']['conventions']['deck_slab_m'] = 0.8
        s['furnishings'] = []
        for r in s['layout']['compartments']:
            r['brushes'] = copy.deepcopy(r['initial_brushes'])
            for b in r['brushes']:
                b['poly'] = clip(b['poly'], s['fit_envelopes'][r['envelope']], s['fit_margin_m'])
                if len(b['poly']) < 3:
                    raise ValueError(f"{s['id']}/{r['id']}: hull erased the room")
            b = r['brushes'][0]
            cx = sum(p[0] for p in b['poly']) / len(b['poly'])
            cz = sum(p[1] for p in b['poly']) / len(b['poly'])
            y = b['y'][0]
            def put(kind, center, size):
                s['furnishings'].append({'room':r['id'], 'kind':kind, 'center_m':center, 'size_m':size})
            if r['kind'] == 'corridor':
                if r['id'].endswith('_route'):
                    z0 = min(p[1] for p in b['poly'])
                    z1 = max(p[1] for p in b['poly'])
                    z = z0 + min(2.2, (z1-z0)/2)
                    put('person', [0, y, z], [.5, 1.8, .4])
            elif r['id'] == 'bunks':
                # Two-tier bunk footprints. The layout checker checks the boxes,
                # not an assumed room size, after the hull trims the corners.
                for k in range(math.ceil(s['crew'] / 2)):
                    columns = 2
                    x = cx + (k % columns - .5) * 1.7
                    z = cz + (k // columns - (math.ceil(s['crew']/4)-1)/2) * 2.7
                    put('bunk', [x, y, z], [.9, 1.6, 2.0])
            elif r['id'] == 'bridge':
                cz = (min(p[1] for p in b['poly']) + max(p[1] for p in b['poly'])) / 2
                for row in [-1.0, 1.0]:
                    for x in [-1.0, 1.0]:
                        put('console', [cx+x, y, cz+row+.3], [1.2, 1.1, .65])
                        put('seat', [cx+x, y, cz+row-.7], [.55, .9, .6])
            elif r['id'] == 'medical':
                put('bed', [cx, y, cz], [.9, .8, 2.0])
            elif r['id'] == 'mess':
                put('table', [cx, y, cz], [2.8, .8, 1.1])
            elif r['group'] in ['weapons', 'power']:
                put('machine', [cx, y, cz], [1.4, 1.6, 2.2])
        for z in s['hull_zones']:
            deck = next(v for v in s['layout']['decks'] if v['id'] == z['deck'])
            z['y_m'][0] = min(z['y_m'][0], deck['floor_y_m']-.5)
            rooms = [r for r in s['layout']['compartments'] if z['deck'] in r['decks']]
            z['y_m'][1] = max(z['y_m'][1], max(b['y'][1] for r in rooms for b in r['brushes'])+.5)
    return d


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = json.loads(CATALOG.read_text(encoding='utf-8'))
    fitted = fit(data)
    if args.check:
        if fitted != data:
            raise SystemExit('Fitted concept catalogue is stale; run fit_enemy_concepts.py')
        print('Three fitted concept layouts reproduce from the retained first plans.')
    else:
        CATALOG.write_text(json.dumps(fitted, indent=2)+'\n', encoding='utf-8')
        print('Wrote three fitted layouts; original polygons retained.')


if __name__ == '__main__':
    main()
