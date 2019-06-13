use crate::{
    environment::{
        actor_wrapper::{
            ActorWrapper,
            ContextWrapper,
            RenderDetails,
            RenderPayload,
            RenderableActorWrapper,
            UpdatePayload,
        },
        update_routine::CanBeWindowHandled,
        RenderWindowParts,
        UpdateWindowParts,
    },
    pipelines::lanes::*,
};
use gfx::{
    format::Srgba8,
    handle::{
        Buffer,
        RenderTargetView,
    },
    pso::PipelineState,
    traits::FactoryExt as _,
    Slice,
};
use gfx_device_gl::Resources;
use gfx_graphics::{
    Texture,
    TextureSettings,
};
use image::{
    ImageResult,
    RgbaImage,
};
use shader_version::{
    glsl::GLSL,
    Shaders,
};

////////////////////////////////////////////////////////////////////////////////

pub struct LanesInitRequest {
    pub lane_image: RgbaImage,
}

impl LanesInitRequest {
    pub fn new(img_buf: &[u8]) -> ImageResult<LanesInitRequest> {
        image::load_from_memory(img_buf)
            .map(|lane_image| lane_image.to_rgba())
            .map(|lane_image| {
                LanesInitRequest {
                    lane_image,
                }
            })
    }

    pub fn debug_new() -> ImageResult<LanesInitRequest> {
        let lane_tex = include_bytes!("../../build_assets/lane_texture.png");
        LanesInitRequest::new(lane_tex)
    }

    fn create_texture_buffer<'a>(
        self,
        uwp: &mut UpdateWindowParts<'a>,
    ) -> Texture<Resources>
    {
        Texture::from_image(
            &mut uwp.tex_ctx,
            &self.lane_image,
            &TextureSettings::new(),
        )
        .unwrap()
    }
}

impl CanBeWindowHandled for LanesInitRequest {
    type Response = Lanes;

    fn handle<'a>(
        self,
        uwp: &mut UpdateWindowParts<'a>,
    ) -> Self::Response
    {
        use crate::pipelines::lanes::*;

        // create the pipeline
        let pipeline = uwp
            .tex_ctx
            .factory
            .create_pipeline_simple(
                Shaders::new()
                    .set(
                        GLSL::V3_30,
                        include_str!("../shaders/lanes.vert.glsl"),
                    )
                    .get(uwp.glsl)
                    .unwrap()
                    .as_bytes(),
                Shaders::new()
                    .set(
                        GLSL::V3_30,
                        include_str!("../shaders/lanes.frag.glsl"),
                    )
                    .get(uwp.glsl)
                    .unwrap()
                    .as_bytes(),
                LaneRenderPipeline::new(),
            )
            .unwrap();

        // declare the vertices of the square of the lanes
        let vertices = [
            [-1., -1.], // bottom left
            [1., -1.],  // bottom rgiht
            [1., 1.],   // top right
            [-1., 1.],  // top left
        ]
        .into_iter()
        .map(|p| Corner::new(*p))
        .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[0, 1, 2, 2, 3, 0];

        // create the vertex buffer
        let (vertex_buffer, slice) = uwp
            .tex_ctx
            .factory
            .create_vertex_buffer_with_slice(&vertices, vert_order);

        // create the texture
        let texture = self.create_texture_buffer(uwp);

        Lanes {
            pipeline,
            vertex_buffer,
            slice,
            texture,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone)]
pub struct LanesRenderDetails {
    texture:      Texture<Resources>,
    color_target: RenderTargetView<Resources, Srgba8>,

    vertex_buffer: Buffer<Resources, Corner>,
    slice:         Slice<Resources>,
    pipeline:      PipelineState<Resources, LaneRenderPipeline::Meta>,
}

impl RenderDetails for LanesRenderDetails {
    fn render<'a>(
        self,
        rwp: &mut RenderWindowParts<'a>,
    )
    {
        let data = LaneRenderPipeline::Data {
            vbuf:      self.vertex_buffer,
            out_color: self.color_target,
            texture:   (self.texture.view, self.texture.sampler),
        };

        rwp.tex_ctx.encoder.draw(&self.slice, &self.pipeline, &data);
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Lanes {
    vertex_buffer: Buffer<Resources, Corner>,
    slice:         Slice<Resources>,
    pipeline:      PipelineState<Resources, LaneRenderPipeline::Meta>,

    texture: Texture<Resources>,
}

impl ActorWrapper for Lanes {
    type Payload = ();

    fn update(
        &mut self,
        _payload: UpdatePayload<Self::Payload>,
        _ctx: &ContextWrapper<Self>,
    )
    {
        // do nothing. this doesn't even need to update
    }
}

impl RenderableActorWrapper for Lanes {
    type Details = LanesRenderDetails;
    type Payload = ();

    fn emit_render_details(
        &mut self,
        payload: RenderPayload<()>,
        _: &ContextWrapper<Self>,
    ) -> Self::Details
    {
        LanesRenderDetails {
            texture:      self.texture.clone(),
            color_target: payload.color_target,

            vertex_buffer: self.vertex_buffer.clone(),
            slice:         self.slice.clone(),
            pipeline:      self.pipeline.clone(),
        }
    }
}
