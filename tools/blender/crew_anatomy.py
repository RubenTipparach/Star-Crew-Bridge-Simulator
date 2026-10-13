"""Connected garment anatomy and fitted hair for the headless crew builder.

The authored JSON owns silhouette tuning. Ring bridges share vertex indices at
shoulders and the crotch so bending cannot pull separate capped limbs apart.
"""
import json
import math
from pathlib import Path

from mathutils import Vector
from crew_rig import blend, limb_weights, torso_weights

ANATOMY_TABLE = Path(__file__).resolve().parents[2] / "data/crew/anatomy.json"


def load_anatomy():
    """Read and reject invalid anatomy tuning before any mesh is built."""
    data = json.loads(ANATOMY_TABLE.read_text())
    keys = "schema _doc variants torso_sides limb_sides skin_inset_m hem_above_hip_m hem_band_m waist_overlap_m armpit_below_shoulder_m shoulder_rounding_m hip_drop_m hip_joint_drop_m crotch_drop_m arm_outward_fraction arm_profile leg_profile ankle_height_boot_fraction boot_toe_length_fraction boot_heel_length_fraction boot_toe_height_m boot_heel_height_m boot_instep_height_m boot_ankle_width_fraction boot_ankle_depth_fraction boot_footprint hair_clearance_m hair_rim_m hairline hair_rows hair_front_span shoulder_skin_weight underarm_skin_weight shoulder_clavicle_weight shoulder_socket_fractions shoulder_relax_strength shoulder_relax_iterations inner_sleeve_support_weight inner_sleeve_support_end covered_arm_ring_fractions hip_skin_weight thigh_root_weight shoulder_root_weight shoulder_outset_m covered_arm_radius_fraction".split()
    if set(data) != set(keys) or data["schema"] != "starcrew.crew-anatomy/1":
        raise ValueError(f"{ANATOMY_TABLE}: unknown or missing anatomy fields")
    for key, value in data.items():
        if type(value) in (int, float) and (not math.isfinite(value) or value <= 0):
            raise ValueError(f"{ANATOMY_TABLE}:{key}: expected a finite positive number")
    if data["torso_sides"] != 12 or data["limb_sides"] != 8:
        raise ValueError("Connected shoulder and pelvis topology requires 12/8 sides")
    for key in ("shoulder_skin_weight", "underarm_skin_weight", "shoulder_clavicle_weight", "shoulder_relax_strength", "inner_sleeve_support_weight", "inner_sleeve_support_end"):
        if not 0 < data[key] < 1:
            raise ValueError(f"{key}: expected a fraction between zero and one")
    if data["shoulder_skin_weight"] + data["shoulder_clavicle_weight"] > 1 or data["underarm_skin_weight"] > data["shoulder_skin_weight"]:
        raise ValueError("Invalid shoulder influence distribution")
    if type(data["shoulder_relax_iterations"]) is not int or not 1 <= data["shoulder_relax_iterations"] <= 8:
        raise ValueError("shoulder_relax_iterations must be an integer in 1-8")
    fractions = data["shoulder_socket_fractions"]
    if not fractions or fractions != sorted(set(fractions)) or any(not 0 < t < 1 for t in fractions):
        raise ValueError("shoulder_socket_fractions must increase between zero and one")
    inner = data["covered_arm_ring_fractions"]
    if not inner or inner != sorted(set(inner)) or not set(inner) <= {t for t,_ in data["arm_profile"]} or inner[0] != data["arm_profile"][0][0] or inner[-1] != 1:
        raise ValueError("covered_arm_ring_fractions must retain valid shoulder and wrist endpoints")
    if set(data["variants"]) != {"male", "female"}:
        raise ValueError("Expected male and female anatomy variants")
    for name, variant in data["variants"].items():
        if set(variant) != set("shoulders chest_depth waist hips arms legs neck head_width".split()) or any(not .5 <= n <= 1.5 for n in variant.values()):
            raise ValueError(f"Invalid {name} anatomy multipliers")
    for key in ("arm_profile", "leg_profile"):
        profile = data[key]
        if not profile or [p[0] for p in profile] != sorted(set(p[0] for p in profile)) or profile[-1][0] != 1 or any(not 0 < t <= 1 or not 0 < r <= 2 for t, r in profile):
            raise ValueError(f"Invalid {key} rings")
    if len(data["boot_footprint"]) != 8 or any(len(v) != 2 or any(not -1 <= n <= 1 for n in v) for v in data["boot_footprint"]):
        raise ValueError("Invalid boot footprint")
    if set(data["hairline"]) != {"crop", "swept", "bob"} or any(len(v) != 2 or any(not 0 < n < 1 for n in v) for v in data["hairline"].values()):
        raise ValueError("Invalid hairline")
    return data


