#version 330

layout (location = 0) in vec2 vertex_pos;
layout (location = 1) in vec2 texture_coord;

out vec2 into_frag_tex_coord;

void main() {
    vec4 padded_vec = vec4(
        vertex_pos,
        0.,
        1.
    );

    into_frag_tex_coord = texture_coord;

    gl_Position = padded_vec;
}
