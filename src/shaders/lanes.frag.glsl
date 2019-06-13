#version 330

in vec2 into_frag_tex_coord;

uniform sampler2D raster_texture;

out vec4 color;

void main() {
    vec4 lanes_tex = texture(raster_texture, into_frag_tex_coord);
    color = lanes_tex;
}
