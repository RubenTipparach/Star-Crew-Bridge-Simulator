// The outside (openspec/changes/deck-pipeline, design section 13b). Two programs:
//
// - sky: drawn first each frame, one triangle over the whole target with no vertex buffer: the
//   stars (three hashed grids of three sizes), the sun's disc and glow, and a planet (a sphere lit
//   by the sun, oceans, land and cloud from value noise, an atmosphere's rim). Every number comes
//   from data/space/exterior.json; directions are ship coordinates.
// - screen: a viewscreen, a quad built from its corner index (no vertex buffer) showing the bow
//   camera's target with faint scan lines.
// One source, compiled by sokol-shdc to GLSL ES 3.00 and GLSL 4.10 (tools/sokol_shaders.py).

@vs sky_vs
layout(binding=0) uniform sky_vs_params {
    // The inverse of the projection times the view's rotation: a clip position to a direction.
    mat4 inv_vp;
};
out vec3 ray;

void main() {
    vec2 p = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2)) * 2.0 - 1.0;
    vec4 d = inv_vp * vec4(p, 1.0, 1.0);
    ray = d.xyz / d.w;
    gl_Position = vec4(p, 1.0, 1.0);
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
    // x: star density; y: star brightness; z: the size of a pixel, radians.
    vec4 stars;
};

in vec3 ray;
out vec4 frag_color;

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

// One grid of stars: a star at a hashed point in some of the cells round the direction.
vec3 star_grid(vec3 d, float scale, float size_px) {
    vec3 p = d * scale;
    vec3 cell = floor(p);
    vec3 h = hash33(cell);
    if (h.z > stars.x) {
        return vec3(0.0);
    }
    vec3 at = normalize(cell + 0.2 + 0.6 * h);
    float r = acos(clamp(dot(d, at), -1.0, 1.0)) / stars.z;
    float k = clamp(1.0 - r / size_px, 0.0, 1.0);
    float tint = hash13(cell + 7.0);
    vec3 c = mix(vec3(0.75, 0.85, 1.0), vec3(1.0, 0.9, 0.75), tint);
    // Most stars are faint: brightness falls with the cube of a uniform draw.
    return c * k * k * stars.y * (0.15 + 0.85 * h.y * h.y * h.y);
}

void main() {
    vec3 d = normalize(ray);
    vec3 c = star_grid(d, 90.0, 1.6) + star_grid(d, 160.0, 1.1) + star_grid(d, 300.0, 0.8);
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
        vec3 col = surf * sun_colour.rgb * max(lambert, 0.0) * 1.1 + surf * cloud.w * (1.0 - day);
        // The limb: the air seen edge on.
        float limb = pow(1.0 - max(dot(n, -d), 0.0), 3.0);
        col += atmosphere.rgb * limb * day * 0.8;
        c = col;
    } else if (b > 0.0 && h < 1.0 + atmosphere.w * 3.0) {
        // Past the limb: the atmosphere's glow, fading over three thicknesses, lit where the sun reaches.
        vec3 edge = normalize(d * b - pc);
        float glow = exp(-(h - 1.0) / max(atmosphere.w, 1e-4));
        float day = smoothstep(-0.3, 0.3, dot(edge, lit_dir));
        c += atmosphere.rgb * glow * day * 0.9;
    }
    frag_color = vec4(c, 1.0);
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
@program screen screen_vs screen_fs
