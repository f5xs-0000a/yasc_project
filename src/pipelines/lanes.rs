use gfx;

////////////////////////////////////////////////////////////////////////////////

gfx_pipeline!( LaneRenderPipeline {
    vbuf: gfx::VertexBuffer<Corner> = (),

    // the name must be the same as declared in the glslf file
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",

    // the name must be the same as declared in the shaders
    texture: gfx::TextureSampler<[f32; 4]> = "raster_texture",
});

gfx_vertex_struct!(Corner {
    // the name must be the same as declared in the glslv file
    vertex_pos: [f32; 2] = "vertex_pos",
    tex_coord:  [f32; 2] = "texture_coord",
});

////////////////////////////////////////////////////////////////////////////////

impl Corner {
    fn new(
        vertex_pos: [f32; 2],
        tex_coord: f32,
    ) -> Corner
    {
        Corner {
            vertex_pos,
            tex_coord,
        }
    }
}
