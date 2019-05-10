pub struct Lanes {
    vertex_buffer: Buffer<Resources, Vertex>,
    slice: Slice<Resources>,
}

impl Lanes {
    pub fn new(
        factory: Arc<Mutex<Factory>>,
        glsl: GLSL,
        output_color: (),
    ) -> LaneGraphics {
        // declare the vertices of the square of the lanes
        let vertices = [
            ([-1., -1.], 0.), // bottom left
            ([1., -1.], 1.),  // bottom rgiht
            ([1., 1.], 1.),   // top right
            ([-1., 1.], 0.),  // top left
        ]
        .into_iter()
        .map(|(p, t)| Vertex::new(*p, *t))
        .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[0, 1, 2, 2, 3, 0];

        // create the vertex buffer
        let (vertex_buffer, slice) =
            factory.create_vertex_buffer_with_slice(&vertices, vert_order);

        Lanes {
            vertex_buffer,
            slice,
        }
    }
}

// the texture of the lanes should not be located here. it should be located in
// either Pipelines or Assets as the texture of the lanes persists throughout
// the whole game
