// The UI program (openspec/changes/lobby, design 3): 2D triangles in points from the top left, a
// texture and a premultiplied colour each, drawn over the output after the blit. The client's UI
// layer (egui) makes the triangles; this program knows nothing of it.
// One source, compiled by sokol-shdc to GLSL ES 3.00 and GLSL 4.10 (tools/sokol_shaders.py).

@vs ui_vs
layout(binding=0) uniform ui_vs_params {
    // xy: 2 / the output's size in points.
    vec4 screen;
};
in vec2 pos;
in vec2 uv;
in vec4 color;
out vec2 v_uv;
out vec4 v_color;

void main() {
    gl_Position = vec4(pos.x * screen.x - 1.0, 1.0 - pos.y * screen.y, 0.0, 1.0);
    v_uv = uv;
    v_color = color;
}
@end

@fs ui_fs
layout(binding=0) uniform texture2D tex;
layout(binding=0) uniform sampler smp;
in vec2 v_uv;
in vec4 v_color;
out vec4 frag_color;

void main() {
    frag_color = v_color * texture(sampler2D(tex, smp), v_uv);
}
@end

@program ui ui_vs ui_fs
