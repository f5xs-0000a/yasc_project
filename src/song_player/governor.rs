use crate::song_player::{
    keyframe::{
        Keyframe,
        TransformationKFCurve,
    },
    song_timer::SongTime,
};
use crate::pipelines::lane_governor::LaneGovernorRenderPipeline;
use camera_controllers::FirstPerson;
use cgmath::{
    Deg,
    Matrix4,
    PerspectiveFov,
    Quaternion,
    Rad,
    Rotation3,
    Vector3,
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LGRenderRequest;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct LGRenderDetails {
    transform: Arc<Matrix4<f32>>,

    lanes: OneshotReceiver<LanesRenderDetails>,
    bt_chips: OneshotReceiver<()>,
    bt_holds: OneshotReceiver<()>,
    fx_chips: OneshotReceiver<()>,
    fx_holds: OneshotReceiver<()>,
    lasers: OneshotReceiver<()>,

    pipeline: PipelineState<Resources, LaneGovernorRenderPipeline::Meta>,
    vbuf: Buffer<Resources, Corner>,
    slice: Slice<Resources>,
}

impl LGRenderDetails {
    pub fn new(
        transform: Arc<Matrix4<f32>>,
        pipeline: PipelineState<Resources, LaneGovernorRenderPipeline::Meta>,
        vbuf: Buffer<Resources, Corner>,
        slice: Slice<Resources>,
    ) -> (
        LGRenderDetails,
        OneshotSender<LanesRenderRequest>, // lanes
        OneshotSender<()>, // chip bts
        OneshotSender<()>, // hold bts
        OneshotSender<()>, // chip fxs
        OneshotSender<()>, // hold fxs
        OneshotSender<()>, // lasers
    ) {
        let lanes = channel();
        let bt_chips = channel();
        let bt_holds = channel();
        let fx_chips = channel();
        let fx_holds = channel();
        let lasers = channel();

        let lgrr = LGRenderDetails {
           transform,
           lanes: lanes.1,
           bt_chips: bt_chips.1,
           bt_holds: bt_holds.1,
           fx_chips: fx_chips.1,
           fx_holds: fx_holds.1,
           lasers: lasers.1,
        };

        (lgrr, lanes.0, bt_chips.0, bt_holds.0, fx_chips.0, fx.holds.0, lasers.0)
    }

    fn create_render_target_texture(
        &mut self,
        factory: &mut Factory,
        window: &GlutinWindow,
    ) -> Option<Texture<Resources>> {
        window.window
            .get_inner_size(|(w, h)| {
                let zero_all_image = ImageBuffer::new(
                    w,
                    h,
                    |_, _| Rgba { data: 0. },
                );

                Texture::from_image(
                    factory,
                    zero_all_image,
                    TextureSettings::new(),
                )
            })
    }

    fn render_lanes(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow,
        lanes: Texture<Resources>,
        lasers: Texture<Resources>,
    ) {
        // the amount of the laser, starting from the judgment line, that will
        // be shown to the player, since the notes and the lasers fall at
        // different speeds.
        //
        // the lower the value, the faster the lasers will fall
        const LASER_CUTOFF: f32 = 0.95;

        // declare the data for the pipeline
        let data = LaneGovernorRenderPipeline::Data {
            vbuf:      self.vertex_buffer,
            out_color: window.output_color.clone(),
            transform: self.transform,
            lanes_texture,
            lasers_texture,
            laser_cutoff: LASER_CUTOFF,
        };

        window.encoder.draw(&slice, &*get_pipeline(factory, glsl), &data);
    }
}

impl RenderDetails for LGRenderDetails {
    fn render(
        self,
        factory: &mut Factory,
        window: &mut GlutinWindow,
        g2d: &mut G2d<Resources>,
        output_color: &RenderTargetView<Resources, Srgba8>,
        output_stencil: &DepthStencilView<Resources, DepthStencil>,
    ) {
        // create a texture which will be utilized as a render target to render
        // the lanes and everything on
        let lane_texture = self.create_render_target_texture(factory);

        // and another one for the lasers
        let laser_texture = self.create_render_target_texture(factory);

        {
        // create a render target handle
        let render_target = factory.view_texture_as_render_target(
            &lane_texture,
            lane_texture.levels, 
            None,
        ).unwrap();

        // create another one for the lasers
        let laser_render_target = factory.view_texture_as_render_target(
            &laser_texture,
            laser_texture.levels, 
            None,
        ).unwrap();

        // bottom to top, this is the ordering of render:
        // Lanes -> FX Hold -> BT Hold -> FX Chip -> BT Chip -> Laser

        // render the lanes
        block_fn(|| self.lanes.wait());
            // .render(factory, g2d, &render_target, output_stencil);

        // render the fx holds
        block_fn(|| self.fx_holds.wait());
            // .render(factory, g2d, &render_target, output_stencil);
        
        // render the bt holds
        block_fn(|| self.bt_holds.wait());
            // .render(factory, g2d, &render_target, output_stencil);
        
        // render the fx chips
        block_fn(|| self.fx_chips.wait());
            // .render(factory, g2d, &render_target, output_stencil);
        
        // render the bt chips
        block_fn(|| self.bt_chips.wait());
            // .render(factory, g2d, &render_target, output_stencil);
        
        // render the lasers
        block_fn(|| self.lasers.wait());
            // .render(factory, g2d, &laser_render_target, output_stencil);

        // both render targets is dropped here
        }

        // then finally utilize the render target as a texture of a rectangle,
        // which would then be rendered on the screen
        self.render_lanes();
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct LGInitRequest {
    // keyframes
    rotation_events: Vec<(SongTime, Keyframe<TransformationKFCurve>)>,
    slant_events:    Vec<(SongTime, Keyframe<TransformationKFCurve>)>,
    zoom_events:     Vec<(SongTime, Keyframe<TransformationKFCurve>)>,
}

fn fulfill_lane_governor_init_request(
    request: Box<LGInitRequest>,
    factory: &mut Factory,
) -> Box<LaneGovernor> {
    let (vbuf, slice) = {
        // declare the vertices of the square of the lanes
        let vertices = vec![
                [-1., -1.],
                [1., -1.],
                [1., 1.],
                [-1., 1.],
            ]
            .into_iter()
            .map(|p| Corner::new(p))
            .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[0, 1, 2, 2, 3, 0];

        // create the vertex buffer
        factory.create_vertex_buffer_with_slice(&vertices, vert_order)
    };

    // create the pipeline
    let pipeline = factory.create_pipeline_simple(
        Shaders::new()
            .set(GLSL::V3_30, include_str!("../shaders/lane_governor.vert.glsl"))
            .get(glsl).unwrap().as_bytes(),
        Shaders::enw()
            .set(GLSL::V3_30, include_str!("../shaders/lane_governor.frag.glsl"))
            .get(glsl).unwrap().as_bytes(),
        LaneGovernorRenderPipeline::new(),
    );

    Box::new(
        LaneGovernor {
            // keyframes
            rotation_events: request.rotation_events,
            slant_events:    request.slant_events,
            zoom_events:     request.zoom_events,

            // current spin
            current_spin: None,

            pipeline,
            vbuf,
            slice,
        }
    )
}

impl LGInitRequest {
    pub(crate) fn debug_new() -> LaneGovernor {
        LaneGovernor {
            rotation_events: vec![],
            slant_events:    vec![],
            zoom_events:     vec![],

            current_spin: None,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

// TODO: maybe this should be an actor too since it handles renders and inputs?
#[derive(Debug)]
pub struct LaneGovernor {
    // keyframes
    rotation_events: Vec<(SongTime, Keyframe<TransformationKFCurve>)>,
    slant_events:    Vec<(SongTime, Keyframe<TransformationKFCurve>)>,
    zoom_events:     Vec<(SongTime, Keyframe<TransformationKFCurve>)>,

    // current spin
    current_spin: Option<Spin>,
    /* at this point, we have the drawable assets. they will be needing the
     * matrix provided to them by the calculate_matrix()
     *lanes: Lanes,
     *notes: Bt,
     *fx: Fx,
     *lasers: Lasers, */

    pipeline: PipelineState<Resources, LaneGovernorRenderPipeline::Meta>,
    vbuf: Buffer<Resources, Corner>,
    slice: Slice<Resources>,
}

// These constant values assume an FoV of Deg(90)
// In case of an FoV = Deg(60), use the commented values instead
// const DEFAULT_ZOOM: f32 = -0.4375;
// const DEFAULT_SLANT: Rad(f32) = Deg(52)
const DEFAULT_ROTATION: Rad<f32> = Rad(0.);
const DEFAULT_SLANT: Rad<f32> = Rad(0.6370451769779303); // Deg(36.5)
const DEFAULT_ZOOM: f32 = -0.9765625;

impl LaneGovernor {
    pub fn get_rotation_adjustment(
        &self,
        time: &SongTime,
    ) -> Rad<f32>
    {
        self.current_spin
            .clone()
            .map(|spin| spin.clamped_rotate(time))
            .unwrap_or(Rad(0.))
    }

    pub fn get_rotation_after_adjustment(
        &self,
        time: &SongTime,
    ) -> Rad<f32>
    {
        self.get_current_rotation(time)
            + self.get_rotation_after_adjustment(time)
    }

    pub fn get_current_rotation(
        &self,
        time: &SongTime,
    ) -> Rad<f32>
    {
        // if there are no rotation events, the rotation is just zero
        if self.rotation_events.is_empty() {
            return DEFAULT_ROTATION;
        }

        // find the index of the current rotation index given the time
        let search =
            self.rotation_events.binary_search_by_key(time, |(t, _)| *t);

        match search {
            Err(idx) => {
                // in case that the index found is within the first index and
                // the second to the last index, inclusive...
                if idx != self.rotation_events.len() {
                    Rad(self.rotation_events[idx - 1].1.interpolate_against(
                        time,
                        &self.rotation_events[idx].1,
                    ))
                }
                // in case that the index found is the last index, we just give
                // the output of the last
                else {
                    Rad(self.rotation_events[idx - 1].1.value())
                }
            },

            // if we have an exact match (which is highly unlikely)
            Ok(idx) => Rad(self.rotation_events[idx].1.value()),
        }
    }

    pub fn get_current_slant(
        &self,
        time: &SongTime,
    ) -> Rad<f32>
    {
        if self.slant_events.is_empty() {
            return DEFAULT_SLANT;
        }

        // find the index of the current rotation index given the time
        let search =
            self.slant_events.binary_search_by_key(time, |(t, _)| t.clone());

        match search {
            Err(idx) => {
                // in case that the index found is within the first index and
                // the second to the last index, inclusive...
                if idx != self.slant_events.len() {
                    Rad(self.slant_events[idx - 1]
                        .1
                        .interpolate_against(time, &self.slant_events[idx].1))
                }
                // in case that the index found is the last index, we just give
                // the output of the last
                else {
                    Rad(self.slant_events[idx - 1].1.value())
                }
            },

            // if we have an exact match (which is highly unlikely)
            Ok(idx) => Rad(self.slant_events[idx].1.value()),
        }
    }

    pub fn get_current_zoom(
        &self,
        time: &SongTime,
    ) -> f32
    {
        if self.zoom_events.is_empty() {
            return DEFAULT_ZOOM;
        }

        let search =
            self.zoom_events.binary_search_by_key(time, |(t, _)| t.clone());

        match search {
            Err(idx) => {
                // in case that the index found is within the first index and
                // the second to the last index, inclusive...
                if idx != self.zoom_events.len() {
                    self.zoom_events[idx - 1]
                        .1
                        .interpolate_against(time, &self.zoom_events[idx].1)
                }
                // in case that the index found is the last index, we just give
                // the output of the last
                else {
                    self.zoom_events[idx - 1].1.value()
                }
            },

            // if we have an exact match (which is highly unlikely)
            Ok(idx) => self.zoom_events[idx].1.value(),
        }
    }

    pub fn calculate_matrix(
        &self,
        time: &SongTime,
    ) -> Matrix4<f32>
    {
        const BACK_OFFSET: f32 = -3.6;
        const VERT_SCALE: f32 = 10.25;

        let rotation = self.get_rotation_after_adjustment(time);
        let slant = self.get_current_slant(time);
        let zoom = self.get_current_zoom(time);

        let model =
            // move the lanes away by a given constant
            Matrix4::from_translation(
                Vector3::new(
                    0.,
                    0.,
                    BACK_OFFSET * zoom.exp(),
                )
            ) *

            // slant the lanes
            Matrix4::from(
                Quaternion::from_axis_angle(
                    Vector3::new(1., 0., 0.),
                    -slant,
                )
            ) *

            // increase the vertical length of the lanes
            Matrix4::from_nonuniform_scale(1., VERT_SCALE, 1.) *

            // move upwards by 1 unit
            Matrix4::from_translation(Vector3::new(0., 1., 0.));

        let view = {
            let camera = get_default_first_person().camera(0.).orthogonal();
            let mut converted = [0.; 16];
            camera
                .iter()
                .flat_map(|s| s.iter())
                .zip(converted.iter_mut())
                .for_each(|(from, to)| *to = *from);

            Matrix4::from(camera)
        };

        let projection = Matrix4::from(PerspectiveFov {
            fovy:   Rad::from(Deg(90.)),
            aspect: 1.,
            near:   core::f32::MIN_POSITIVE,
            far:    1.,
        });

        let post_mvp = mvp(&model, &view, &projection);

        // rotate the lanes from a center point in the camera
        Matrix4::from(
            Quaternion::from_axis_angle(
                Vector3::new(0., 0., 1.),
                rotation,
            )
        ) *

        // move the lanes' view downwards
        Matrix4::from_translation(
            Vector3::new(0., -0.975, 0.)
        ) *

        post_mvp
    }
}

impl Actor for LaneGovernor {
}

impl Handles<LGRenderRequest> for LaneGovernor {
    type Response = LGRenderDetails;

    fn handle(
        &mut self,
        _: LGRenderDetails,
        _: &ContextImmutHalf<Self>
    ) -> Self::Response {
        // TODO: this None will be used in place of the actual song time for now
        let song_time = (None).unwrap_or(SongTime(0));
        let transform = Arc::new(self.calculate_matrix(&song_time));

        let (
            details,
            lanes_sender,
            bt_chips_sender,
            bt_holds_sender,
            fx_chips_sender,
            fx_holds_sender,
            lasers_sender,
        ) = LGRenderDetails::new(
            transform.clone(),
            self.pipeline.clone(),
            self.vbuf.clone(),
            self.slice.clone(),
        );

        lanes_sender.send(LanesRenderRequest);

        details
    }
}

fn mvp(
    m: &Matrix4<f32>,
    v: &Matrix4<f32>,
    p: &Matrix4<f32>,
) -> Matrix4<f32>
{
    p * (v * m)
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct SpinBuilder {
    pub duration:  SongTime,
    pub direction: bool,
    pub spin_type: SpinType,
}

#[derive(Debug, Clone)]
pub struct Spin {
    start:     SongTime,
    duration:  SongTime,
    direction: bool,
    spin_type: SpinType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum SpinType {
    Spin,
    Sway,
}

impl SpinBuilder {
    pub fn build(
        self,
        start: SongTime,
    ) -> Spin
    {
        Spin {
            start,
            duration: self.duration,
            direction: self.direction,
            spin_type: self.spin_type,
        }
    }
}

impl SpinType {
    /// Returns the corresponding rotation given a time value.
    ///
    /// The time value should be within (0, 1). If outside the range, the
    /// function will return 0 (as if there is no rotation).
    pub fn clamped_rotate(
        &self,
        time_val: f32,
    ) -> Rad<f32>
    {
        use SpinType::*;

        // if outside the range of (0, 1)
        if !(0. < time_val && time_val < 1.) {
            return Rad(0.);
        }

        match self {
            Spin => {
                unimplemented!()
                // utilize envelopes in here
                // there is an envelope crate out there
            },

            Sway => unimplemented!(),
        }
    }
}

impl Spin {
    pub fn clamped_rotate(
        &self,
        time_val: &SongTime,
    ) -> Rad<f32>
    {
        if !(self.start < *time_val && *time_val < self.start + self.duration) {
            return Rad(0.);
        }

        let progress =
            (time_val.0 - self.start.0) as f32 / self.duration.0 as f32;

        self.spin_type.clamped_rotate(progress)
    }
}

fn get_default_first_person() -> FirstPerson {
    use camera_controllers::FirstPersonSettings;

    FirstPerson::new(
        [0., 0., 0.],
        camera_controllers::FirstPersonSettings::keyboard_wasd(),
    )
}

////////////////////////////////////////////////////////////////////////////////
