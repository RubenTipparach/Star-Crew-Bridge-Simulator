"""Grabon relief designs for the shared panel texture baker.

Interlocking armor, diagonal stays, bronze clamps and ember indicators replace
the Federation panel silhouettes. The caller owns materials, rendering, emission,
palette reduction, UV scale and output validation.
"""
import math


def brace(P,name,a,b,width=.08,depth=.045,role='trim'):
    dx,dy=b[0]-a[0],b[1]-a[1];length=math.hypot(dx,dy)
    nx,ny=-dy/length*width/2,dx/length*width/2
    return P.poly(name,[(a[0]+nx,a[1]+ny),(a[0]-nx,a[1]-ny),
                        (b[0]-nx,b[1]-ny),(b[0]+nx,b[1]+ny)],.008,depth,role,.006)


def glyph(P,x,y,z=.06):
    # Original bar-and-notch marking, modeled as painted strokes without text.
    for i,h in enumerate([.08,.15,.11]):
        P.box('clan_mark',(x+i*.065,y,z),(x+i*.065+.028,y+h,z+.002),'stencil')


def armor(K,P,narrow=False):
    slab=K.plate_slab(P)
    w=.31 if narrow else .80
    shield=P.poly('interlocking_shield',[(-w,.18),(w*.42,.18),(w,.49),(w,1.57),
                                (w*.42,1.82),(-w,1.82),(-w*.68,1.02)],.003,.031,'bulkhead',.008)
    P.union(slab,'joined_armor',[shield])
    brace(P,'shoulder_stay',(-w,1.73),(-w*.55,1.40),.045,.055,'paint2')
    brace(P,'lower_stay',(w,.25),(w*.58,.50),.045,.055,'paint2')
    K.bolts(P,[(-w+.10,.29),(w-.1,1.65)],r=.021,z1=.051)
    return slab


def wall(K,P,F,item):
    slab=armor(K,P,item in ('narrow','flank'))
    if item=='plate':
        glyph(P,.34,.32,.035)
        return
    if item in ('vent','ribbed'):
        P.cut(slab,'deep_angled_intake',[P.recess(-.58,.57,.58,1.52,.085)])
        for i in range(7):
            y=.66+i*.115
            brace(P,'slanted_intake_vane',(-.49,y+.12),(.49,y),.046,.068,'paint2')
        brace(P,'intake_spine',(0,.55),(0,1.55),.08,.085)
    elif item=='pipes':
        P.cut(slab,'service_well',[P.recess(-.59,.28,.59,1.76,.10)])
        for i,x in enumerate([-.39,-.10,.24]):
            P.cyl('pressure_pipe','y',(x,.085),.065,.34,1.7,'trim',sides=12)
            for y in (.55,1.36):P.cyl('armored_coupling','y',(x,.085),.09,y,y+.10,'paint2',sides=8)
        K.cplate(P,'valve_shield',-.49,.88,.48,1.23,.13,.13,.19,'paint2',bevel=.012)
        P.box('pressure_tell',(.27,.98,.192),(.39,1.07,.199),'amber')
    elif item=='hatch':
        hatch=K.cplate(P,'hex_service_hatch',-.61,.32,.61,1.74,.21,.035,.09,'paint2',bevel=.018)
        P.cut(hatch,'hatch_pull',[P.box('pull_cut',(-.23,.91,.045),(.23,1.06,.14),'machinery')])
        for x in (-.59,.46):P.box('hatch_dog',(x,.72,.08),(x+.13,1.35,.14),'trim',bevel=.012)
        P.box('seal_indicator',(.23,1.41,.1),(.4,1.47,.105),'amber')
    elif item=='junction':
        for x in (-.53,.38):P.cyl('feed','y',(x,.035),.045,.22,1.77,'rubber',sides=10)
        K.cplate(P,'junction_shield',-.57,.58,.57,1.46,.18,.025,.14,'paint2',bevel=.014)
        for y in (.73,1.05):
            brace(P,'switch_guard',(-.40,y),(.31,y+.13),.075,.19)
            P.box('status_lens',(.32,y,.15),(.41,y+.07,.16),'amber')
    elif item in ('light','flank','narrow'):
        w=.17 if item!='light' else .24
        K.cplate(P,'recessed_lamp_column',-w-.065,.30,w+.065,1.77,.10,.01,.10,'paint2')
        for i in range(5):
            y=.42+i*.245
            P.poly('ember_lens',[(-w,y),(w*.55,y),(w,y+.055),(w,y+.16),(-w,y+.16)],.102,.12,'light_panel')
            brace(P,'lamp_guard',(-w-.07,y+.19),(w+.07,y+.13),.035,.16)
        if item=='flank':glyph(P,.26,1.1,.04)
    elif item=='screen':
        K.cplate(P,'tactical_screen_hood',-.67,.73,.67,1.75,.18,.02,.075,'paint2',bevel=.012)
        P.box('display',(-.50,.94,.077),(.50,1.565,.083),'screen')
        for i in range(5):
            P.box('hard_key',(-.4+i*.18,.81,.078),(-.29+i*.18,.875,.11),'amber' if i==0 else 'trim',bevel=.006)
        glyph(P,-.38,.42,.04)
    else:raise ValueError(item)