def bridge(s, a, b, colour, part):
    """Join equally sized loops using shared boundary vertices."""
    for i in range(len(a)):
        j = (i + 1) % len(a)
        s.face((a[i], a[j], b[j], b[i]), colour, part)


def align_loop(s, boundary, loop):
    """Match a circular limb ring to the ordered armhole without a twist."""
    choices = []
    for ring in (loop, list(reversed(loop))):
        for offset in range(len(ring)):
            candidate = ring[offset:] + ring[:offset]
            error = sum((Vector(s.vertices[a])-Vector(s.vertices[b])).length_squared for a,b in zip(boundary,candidate))
            choices.append((error,candidate))
    return min(choices,key=lambda item:item[0])[1]


def ring(s, center, rx, ry, weights, sides=12):
    """A horizontal ellipse with consistent circumference order."""
    return [s.vertex((center[0]+rx*math.cos(math.tau*i/sides),center[1]+ry*math.sin(math.tau*i/sides),center[2]),weights) for i in range(sides)]


def relax_socket(s, selected, tuning):
    """Round the local socket using quad edges while preserving its outer boundary."""
    neighbours = {vi:set() for vi in selected}
    for face in s.faces:
        for a,b in zip(face,face[1:]+face[:1]):
            if a in neighbours:
                neighbours[a].add(b)
            if b in neighbours:
                neighbours[b].add(a)
    for _ in range(tuning["shoulder_relax_iterations"]):
        updates = {}
        for vi,adjacent in neighbours.items():
            mean = sum((Vector(s.vertices[j]) for j in sorted(adjacent)),Vector())/len(adjacent)
            updates[vi] = Vector(s.vertices[vi]).lerp(mean,tuning["shoulder_relax_strength"])
        for vi,point in updates.items():
            s.vertices[vi] = tuple(point)


