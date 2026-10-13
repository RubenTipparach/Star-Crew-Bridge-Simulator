// The effects program (all-hands 4; the owner, 2026-10-12: "fire is also super bare bones"): camera-facing quads
// built on the CPU each frame, one draw, blended premultiplied so a flame adds its light (alpha 0) and smoke and a
// spray's agent cover what is behind them (alpha over). Each quad says what it is in its fourth parameter:
//   0 a flame (docs/mockups/lib/firefight.js's flame shader: an fbm body, red to orange to a yellow core),
//   1 an ember on a burnt floor (the mockup's knocked-down cells), 2 a puff of smoke, 3 a puff of agent,
//   4 the smoke's haze over the eye.
// Its positions are relative to the eye, metres, as the decks are drawn. One source, compiled by sokol-shdc
// (tools/sokol_shaders.py). The mockup's falling edges, `smoothstep(1.0, 0.5, x)`, are written `fall(0.5, 1.0, x)`:
// GLSL leaves smoothstep with its first edge above its second undefined, and the Pi's driver draws nothing with it.

@vs fx_vs
layout(binding=0) uniform fx_vs_params {
    // The projection times the view, for positions relative to the eye.
    mat4 vp;
};

in vec3 fx_pos;
in vec2 fx_uv;
in vec4 fx_p;

out vec2 uv;
out vec4 p;

void main() {
    gl_Position = vp * vec4(fx_pos, 1.0);
    uv = fx_uv;
    p = fx_p;
}
@end

@fs fx_fs
layout(binding=1) uniform fx_fs_params {
    // x: the clock, seconds (the flames' noise rises with it).
    vec4 time;
};

// uv: 0-1 across the quad, y up. p.x: the quad's phase (0-1, its own noise), p.y: heat (0-1), p.z: what it is (see
// the top), p.w: its strength (0-1: a puff's age left, a haze's thickness).
in vec2 uv;
in vec4 p;
out vec4 frag_color;

float hash(vec2 q) {
    return fract(sin(dot(q, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 q) {
    vec2 i = floor(q);
    vec2 f = fract(q);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

// 1 below e0 falling to 0 above e1 (e0 < e1): smoothstep(e1, e0, x) as WebGL draws it.
float fall(float e0, float e1, float x) {
    return 1.0 - smoothstep(e0, e1, x);
}

float fbm(vec2 q) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 4; i++) {
        v += a * noise(q);
        q *= 2.03;
        a *= 0.5;
    }
    return v;
}

void main() {
    float t0 = time.x;
    float kind = p.z;
    if (kind < 0.5) {
        // A flame: the mockup's, line for line.
        float t = t0 * 1.7 + p.x * 7.0;
        float n = fbm(vec2(uv.x * 3.0 + p.x * 5.0, uv.y * 2.3 - t));
        float n2 = fbm(vec2(uv.x * 6.5 - p.x, uv.y * 4.5 - t * 1.6));
        float x = (uv.x - 0.5) * 2.0;
        float wy = mix(0.85, 0.06, pow(uv.y, 0.75));
        float d = abs(x + (n - 0.5) * 0.8 * uv.y) / max(wy, 0.03);
        float body = 1.0 - smoothstep(0.3, 1.0, d + (n2 - 0.5) * 0.7);
        body *= fall(0.5, 1.0, uv.y + (n - 0.5) * 0.5);
        body *= smoothstep(0.0, 0.05, uv.y);
        float core = body * (1.0 - smoothstep(0.0, 0.6, uv.y * 1.2 + d * 0.6));
        vec3 col = mix(vec3(0.85, 0.1, 0.02), vec3(1.0, 0.5, 0.07), smoothstep(0.0, 0.7, body));
        col = mix(col, vec3(1.0, 0.9, 0.55), core);
        frag_color = vec4(col * body * (0.6 + 0.5 * p.y) * 1.25, 0.0);
    } else if (kind < 1.5) {
        // Embers: a glowing crust in patches, breathing, dying away as p.w falls.
        float n = fbm(vec2(uv.x * 5.0 + p.x * 13.0, uv.y * 2.0 - t0 * 0.5));
        float m = smoothstep(0.52, 0.8, n) * (1.0 - uv.y) * smoothstep(0.0, 0.2, uv.x) * fall(0.8, 1.0, uv.x);
        float fl = 0.55 + 0.45 * sin(t0 * 3.7 + p.x * 21.0);
        frag_color = vec4(vec3(1.0, 0.22, 0.04) * m * fl * 1.4 * p.w, 0.0);
    } else if (kind < 3.5) {
        // A puff, soft and round, its edge broken by noise: smoke dark and warm from the fire under it, agent white.
        vec2 c = uv - 0.5;
        float r = length(c) * 2.0;
        float n = fbm(c * 3.0 + vec2(p.x * 17.0, -t0 * 0.3));
        float a = (1.0 - smoothstep(0.35, 1.0, r + (n - 0.5) * 0.5)) * p.w;
        vec3 col = kind < 2.5 ? mix(vec3(0.05, 0.045, 0.04), vec3(0.22, 0.09, 0.03), p.y) : vec3(0.92, 0.95, 0.98);
        a *= kind < 2.5 ? 0.75 : 0.5;
        frag_color = vec4(col * a, a);
    } else {
        // The haze of a smoky room over the eye: thicker toward the top, warm from the flames.
        float a = p.w * mix(0.55, 1.0, uv.y);
        vec3 col = mix(vec3(0.06, 0.05, 0.045), vec3(0.3, 0.12, 0.04), p.y);
        frag_color = vec4(col * a, a);
    }
}
@end

@program fx fx_vs fx_fs
