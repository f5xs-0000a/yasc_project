#version 330

in vec2 texture_coord;
out vec4 color;

uniform sampler2D raster_texture;

void main() {
    vec4 tex = texture(raster_texture, texture_coord);
    color = tex;
}
