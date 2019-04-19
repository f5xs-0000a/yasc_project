gfx_pipeline!( lane_pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),

    // the name must be the same as declared in the glslf file
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",

    // the name must be the same as declared in the shaders
    transform: gfx::Global<[[f32; 4]; 4]> = "transform",
    
    // the name must be the same as declared in the shaders
    texture: gfx::TextureSampler<[f32; 4]> = "raster_texture",
    //out_depth: gfx::DepthTarget<::gfx::format::DepthStencil> =
    //    gfx::preset::depth::LESS_EQUAL_WRITE,
});

gfx_vertex_struct!(Vertex {
    // the name must e the same as declared in the glslv file
    vertex_pos: [f32; 2] = "vertex_pos",
    tex_coord:  f32 = "texture_coord",
});

////////////////////////////////////////////////////////////////////////////////

impl Vertex {
    fn new(
        vertex_pos: [f32; 2],
        tex_coord: f32,
    ) -> Vertex
    {
        Vertex {
            vertex_pos,
            tex_coord,
        }
    }
}
