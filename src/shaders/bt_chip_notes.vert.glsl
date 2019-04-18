#version 330

layout (location = 0) in vec2 note_pos;
layout (location = 1) in int note_index;
layout (location = 2) in int corner_type;

uniform float song_offset;
uniform float hi_speed;
uniform float note_graphic_height;
uniform mat4 transform;

out vec2 texture_coord;

void main() {
    // determine the vertex' real center
    vec2 cur_pos = note_pos;
    cur_pos[1] = (cur_pos[1] - song_offset) * hi_speed;

    vec2 new_note_pos;
    switch (corner_type) {
        case 0: // upper left
            new_note_pos = cur_pos + vec2(-0.5, note_graphic_height);
            texture_coord = vec2(0., 1.);
            break;

        case 1: // upper right
            new_note_pos = cur_pos + vec2(0.5, note_graphic_height);
            texture_coord = vec2(1., 1.);
            break;

        case 2: // lower left
            new_note_pos = cur_pos + vec2(-0.5, 0.);
            texture_coord = vec2(0., 0.);
            break;

        case 3: // lower right
            new_note_pos = cur_pos + vec2(0.5, 0.);
            texture_coord = vec2(1., 0.);
            break;
    }

    gl_Position = transform * vec4(new_note_pos, 0., 1.);
}