def flat(K,P,F,kind,item):
    slab=K.plate_slab(P)
    if item=='walkway':
        # Staggered broad tread ribs repeat on both axes, with no picture seam.
        for x in (-1.5,-.5,.5,1.5):
            for y in (-.5,.5,1.5,2.5):
                for i in range(4):
                    yy=y-.32+i*.18
                    brace(P,'traction_rib',(x-.35,yy),(x+.35,yy+.15),.055,.019,'paint2')
                P.box('route_inlay',(x-.025,y-.05,.021),(x+.025,y+.05,.025),'safety')
        return
    shield=K.cplate(P,'deck_shield',-.93,.07,.93,1.93,.17,.0,.014,'bulkhead')
    P.union(slab,'joined_deck_plate',[shield])
    if item=='plate':
        for x in (-.64,.64):P.box('plate_dog',(x-.055,.82,.017),(x+.055,1.18,.032),'trim')
        return
    if item in ('grille','grate','vent','drain','fan','ribbed'):
        extent=.55 if item not in ('vent','drain') else .37
        P.cut(slab,'deep_sump',[P.recess(-extent,.43,extent,1.57,.11)])
        P.box('black_sump',(-extent,.43,-.07),(extent,1.57,-.065),'machinery')
        for i in range(9):
            y=.47+i*.12
            brace(P,'diagonal_grate',(-extent+.025,y),(extent-.025,y+.09),.033,.052,'trim')
        if item=='fan':
            K.ring(P,'hex_fan_collar',0,1,.63,.50,.02,.07,'paint2',sides=6)
            P.ngon_prism('hub',0,1,.13,6,.035,.09,'trim')
    elif item in ('pipes','trench','cable_tray'):
        for x in (-.23,0,.23):
            P.cyl('ceiling_conduit','y',(x,.045),.048,0,2,'trim' if item=='pipes' else 'rubber',sides=10)
        for y in (.32,1.55):P.box('conduit_yoke',(-.4,y,.06),(.4,y+.12,.10),'paint2',bevel=.012)
    elif item in ('hatch','access'):
        K.cplate(P,'maintenance_hatch',-.64,.36,.64,1.64,.20,.02,.06,'paint2',bevel=.015)
        brace(P,'hatch_bar',(-.38,.70),(.38,1.3),.09,.12,'trim')
        if kind=='ceiling':P.box('hatch_latch_light',(.35,.6,.065),(.43,.76,.07),'amber')
    elif item=='lamp_surround':
        for y in (.64,1.28):
            P.box('lamp_channel',(-1.05,y,.018),(1.05,y+.08,.050),'paint2')
        for x in (-.75,.75):brace(P,'lamp_yoke',(x-.1,.69),(x+.1,1.30),.065,.062,'trim')
    elif item=='hazard':
        for y in (.25,.75,1.25,1.75):
            brace(P,'landing_marker',(-.55,y),(.15,y+.22),.11,.035,'safety')
            brace(P,'landing_marker',(.15,y+.22),(.55,y+.09),.11,.035,'safety')
    else:raise ValueError(item)


def strip(K,P,kind,item,h):
    slab=K.plate_slab(P,strip=True,role='paint2')
    for ox in (-2,0,2):
        for y in (h*.14,h*.86):
            P.box('bronze_flange',(ox-1.05,y-.012,.01),(ox+1.05,y+.012,.035),'trim')
        if item in ('rib_ends','frame','rib','beam','cove'):
            for x in (-.58,.42):
                brace(P,'diagonal_web',(ox+x-.16,h*.23),(ox+x+.16,h*.77),h*.19,.052,'trim')
                P.box('web_bolt',(ox+x-.025,h*.44,.054),(ox+x+.025,h*.56,.075),'machinery')
        elif item in ('top','pipe','louvre','trench'):
            for y in (h*.36,h*.62):
                P.cyl('linear_conduit','x',(y,.018),min(.028,h*.07),ox-1.05,ox+1.05,'trim',sides=10)
            P.box('pipe_yoke',(ox-.07,h*.24,.028),(ox+.07,h*.75,.065),'paint2')
        else:
            for x in (-.72,-.23,.26,.75):
                K.cplate(P,'angular_splice',ox+x-.18,h*.25,ox+x+.18,h*.75,min(.04,h*.1),.003,.033,'bulkhead')
                if item in ('base','riser','riser_low','step','kick','housing'):
                    P.box('orange_inlay',(ox+x-.13,h*.65,.035),(ox+x+.10,h*.73,.039),'safety')
    return slab


def build_panel(K,P,F,kind,item,D):
    """Author one relief target, including trim rows at their actual height."""
    if kind=='module':wall(K,P,F,item)
    elif kind in ('ceiling','floor'):flat(K,P,F,kind,item)
    elif kind=='strip':strip(K,P,kind,item,.5)
    elif kind in ('trims','platforms','edges'):strip(K,P,kind,item,D[kind]['rows'][item]['h_m'])
    else:raise ValueError(kind)
