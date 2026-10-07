// The deck program (engine-stack design section 7, shader 1): the 28-byte deck vertex
// (sc-core::vertex, deck-pipeline section 5), three baked colour sets blended by the
// compartment's state weights (normal, red alert, emergency), a texture array layer per
// material, and a per-vertex term for a dynamic light. deck_flat skips the texture fetch
// (the probe's fill-rate scene compares the two, design section 11, scene 3).
// One source, compiled by sokol-shdc to GLSL ES 3.00 and GLSL 4.10 (tools/sokol_shaders.py).

@vs deck_vs
layout(binding=0) uniform deck_vs_params {
    mat4 mvp;
    // x, y, z: the weights of the normal, red alert and emergency colour sets; they sum to 1.
    vec4 state_weights;
    // xyz: a dynamic light's direction towards the light, w: its strength (0: none).
    vec4 flash;
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

void main() {
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

in vec4 color;
in vec3 uv_layer;
out vec4 frag_color;

void main() {
    frag_color = vec4(texture(sampler2DArray(tex, smp), uv_layer).rgb * color.rgb, 1.0);
}
@end

@fs deck_flat_fs
in vec4 color;
in vec3 uv_layer;
out vec4 frag_color;

void main() {
    frag_color = vec4(color.rgb, 1.0);
}
@end

@program deck deck_vs deck_fs
@program deck_flat deck_vs deck_flat_fs
