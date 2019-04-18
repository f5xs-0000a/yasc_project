#version 330

in float into_frag_tex_coord;
out vec4 color;

uniform sampler1D raster_texture;

void main() {
    vec4 tex = texture(raster_texture, into_frag_tex_coord);
    color = tex;
}
