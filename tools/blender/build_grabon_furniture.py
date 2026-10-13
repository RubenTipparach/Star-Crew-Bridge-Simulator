"""Grabon interior furniture: armored stands, angular seats and enclosed berths.

Uses the shared hard-surface CSG, cleanup, checks and export pipeline. Metre UVs
take the Grabon panel collection in the review; these are faction silhouettes.
"""
from pathlib import Path
import sys
import bpy
sys.path.insert(0,str(Path(__file__).parent))
from hs_kit import Prop, PropSet, run, frame

ROOT=Path(__file__).resolve().parents[2]


def console():
    p=Prop('grabon_console','Grabon tactical lectern','floor at center')
    body=p.hull('armored_lectern',[(x,y,z) for y,w,back,front in [(0,.43,-.26,.30),(.15,.46,-.26,.30),(.77,.30,-.18,.19),(1.07,.63,-.28,.28)] for x in (-w,w) for z in (back,front)],'bulkhead')
    p.chamfer(body,'lectern_edges',.025,lambda m,d,n1,n2:m.y>.8)
    # Working face slopes toward the seated operator at +Z.
    screen=p.hull('display_recess',[(x,y,z) for x in (-.48,.48) for y,z in [(1.035,-.20),(1.12,-.20),(.99,.22),(1.12,.22)]],'light_panel')
    p.cut(body,'inset_tactical_display',[screen])
    pieces=[]
    for side in (-1,1):
        x=side*.54
        pieces.append(p.box('bronze_hand_grip',(x-.035,.91,.23),(x+.035,1.11,.32),'trim'))
    p.union(body,'hands_on_lectern',pieces)
    p.body=body;p.operators=[[0,0,.77]]
    def interface(D):
        m=frame((0,1.0125,.01),83.8845)
        for side in (-1,1):
            for i in range(5):
                x=side*(.12+i*.057)
                h=.045+(.10 if i%3==0 else .055)
                D.paint(m,[(x-.012,-.12),(x+.012,-.12),(x+.012,-.12+h),(x-.012,-.12+h)],'led_amber')
        D.paint(m,[(-.045,.02),(0,.14),(.045,.02),(.015,.04),(0,.095),(-.015,.04)],'led_amber')
        for x in (-.35,-.20,.20,.35):D.paint(m,[(x-.04,.125),(x+.04,.125),(x+.04,.15),(x-.04,.15)],'copper')
    p.decor.append(interface)
    return p


def chair():
    p=Prop('grabon_chair','Grabon armored command seat','floor at center')
    body=p.hull('splayed_base',[(x,y,z) for y,w,d in [(0,.30,.25),(.08,.29,.24),(.39,.20,.18)] for x in (-w,w) for z in (-d,d)],'machinery')
    seat=p.box('seat_pan',(-.28,.37,-.25),(.28,.47,.28),'bulkhead')
    back=p.prism('faceted_back',[(-.29,.43),(.29,.43),(.29,.99),(.20,1.19),(-.20,1.19),(-.29,.99)],'z',-.27,-.14,'bulkhead')
    pieces=[seat,back,p.box('leather_seat',(-.245,.46,-.18),(.245,.51,.24),'upholstery')]
    pieces.append(p.prism('back_cushion',[(-.21,.55),(.21,.55),(.21,.96),(.13,1.09),(-.13,1.09),(-.21,.96)],'z',-.15,-.105,'upholstery_panel'))
    for side in (-1,1):
        x=side*.29
        pieces.append(p.box('armored_arm',(x-.04,.43,-.14),(x+.04,.68,.20),'trim'))
        pieces.append(p.box('arm_pad',(x-.045,.675,-.10),(x+.045,.71,.23),'upholstery_panel'))
    p.union(body,'seat_assembly',pieces);p.body=body;p.seat=[0,.51,.04];p.operators=[[0,0,.04]]
    return p


def bunk():
    p=Prop('grabon_bunk','Two armored sleeping alcoves','floor at center')
    body=p.box('rear_spine',(-.47,0,-1.05),(.47,1.95,-.94),'bulkhead')
    pieces=[]
    for y in (.27,1.19):
        pieces.append(p.box('sleeping_shelf',(-.47,y,-1.0),(.47,y+.1,1.04),'machinery'))
        pieces.append(p.box('padded_roll',(-.40,y+.08,-.83),(.40,y+.21,.91),'upholstery_panel'))
        pieces.append(p.box('head_cushion',(-.38,y+.18,-.83),(.38,y+.30,-.55),'upholstery'))
    for side in (-1,1):
        x=side*.43
        pieces.append(p.box('front_stanchion',(x-.04,.0,.87),(x+.04,1.95,1.02),'trim'))
        pieces.append(p.box('upper_shield',(x-.045,1.75,-.98),(x+.045,1.95,.92),'bulkhead'))
    p.union(body,'berth_assembly',pieces);p.body=body
    return p


def table():
    p=Prop('grabon_table','Grabon communal trestle','floor at center')
    top=[(-1.34,-.40),(-1.13,-.56),(1.13,-.56),(1.34,-.40),(1.34,.40),(1.13,.56),(-1.13,.56),(-1.34,.40)]
    body=p.prism('octagonal_slab',top,'y',.69,.79,'bulkhead')
    pieces=[]
    for x in (-.87,.87):
        pieces.append(p.hull('splayed_trestle',[(x+dx,y,z) for y,half in [(0,.40),(.72,.23)] for dx in (-.065,.065) for z in (-half,half)],'trim'))
    p.union(body,'trestle_assembly',pieces);p.body=body
    return p


def faction(builder):
    def build():
        p=builder()
        p.finish_of.update({'bulkhead':'grabon_armor','machinery':'grabon_housing','trim':'grabon_bronze','light_panel':'grabon_glass'})
        return p
    return build


PROPS={'grabon_console':faction(console),'grabon_chair':faction(chair),'grabon_bunk':faction(bunk),'grabon_table':faction(table)}
SET=PropSet('grabon_furniture','GrabonFurniture',str(ROOT/'assets/models/grabon_furniture'),
    'tools/blender/build_grabon_furniture.py',PROPS,
    {'grabon_console':420,'grabon_chair':360,'grabon_bunk':420,'grabon_table':160},
    'Grabon review furniture; uses the faction Blender-baked panel collection.',
    ['Metres, +Y up, +Z toward the operator. Shared CSG checks and deterministic GLB export.'],__doc__)
if __name__=='__main__':run(SET)
