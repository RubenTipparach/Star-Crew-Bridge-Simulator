// The outside (openspec/changes/deck-pipeline, design section 13b; openspec/changes/nebula-skies). Four programs:
//
// - sky: drawn first each frame on the sky mesh, a closed sphere of directions with a colour each (Homeworld's
//   vertex-coloured background, nebula-skies design 3): the vertex colour (sRGB, Gouraud shaded), the sun's disc and
//   glow, and a planet (a sphere lit by the sun, oceans, land and cloud from value noise, an atmosphere's rim), with an
//   ordered dither so a slow gradient never bands. Its alpha is the share of the faint stars that show there: none
//   where the nebula is dense or the planet stands, less while the view turns. Its vertices lie on the far plane (the
//   sky layer, sky-behind-solids). Every number comes from the sky file and data/space/exterior.json; directions are
//   the frame the view's rotation turns into the eye's (the system's in a patrol, the ship's in the walk).
// - faint: the faint stars (three hashed grids of three sizes), one triangle over the whole target with no vertex
//   buffer, added in proportion to the alpha the sky left. Their per-pixel work is the sky's dearest, so it is done
//   once a pixel here rather than again at every edge of the mesh's small triangles (measured on the Pi 5: the
//   nebula-skies benchmark).
// - star_lines and star_heads: the vector stars (FreeSpace's, nebula-skies design 5.2), each a GL line from where it is
//   on screen back toward where it was tail_s ago, its tail capped and its brightness dimmed as it grows, and for the
//   brighter classes a round GL point over its head; alpha-blended over the sky, on the far plane.
// - screen: a viewscreen, a quad built from its corner index (no vertex buffer) showing the bow
//   camera's target with faint scan lines.
// One source, compiled by sokol-shdc to GLSL ES 3.00 and GLSL 4.10 (tools/sokol_shaders.py).

@vs sky_vs
layout(binding=0) uniform sky_vs_params {
    // The projection times the view's rotation (no translation): a direction to a clip position.
    mat4 vp;
};
in vec3 sky_dir;
in vec4 sky_col;
out vec3 ray;
out vec4 col;

void main() {
    ray = sky_dir;
    col = sky_col;
    // A direction at infinity, put on the far plane (z = w): the sky layer's depth.
    vec4 p = vp * vec4(sky_dir, 0.0);
    gl_Position = p.xyww;
}
@end