def build_clothes(s, a, p, g, c, tuning, variant, settings, width, hip, shoulder, neck_bottom, skin, tunic):
    """Build a welded tunic and trousers, returning joint and hand landmarks."""
    chest_x = a["chest_width"]*p["shoulder_width_scale"]*width*variant["shoulders"]/2
    chest_y = a["chest_depth"]*width*variant["chest_depth"]/2
    waist_x = a["waist_width"]*p["waist_width_scale"]*width*variant["waist"]/2
    waist_y = a["waist_depth"]*width/2
    neck_r = a["neck_radius"]*variant["neck"]
    hip_x = a["hip_width"]*width*variant["hips"]/2
    hem = hip+tuning["hem_above_hip_m"]
    arm_z = shoulder-tuning["armpit_below_shoulder_m"]
    top_z = shoulder+tuning["shoulder_rounding_m"]
    body_weights = lambda point: torso_weights(point[2],hip,shoulder)
    levels = [(hem,waist_x,waist_y),(hem+tuning["hem_band_m"],waist_x,waist_y),
              ((hem+arm_z)/2,(waist_x+chest_x)/2,(waist_y+chest_y)/2),
              (arm_z,chest_x,chest_y),((arm_z+top_z)/2,chest_x,chest_y),
              (top_z,chest_x-g["clearance"],chest_y-g["clearance"]),
              (neck_bottom-g["collar_height"]/3,neck_r+g["collar_thickness"],neck_r+g["collar_thickness"]),
              (neck_bottom,neck_r+g["collar_thickness"],neck_r+g["collar_thickness"])]
    loops = [ring(s,(0,0,z),rx,ry,body_weights) for z,rx,ry in levels]
    for level,(lo,hi) in enumerate(zip(loops,loops[1:])):
        for i in range(12):
            if level in (3,4) and i in (11,0,5,6):
                continue
            s.face((lo[i],lo[(i+1)%12],hi[(i+1)%12],hi[i]),c["trim"] if level in (0,6) else tunic,"garment_tunic")
    s.face(tuple(reversed(loops[0])),c["trim"],"garment_tunic")
    s.face(tuple(loops[-1]),c["trim"],"garment_tunic")
    limbs = {}
    for side,sign,sector in (("L",1,0),("R",-1,6)):
        get = lambda level, offset: loops[level][(sector+offset)%12]
        boundary = [get(3,-1),get(3,0),get(3,1),get(4,1),get(5,1),get(5,0),get(5,-1),get(4,-1)]
        start = Vector((sign*(chest_x+tuning["shoulder_outset_m"]),0,shoulder-g["clearance"]))
        wrist = start+Vector((sign*a["arm_length"]*tuning["arm_outward_fraction"],0,-a["arm_length"]))
        elbow = start.lerp(wrist,.5)+Vector((0,settings["elbow_offset_m"],0))
        axis = (wrist-start).normalized()
        right = axis.cross(Vector((0,1,0))).normalized()
        back = axis.cross(right).normalized()
        for vi in boundary:
            height = (s.vertices[vi][2]-arm_z)/(top_z-arm_z)
            arm_weight = tuning["underarm_skin_weight"]*(1-height)+tuning["shoulder_skin_weight"]*height
            clavicle_weight = tuning["shoulder_clavicle_weight"]*height
            s.weights[vi] = {"chest":1-arm_weight-clavicle_weight,f"upper_arm_{side}":arm_weight,f"clavicle_{side}":clavicle_weight}
        previous = boundary
        sleeve_loops = []
        socket = []
        for t,mult in tuning["arm_profile"]:
            center = start.lerp(elbow,t*2) if t<=.5 else elbow.lerp(wrist,(t-.5)*2)
            local_axis = axis
            right = local_axis.cross(Vector((0,1,0))).normalized()
            back = local_axis.cross(right).normalized()
            radius = (a["arm_radius"]*variant["arms"]+g["clearance"])*mult
            weights = blend(f"clavicle_{side}",f"upper_arm_{side}",tuning["shoulder_root_weight"]) if t==tuning["arm_profile"][0][0] else limb_weights(t,f"upper_arm_{side}",f"forearm_{side}",settings)
            current = []
            for i in range(8):
                angle = math.tau*i/8
                support = max(0,-sign*math.cos(angle))*max(0,1-t/tuning["inner_sleeve_support_end"])*tuning["inner_sleeve_support_weight"]
                local_weights = {name:value*(1-support) for name,value in weights.items()}
                local_weights["chest"] = local_weights.get("chest",0)+support
                current.append(s.vertex(center+radius*(right*math.cos(angle)+back*math.sin(angle)),local_weights))
            current = align_loop(s,previous,current)
            if not sleeve_loops:
                for fraction in tuning["shoulder_socket_fractions"]:
                    support_loop = []
                    for inner,outer in zip(boundary,current):
                        mixed = {name:s.weights[inner].get(name,0)*(1-fraction)+s.weights[outer].get(name,0)*fraction for name in sorted(set(s.weights[inner])|set(s.weights[outer]))}
                        support_loop.append(s.vertex(Vector(s.vertices[inner]).lerp(Vector(s.vertices[outer]),fraction),mixed))
                    bridge(s,previous,support_loop,tunic,"garment_tunic")
                    socket += support_loop
                    previous = support_loop
            sleeve_loops.append(current)
            bridge(s,previous,current,c["trim"] if t==1 else tunic,"garment_tunic")
            previous = current
        s.face(tuple(previous),c["trim"],"garment_tunic")
        relax_socket(s,set(boundary+socket+sleeve_loops[0]),tuning)
        limbs[side] = {"arm_start":start,"elbow":elbow,"wrist":wrist,"arm_axis":axis}
        inner_loops = []
        for index,loop in enumerate(sleeve_loops):
            if tuning["arm_profile"][index][0] not in tuning["covered_arm_ring_fractions"]:
                continue
            center = sum((Vector(s.vertices[vi]) for vi in loop),Vector())/len(loop)
            inset = -axis*tuning["skin_inset_m"] if index==len(sleeve_loops)-1 else Vector()
            inner_loops.append([s.vertex(center+(Vector(s.vertices[vi])-center)*tuning["covered_arm_radius_fraction"]+inset,s.weights[vi]) for vi in loop])
        for lo,hi in zip(inner_loops,inner_loops[1:]):
            bridge(s,lo,hi,skin,f"covered_arm_{side}")
        s.face(tuple(reversed(inner_loops[0])),skin,f"covered_arm_{side}")
        s.face(tuple(inner_loops[-1]),skin,f"covered_arm_{side}")
    # A coarse enclosed torso remains as body geometry. Limbs retain connected
    # garment topology; hidden skin uses narrower anatomical limb cages below.
    s.loft("covered_torso", [((0,0,hem+g["clearance"]),waist_x-g["clearance"],waist_y-g["clearance"]),
        ((0,0,arm_z),chest_x-g["clearance"]*5,chest_y-g["clearance"]*2),
        ((0,0,neck_bottom-g["clearance"]),neck_r-g["clearance"],neck_r-g["clearance"])],skin,body_weights,12)
    # A single pelvis loop branches through shared crotch edges into both legs.
    upper_z = hem+tuning["waist_overlap_m"]
    low_z = hip-tuning["hip_drop_m"]
    upper = ring(s,(0,0,upper_z),waist_x-g["clearance"],waist_y-g["clearance"],body_weights)
    lower = ring(s,(0,0,low_z),hip_x,waist_y,"pelvis")
    bridge(s,upper,lower,c["trousers"],"garment_trousers")
    s.face(tuple(upper),c["trousers"],"garment_trousers")
    crotch = s.vertex((0,0,hip-tuning["crotch_drop_m"]),{"pelvis":1-tuning["hip_skin_weight"],"thigh_L":tuning["hip_skin_weight"]/2,"thigh_R":tuning["hip_skin_weight"]/2})
    ankle_z = a["boot_height"]*tuning["ankle_height_boot_fraction"]
    for side,sign,order in (("L",1,[9,10,11,0,1,2,3]),("R",-1,[3,4,5,6,7,8,9])):
        x = sign*a["leg_spacing"]*width*variant["hips"]
        start = Vector((x,0,hip-tuning["hip_joint_drop_m"]))
        knee = Vector((x,-settings["knee_offset_m"],(start.z+ankle_z)/2))
        ankle = Vector((x,0,ankle_z))
        boundary = [lower[i] for i in order]+[crotch]
        for i in order:
            s.weights[lower[i]] = {"pelvis":1-tuning["hip_skin_weight"],"thigh_L":tuning["hip_skin_weight"]/2,"thigh_R":tuning["hip_skin_weight"]/2} if i in (3,9) else blend("pelvis",f"thigh_{side}",tuning["hip_skin_weight"])
        previous = boundary
        radius = a["leg_radius"]*width*variant["legs"]
        for t,mult in tuning["leg_profile"]:
            center = start.lerp(knee,t*2) if t<=.5 else knee.lerp(ankle,(t-.5)*2)
            weights = blend("pelvis",f"thigh_{side}",tuning["thigh_root_weight"]) if t==tuning["leg_profile"][0][0] else limb_weights(t,f"thigh_{side}",f"shin_{side}",settings)
            current = ring(s,center,radius*mult,radius*mult,weights,8)
            current = align_loop(s,previous,current)
            bridge(s,previous,current,c["trousers"],"garment_trousers")
            previous = current
        s.face(tuple(previous),c["trousers"],"garment_trousers")
        # Skin ends inside the boot and remains inside the trouser's thigh/calf.
        s.limb(f"covered_leg_{side}",start.lerp(knee,.5),knee,ankle,radius*.48,skin,f"thigh_{side}",f"shin_{side}",settings,8,taper=.8)
        build_boot(s,x,a,p,g,c,tuning,radius,side)
        limbs[side].update(x=x,knee=knee,ankle=ankle,thigh_start=start)
    return limbs,body_weights,chest_x,chest_y,neck_r


