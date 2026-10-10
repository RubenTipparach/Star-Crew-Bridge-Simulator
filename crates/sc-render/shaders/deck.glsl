// The deck program (engine-stack design section 7, shader 1): the 28-byte deck vertex
// (sc-core::vertex, deck-pipeline section 5), three baked colour sets blended by the
// compartment's state weights (normal, red alert, emergency), a texture array layer per
// material, and a per-vertex term for a dynamic light. deck_flat skips the texture fetch
// (the probe's fill-rate scene compares the two, design section 11, scene 3). deck_probe lights a
// moving thing (a crew figure) from an ambient cube of light probes instead of a baked colour
// (light-baking design sections 6 and 16): its vertex colour is its albedo.
// One source, compiled by sokol-shdc to GLSL ES 3.00 and GLSL 4.10 (tools/sokol_shaders.py).

@vs deck_vs
layout(binding=0) uniform deck_vs_params {
    mat4 mvp;
    // x, y, z: the weights of the normal, red alert and emergency colour sets; they sum to 1.
    vec4 state_weights;
    // xyz: a dynamic light's direction towards the light, w: its strength (0: none).
    vec4 flash;
    // x: a clip height in the mesh's own frame, metres: nothing above it is drawn (the ship map's cut,
    // ship-plan-view 5); a huge value draws everything.
    vec4 clip;
};

// Integer attributes (sokol_gfx's SHORT4 and SHORT2 are integer formats): exact, decoded here.
in ivec4 pos_mover_layer;
in vec4 normal;
in vec4 color_normal;
in vec4 color_red_alert;
in vec4 color_emergency;
in ivec2 uv;

out vec4 color;
out vec3 uv_layer;
out float above_clip;

void main() {
    above_clip = float(pos_mover_layer.y) * (1.0 / 1024.0) - clip.x;
    gl_Position = mvp * vec4(vec3(pos_mover_layer.xyz) * (1.0 / 1024.0), 1.0);
    // The fourth lane holds the mover in its low byte and the texture layer in its high byte
    // (sc-core::vertex); it arrives sign-extended, so mask it back to 16 bits.
    int w = pos_mover_layer.w & 0xffff;
    float layer = float(w >> 8);
    vec4 c = color_normal * state_weights.x + color_red_alert * state_weights.y + color_emergency * state_weights.z;
    // A baked colour byte is a display multiplier from 0 to 2 (light-baking design 5).
    c.rgb = c.rgb * 2.0 + vec3(max(dot(normal.xyz, flash.xyz), 0.0) * flash.w);
    color = vec4(c.rgb, 1.0);
    uv_layer = vec3(vec2(uv) * (1.0 / 1024.0), layer);
}
@end

@fs deck_fs
layout(binding=0) uniform texture2DArray tex;
layout(binding=0) uniform sampler smp;
layout(binding=1) uniform deck_fs_params {
    // x: the first layer whose alpha is an emission mask (the panel layers); y: how bright the masks
    // glow in the current lighting state (panels.json glow, blended by the state weights).
    vec4 glow;
};

in vec4 color;
in vec3 uv_layer;
in float above_clip;
out vec4 frag_color;

void main() {
    if (above_clip > 0.0) {
        discard;
    }
    vec4 t = texture(sampler2DArray(tex, smp), uv_layer);
    // A panel layer's alpha is its emission mask: there the texel shows as itself, whatever the light.
    float emit = step(glow.x - 0.5, uv_layer.z) * t.a * glow.y;
    frag_color = vec4(mix(t.rgb * color.rgb, t.rgb, emit), 1.0);
}
@end

@vs deck_probe_vs
layout(binding=0) uniform deck_probe_vs_params {
    mat4 mvp;
    // The model's rotation (a bot's yaw): it turns a normal into the ship's frame, where the cube's axes are.
    mat4 model;
    // The ambient cube, display multipliers (0 to 2): a surface facing +X, -X, +Y, -Y, +Z and -Z.
    vec4 cube[6];
    // x, y, z: the weights of the albedo's normal, red alert and emergency sets (a figure has one in all three).
    vec4 state_weights;
    // x: the clip height, as deck_vs's.
    vec4 clip;
};

in ivec4 pos_mover_layer;
in vec4 normal;
in vec4 color_normal;
in vec4 color_red_alert;
in vec4 color_emergency;
in ivec2 uv;

out vec4 color;
out vec3 uv_layer;
out float above_clip;

void main() {
    above_clip = float(pos_mover_layer.y) * (1.0 / 1024.0) - clip.x;
    gl_Position = mvp * vec4(vec3(pos_mover_layer.xyz) * (1.0 / 1024.0), 1.0);
    vec3 n = normalize((model * vec4(normal.xyz, 0.0)).xyz);
    vec3 n2 = n * n;
    // Half-Life 2's ambient cube: each axis's colour weighted by the squared normal along it.
    vec3 light = n2.x * (n.x >= 0.0 ? cube[0].rgb : cube[1].rgb)
               + n2.y * (n.y >= 0.0 ? cube[2].rgb : cube[3].rgb)
               + n2.z * (n.z >= 0.0 ? cube[4].rgb : cube[5].rgb);
    vec4 a = color_normal * state_weights.x + color_red_alert * state_weights.y + color_emergency * state_weights.z;
    // The albedo byte is stored as a display multiplier (0 to 2) like a baked colour; display times display.
    color = vec4(a.rgb * 2.0 * light, 1.0);
    uv_layer = vec3(vec2(uv) * (1.0 / 1024.0), 0.0);
}
@end

@fs deck_flat_fs
in vec4 color;
in vec3 uv_layer;
in float above_clip;
out vec4 frag_color;

void main() {
    if (above_clip > 0.0) {
        discard;
    }
    frag_color = vec4(color.rgb, 1.0);
}
@end

@program deck deck_vs deck_fs
@program deck_flat deck_vs deck_flat_fs
@program deck_probe deck_probe_vs deck_flat_fs
