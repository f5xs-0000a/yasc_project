use gfx;

////////////////////////////////////////////////////////////////////////////////

gfx_pipeline!( LaneGovernorRenderPipeline {
    vbuf: gfx::VertexBuffer<Corner> = (),

    // the name must be the same as declared in the glslf file
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",

    // the name must be the same as declared in the shaders
    transform: gfx::Global<[[f32; 4]; 4]> = "transform",

    // the name must be the same as declared in the shaders
    lanes_texture: gfx::TextureSampler<[f32; 4]> = "lanes_texture",
    lasers_texture: gfx::TextureSampler<[f32; 4]> = "laser_texture",

    lasers_cutoff: gfx::Global<f32> = "laser_cutoff",
});

gfx_vertex_struct!(Corner {
    // the name must be the same as declared in the glslv file
    vertex_pos: [f32; 2] = "vertex_pos",
});

////////////////////////////////////////////////////////////////////////////////

impl Corner {
    pub fn new(vertex_pos: [f32; 2]) -> Corner {
        Corner {
            vertex_pos,
        }
    }
}