def build_boot(s,x,a,p,g,c,tuning,radius,side):
    """Low toe box, heel and instep join a narrow ankle and calf shaft."""
    half_width = a["foot_width"]*p["foot_scale"]/2
    length = a["foot_length"]*p["foot_scale"]
    weights = lambda point: blend(f"foot_{side}",f"shin_{side}",(point[2]-tuning["boot_instep_height_m"])/(a["boot_height"]-tuning["boot_instep_height_m"]))
    loops=[]
    for level in (0,g["boot_sole_height"],None):
        loop=[]
        for xx,yy in tuning["boot_footprint"]:
            y = yy*length*tuning["boot_heel_length_fraction" if yy>0 else "boot_toe_length_fraction"]
            z = level if level is not None else tuning["boot_heel_height_m" if yy>0 else "boot_toe_height_m"]
            loop.append(s.vertex((x+xx*half_width,y,z),weights))
        loops.append(loop)
    for z,rx,ry in ((tuning["boot_instep_height_m"],half_width*tuning["boot_ankle_width_fraction"],length*tuning["boot_ankle_depth_fraction"]/2),(a["boot_height"]-g["cuff_height"],radius,radius),(a["boot_height"],radius+g["clearance"]/2,radius+g["clearance"]/2)):
        loops.append(align_loop(s,loops[-1],ring(s,(x,0,z),rx,ry,weights,8)))
    for i,(lo,hi) in enumerate(zip(loops,loops[1:])):
        bridge(s,lo,hi,c["seam"] if i in (0,4) else c["boots"],f"boot_{side}")
    s.face(tuple(reversed(loops[0])),c["boots"],f"boot_{side}")
    s.face(tuple(loops[-1]),c["boots"],f"boot_{side}")


