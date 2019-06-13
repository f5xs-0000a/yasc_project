#version 330

in vec2 lanes_texture_coord;
in vec2 laser_texture_coord;

uniform sampler2D lanes_texture;
uniform sampler2D laser_texture;

out vec4 color;

////////////////////////////////////////////////////////////////////////////////

void main() {
    vec4 lanes_tex = texture(lanes_texture, lanes_texture_coord);
    vec4 laser_tex = texture(laser_texture, laser_texture_coord);

    color = pow(
        (sqrt(lanes_tex) + sqrt(laser_tex)) / 2.,
        vec4(2., 2., 2., 2.)
    );
}
