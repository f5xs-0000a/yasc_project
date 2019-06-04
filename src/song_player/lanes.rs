////////////////////////////////////////////////////////////////////////////////

pub struct LanesInitRequest {
    pub lane_image: DynamicImage,
}

impl LanesInitRequest {
    pub fn new<S>(img_buf: &[u8]) -> ImageResult<LanesInitRequest> {
        image::load_from_memory(img_buf)
            .map(|lane_image| LanesInitRequest { lane_image })
    }
    
    pub fn create_texture_buffer(self, factory: &mut Factory) -> Texture {
        Texture::from_image(
            factory,
            self.lane_image.to_rgba(),
            Texture_Settings::new(),
        )
    }
}

fn fulfill_lanes_init_request(
    req: Box<LanesInitRequest>,
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

    // create the texture
    let texture = self.create_texture_buffer(factory);

    Box::new(
        Lanes {
            pipeline,
            vertex_buffer,
            slice,
            texture,
        }
    )
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct LanesRenderRequest {
    respond_channel: OneshotSender<LanesRenderDetails>,
    texture: Texture<Resources, Srgba8>,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct LanesRenderDetails {
    pub vertex_buffer: Buffer<Resources, Vertex>,
    pub slice: Slice<Resources>,
    pub pipeline: PipelineState<Resources, LaneRenderPipeline::Meta>,

    pub transform: Arc<Matrix4>,
}

impl RenderDetails for LanesRenderDetails {
    fn render(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow;
        g2d: &mut Gfx2d<Resources>,
        output_color: &RenderTargetView<Resources, Srgba8>,
        output_stencil: &DepthStencilView<Resources, DepthStencil>,
    ) {
        // since we are rendering on a texture to be used by the governor (which
        // will be the one utilizing the transformation matrices), we're not
        // going to be using any transformation matrices
    
        // declare the data for the pipeline
        let data = lane_pipe::Data {
            vbuf:      self.vertex_buffer,
            out_color: window.output_color.clone(),
            transform: self.transform,
            lanes_texture,
            lasers_texture,
            laser_cutoff: LASER_CUTOFF,
        };
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Lanes {
    vertex_buffer: Buffer<Resources, Vertex>,
    slice: Slice<Resources>,
    pipeline: PipelineState<Resources, LaneRenderPipeline::Meta>,

    texture: Texture,
}

impl Actor for Lanes {
}

impl Handle<

// the texture of the lanes should not be located here. it should be located in
// either Pipelines or Assets as the texture of the lanes persists throughout
// the whole game