def fitted_hair(s,row,profile,head_w,head_d,head_h,head_bottom,sides,tuning):
    """Hair sits outside the exact head loft, including its temple and bob rim."""
    front,back = tuning["hairline"][row["hair_style"]]
    clearance = tuning["hair_clearance_m"]
    def radius(z):
        for (za,ra),(zb,rb) in zip(profile,profile[1:]):
            if za<=z<=zb:
                return ra+(rb-ra)*(z-za)/(zb-za)
        return profile[-1][1]
    loops=[]
    # Dense fitted rows and a conservative radial envelope keep the hair
    # outside skull curvature. The exported triangles are checked against it.
    bands = max(tuning["hair_rows"],len(profile)-1)
    for band in range(bands+1):
        loop=[]
        for i in range(sides):
            angle=math.tau*i/sides
            facing=min(1,max(0,-math.sin(angle))/tuning["hair_front_span"])
            low=back+(front-back)*facing
            z=low+(1-low)*band/bands
            r=max(radius(z),radius(min(1,z+(1-low)/bands)))
            if row["hair_style"]=="bob" and z<.67:
                r=max(r,.98)
            loop.append(s.vertex(((head_w*r/2+clearance)*math.cos(angle),(head_d*r/2+clearance)*math.sin(angle),head_bottom+head_h*z+clearance),"head"))
        loops.append(loop)
    for lo,hi in zip(loops,loops[1:]):
        bridge(s,lo,hi,row["hair_srgb"],"hair_cap")
    inner=[]
    for vi in loops[0]:
        x,y,z=s.vertices[vi]
        direction=Vector((x,y,0)).normalized()*tuning["hair_rim_m"]
        inner.append(s.vertex((x-direction.x,y-direction.y,z),"head"))
    bridge(s,inner,loops[0],row["hair_srgb"],"hair_cap")
    s.face(tuple(loops[-1]),row["hair_srgb"],"hair_cap")
