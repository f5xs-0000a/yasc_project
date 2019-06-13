#version 330

layout (location = 0) in vec2 vertex_pos;

uniform float laser_cutoff;
uniform mat4 transform;

// we emit from this shader the coordinates of the texture of the lanes and
// lasers
out vec2 lanes_texture_coord;
out vec2 laser_texture_coord;

////////////////////////////////////////////////////////////////////////////////

float linear_map(
    float x_i,
    float x_min,
    float x_max,
    float y_min,
    float y_max
) {
    return (x_i - x_min) / (x_max - x_min) * (y_max - y_min) + y_min;
}

// since the vertex lies in [-1, 1]^2 space while the texture lies in [0, 1]^2
// space, we need a function that maps the vertex space to texture space (which
// is just a two-dimensional linear map)
vec2 vert_to_tex_mapper(vec2 pos) {
    pos[0] = linear_map(pos[0], -1., 1., 0., 1.);
    pos[1] = linear_map(pos[1], -1., 1., 0., 1.);

    return pos;
}

void main() {
    // declare the coordinates of the texture of the vertex
    lanes_texture_coord = vert_to_tex_mapper(vertex_pos);
    laser_texture_coord = lanes_texture_coord;
    laser_texture_coord[1] *= laser_cutoff;

    // map the position of the vectors according to the transformation matrix
    vec4 padded_vec = vec4(
        vertex_pos,
        0.,
        1.
    );
    gl_Position = transform * padded_vec;
}