@block hashes
float hash13(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

vec3 hash33(vec3 p) {
    p = fract(p * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return fract((p.xxy + p.yxx) * p.zyx);
}
@end

@fs sky_fs
layout(binding=1) uniform sky_fs_params {
    // xyz: the way to the sun; w: the cosine of the disc's angular radius.
    vec4 sun;
    // rgb: the sun's colour (linear); w: the glow's strength.
    vec4 sun_colour;
    // xyz: the way to the planet's centre; w: the sine of its angular radius.
    vec4 planet;
    // rgb: sea; w: the share of land.
    vec4 ocean;
    // rgb: land; w: the share of cloud.
    vec4 land;
    // rgb: cloud; w: the night side's brightness.
    vec4 cloud;
    // rgb: the atmosphere's rim; w: its thickness as a share of the radius.
    vec4 atmosphere;
    // x: the faint stars' share this frame (they fade while the view turns, nebula-skies design 5.5).
    vec4 stars;
};

in vec3 ray;
// rgb: the nebula's colour (sRGB); a: the share of the stars behind it the nebula hides.
in vec4 col;
out vec4 frag_color;

@include_block hashes

float noise(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = mix(mix(hash13(i), hash13(i + vec3(1, 0, 0)), f.x), mix(hash13(i + vec3(0, 1, 0)), hash13(i + vec3(1, 1, 0)), f.x), f.y);
    float b = mix(mix(hash13(i + vec3(0, 0, 1)), hash13(i + vec3(1, 0, 1)), f.x), mix(hash13(i + vec3(0, 1, 1)), hash13(i + vec3(1, 1, 1)), f.x), f.y);
    return mix(a, b, f.z);
}

float fbm(vec3 p) {
    float s = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        s += a * noise(p);
        p = p * 2.03 + vec3(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    return s;
}

void main() {
    vec3 d = normalize(ray);
    vec3 c = col.rgb;
    // The faint stars' share (drawn by the faint program): hidden where the nebula is dense, faded while the view turns.
    float faint = (1.0 - col.a) * stars.x;
    // The sun: a hard disc and a soft glow.
    float cs = dot(d, sun.xyz);
    c += sun_colour.rgb * (smoothstep(sun.w - 0.00002, sun.w + 0.00002, cs) * 8.0 + sun_colour.w * pow(max(cs, 0.0), 400.0) + 0.04 * sun_colour.w * pow(max(cs, 0.0), 12.0));
    // The planet: a sphere of radius sin(a) one unit away along its direction.
    vec3 pc = planet.xyz;
    float r = planet.w;
    float b = dot(d, pc);
    float disc = b * b - (1.0 - r * r);
    float atm = r * (1.0 + atmosphere.w);
    // The ray's least distance from the centre, as a share of the radius.
    float h = sqrt(max(1.0 - b * b, 0.0)) / r;
    vec3 lit_dir = sun.xyz;
    if (disc > 0.0 && b > 0.0) {
        float t = b - sqrt(disc);
        vec3 n = normalize(d * t - pc);
        float lambert = dot(n, lit_dir);
        // Surface: noise on the sphere, cloud bands stretched along the latitude.
        // fbm sits about 0.5 with a spread near 0.12: a share maps to a threshold on it.
        float lt = 0.5 + (0.5 - ocean.w) * 0.35;
        float lnd = smoothstep(lt - 0.015, lt + 0.015, fbm(n * 3.0 + 4.0));
        vec3 ground = mix(ocean.rgb, land.rgb, lnd);
        float cl = fbm(vec3(n.x * 2.5, n.y * 7.0, n.z * 2.5) + fbm(n * 4.0) * 1.5);
        float ct = 0.5 + (0.5 - land.w) * 0.35;
        float cov = smoothstep(ct - 0.06, ct + 0.1, cl * 0.75);
        vec3 surf = mix(ground, cloud.rgb, cov);
        float day = smoothstep(-0.08, 0.25, lambert);
        vec3 lit = surf * sun_colour.rgb * max(lambert, 0.0) * 1.1 + surf * cloud.w * (1.0 - day);
        // The limb: the air seen edge on.
        float limb = pow(1.0 - max(dot(n, -d), 0.0), 3.0);
        lit += atmosphere.rgb * limb * day * 0.8;
        // The nebula's light on the night side, faintly: the planet stands in it (the mockup's 6%).
        c = lit + col.rgb * 0.06 * (1.0 - day);
        // No star shows through the planet.
        faint = 0.0;
    } else if (b > 0.0 && h < 1.0 + atmosphere.w * 3.0) {
        // Past the limb: the atmosphere's glow, fading over three thicknesses, lit where the sun reaches.
        vec3 edge = normalize(d * b - pc);
        float glow = exp(-(h - 1.0) / max(atmosphere.w, 1e-4));
        float day = smoothstep(-0.3, 0.3, dot(edge, lit_dir));
        c += atmosphere.rgb * glow * day * 0.9;
    }
    // A 4 x 4 ordered (Bayer) dither of one 8-bit step, so a slow gradient over near black never bands on an 8-bit
    // target (design 3); arithmetic on the pixel's position, no texture.
    vec2 px = floor(gl_FragCoord.xy);
    vec2 h2 = floor(px * 0.5);
    float bayer = fract(h2.x * 0.5 + h2.y * h2.y * 0.75) * 0.25 + fract(px.x * 0.5 + px.y * px.y * 0.75);
    c += (bayer - 0.47) / 255.0;
    frag_color = vec4(c, faint);
}
@end

@vs faint_vs
layout(binding=0) uniform faint_vs_params {
    // The inverse of the projection times the view's rotation: a clip position to a direction.
    mat4 inv_vp;
};
out vec3 ray;

void main() {
    vec2 p = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2)) * 2.0 - 1.0;
    vec4 d = inv_vp * vec4(p, 1.0, 1.0);
    ray = d.xyz / d.w;
    // On the far plane: the sky layer.
    gl_Position = vec4(p, 1.0, 1.0);
}
@end

@fs faint_fs
layout(binding=1) uniform faint_fs_params {
    // x: star density; y: star brightness; z: the size of a pixel, radians.
    vec4 stars;
};

in vec3 ray;
out vec4 frag_color;

@include_block hashes

// One grid of stars: a star at a hashed point in some of the cells round the direction.
vec3 star_grid(vec3 d, float scale, float size_px) {
    vec3 p = d * scale;
    vec3 cell = floor(p);
    vec3 h = hash33(cell);
    if (h.z > stars.x) {
        return vec3(0.0);
    }
    vec3 at = normalize(cell + 0.2 + 0.6 * h);
    // The angle to the star as the chord's length (2 sin(r / 2), within a millionth of r over the few pixels a star
    // spans): no acos per grid per pixel, which the nebula's many small triangles would pay again at their edges.
    float r = length(d - at) / stars.z;
    float k = clamp(1.0 - r / size_px, 0.0, 1.0);
    float tint = hash13(cell + 7.0);
    vec3 c = mix(vec3(0.75, 0.85, 1.0), vec3(1.0, 0.9, 0.75), tint);
    // Most stars are faint: brightness falls with the cube of a uniform draw.
    return c * k * k * stars.y * (0.15 + 0.85 * h.y * h.y * h.y);
}

void main() {
    vec3 d = normalize(ray);
    // Added in proportion to the alpha the sky left there (the blend's destination alpha).
    frag_color = vec4(star_grid(d, 90.0, 1.6) + star_grid(d, 160.0, 1.1) + star_grid(d, 300.0, 0.8), 0.0);
}
@end

@vs star_vs
layout(binding=0) uniform star_vs_params {
    // This frame's and last frame's projection times view rotation, and the rotations alone.
    mat4 vp;
    mat4 prev_vp;
    mat4 rot;
    mat4 prev_rot;
    // xy: the target's size in pixels; z: a pixel's angle, radians; w: unused.
    vec4 view;
    // x: the tail's scale (tail_s over the frame's seconds; 0 at a cut); y: the cap, radians; z: the least line,
    // pixels; w: the brightness lost per radian of tail.
    vec4 tail;
    // x: the least brightness kept; y: the tail end's share of the head's opacity; z: a streak's least opacity;
    // w: how far a streak whitens.
    vec4 look;
    // xyz: the way to the planet's centre; w: the cosine of its angular radius (stars inside are not drawn).
    vec4 planet;
    // xyz: the way to the sun; w: the cosine of its disc's angular radius.
    vec4 sun;
};
in vec3 star_dir;
// rgb: the star's colour (sRGB); a: its brightness.
in vec4 star_col;
// x: 0 for the head, 1 for the tail end; y: the head's size at rest, pixels.
in vec2 star_end;
out vec4 col;

void main() {
    // The tail's angle this frame: how far the view turned this star, scaled to tail_s, capped.
    vec3 a = (rot * vec4(star_dir, 0.0)).xyz;
    vec3 b = (prev_rot * vec4(star_dir, 0.0)).xyz;
    float moved = acos(clamp(dot(a, b), -1.0, 1.0));
    float tl = min(moved * tail.x, tail.y);
    float ratio = moved > 1e-7 ? tl / moved : 0.0;
    vec4 p = vp * vec4(star_dir, 0.0);
    vec4 q = prev_vp * vec4(star_dir, 0.0);
    vec2 hp = p.xy / p.w * 0.5 * view.xy;
    vec2 qp = q.xy / q.w * 0.5 * view.xy;
    vec2 tp = q.w > 0.0 ? hp + (qp - hp) * ratio : hp;
    vec2 dv = tp - hp;
    float len = length(dv);
    // Near the edge of the view the projection stretches: never longer than the cap's length at the centre, with room.
    float max_px = tail.y / view.z * 1.5;
    if (len > max_px) {
        tp = hp + dv * (max_px / len);
        dv = tp - hp;
        len = max_px;
    }
    // A star at rest is a point: a line of the least length (FreeSpace's one-pixel fudge).
    if (len < tail.z) {
        tp = hp + (len > 1e-4 ? dv / len : vec2(1.0, 0.0)) * tail.z;
    }
    vec2 o = star_end.x < 0.5 ? hp : tp;
    bool hidden = dot(star_dir, planet.xyz) > planet.w || dot(star_dir, sun.xyz) > sun.w;
    // On the far plane (the sky layer); a hidden star is put outside the clip volume.
    gl_Position = hidden ? vec4(0.0, 0.0, 2.0, 1.0) : vec4(o / (0.5 * view.xy) * p.w, p.w, p.w);
    gl_PointSize = star_end.y + 1.0;
    float dim = max(look.x, 1.0 - tail.w * tl);
    // At rest a star is its colour at its brightness; as its tail passes a quarter degree it whitens and lifts toward
    // the least opacity, so a streak reads on a bright cloud (design 5.2).
    float s = smoothstep(0.0, 0.0044, tl);
    col = vec4(mix(star_col.rgb, mix(star_col.rgb, vec3(1.0), look.w), s), mix(star_col.a, look.z + (1.0 - look.z) * star_col.a, s) * dim);
    if (star_end.x > 0.5) {
        col.a *= mix(1.0, look.y, smoothstep(0.0, 3.0, len));
    }
}
@end

@fs star_line_fs
in vec4 col;
out vec4 frag_color;

void main() {
    frag_color = col;
}
@end

@fs star_head_fs
in vec4 col;
out vec4 frag_color;

void main() {
    vec2 q = gl_PointCoord * 2.0 - 1.0;
    float r = dot(q, q);
    if (r > 1.0) {
        discard;
    }
    frag_color = vec4(col.rgb, col.a * (1.0 - r) * (1.0 - r));
}
@end

@vs screen_vs
layout(binding=0) uniform screen_vs_params {
    // Maps the quad's corners (-1..1 in x and y, z 0) to clip space.
    mat4 mvp;
};
out vec2 uv;

void main() {
    // Two triangles, counter-clockwise seen from the front (+z of the quad).
    int i = gl_VertexIndex;
    int k = (i < 3) ? i : ((i == 3) ? 0 : i - 2);
    vec2 p = vec2((k == 1 || k == 2) ? 1.0 : -1.0, (k >= 2) ? 1.0 : -1.0);
    uv = p * 0.5 + 0.5;
    gl_Position = mvp * vec4(p, 0.0, 1.0);
}
@end

@fs screen_fs
layout(binding=0) uniform texture2D tex;
layout(binding=0) uniform sampler smp;
layout(binding=1) uniform screen_fs_params {
    // x: the scan lines' depth; y: the target's height in pixels; z: brightness.
    vec4 look;
};

in vec2 uv;
out vec4 frag_color;

void main() {
    vec3 c = texture(sampler2D(tex, smp), uv).rgb;
    float line = 1.0 - look.x * (0.5 + 0.5 * sin(uv.y * look.y * 3.14159265));
    // A faint frame of shade at the edges, as a screen's glass.
    vec2 e = min(uv, 1.0 - uv);
    float edge = smoothstep(0.0, 0.03, min(e.x, e.y * 2.0));
    frag_color = vec4(c * line * (0.6 + 0.4 * edge) * look.z, 1.0);
}
@end

@program sky sky_vs sky_fs
@program faint faint_vs faint_fs
@program star_lines star_vs star_line_fs
@program star_heads star_vs star_head_fs
@program screen screen_vs screen_fs
