// The blit program: the 3D pass, rendered at 1280 x 720 (engine-stack design section 5), drawn
// over the whole output with one triangle that needs no vertex buffer. Its source texture is
// sampled nearest or linear as the caller's sampler says.

@vs blit_vs
out vec2 uv;

void main() {
    vec2 p = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    uv = p;
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
@end

@fs blit_fs
layout(binding=0) uniform texture2D tex;
layout(binding=0) uniform sampler smp;

in vec2 uv;
out vec4 frag_color;

void main() {
    frag_color = vec4(texture(sampler2D(tex, smp), uv).rgb, 1.0);
}
@end

@program blit blit_vs blit_fs
