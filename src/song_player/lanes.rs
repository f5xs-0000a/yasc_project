pub struct LanesInitRequest;

fn fulfill_lanes_init_request(
    request: Box<LanesInitRequest>,
    factory: &mut Factory,
) -> Box<LaneGovernor> {
    // create the pipeline
    let pipeline = factory.create_pipeline_simple(
        Shaders::new()
            .set(GLSL::V3_30, include_str!("shaders/lanes.vert.glsl"))
            .get(glsl).unwrap().as_bytes(),
        Shaders::enw()
            .set(GLSL::V3_30, include_str!("shaders/lanes.frag.glsl"))
            .get(glsl).unwrap().as_bytes(),
        LaneRenderPipeline::new(),
    );

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

    Box::new(
        Lanes {
            pipeline,
            vertex_buffer,
            slice,
        }
    )
}

////////////////////////////////////////////////////////////////////////////////

pub struct LanesRenderRequest {
    pub vertex_buffer: Buffer<Resources, Vertex>,
    pub slice: Slice<Resources>,
    pub pipeline: PipelineState<Resources, LaneRenderPipeline::Meta>,

    pub transform: Arc<Matrix4>,
}


////////////////////////////////////////////////////////////////////////////////

pub struct Lanes {
    vertex_buffer: Buffer<Resources, Vertex>,
    slice: Slice<Resources>,
    pipeline: 
}

impl Lanes {
}

// the texture of the lanes should not be located here. it should be located in
// either Pipelines or Assets as the texture of the lanes persists throughout
// the whole game
