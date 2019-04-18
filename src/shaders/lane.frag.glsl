#version 330

layout (location = 0) in vec2 vertex_pos;
layout (location = 1) in float texture_coord;

uniform mat4 transform;

out float into_frag_tex_coord;

void main() {
    vec4 padded_vec = vec4(
        vertex_pos,
        0.,
        1.
    );

    into_frag_tex_coord = texture_coord;

    gl_Position = transform * padded_vec;
}
