#[macro_use]
use gfx;
use camera_controllers::{
    FirstPerson,
    FirstPersonSettings,
};
use cgmath::{
    Deg,
    Matrix4,
    Rad,
    Rotation3 as _,
};
use gfx::{
    handle::{
        Buffer,
        ShaderResourceView,
    },
    traits::FactoryExt as _,
    Factory as _,
    PipelineState,
    Slice,
};
use gfx_device_gl::{
    Factory,
    Resources,
};
use image::GenericImageView;
use parking_lot::RwLock;
use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
};
use shader_version::{
    glsl::GLSL,
    Shaders,
};
use std::sync::Arc;

////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref PIPELINE: RwLock<Option<Arc<PipelineState<Resources, lane_pipe::Meta>>>> =
        RwLock::new(None);
}

fn get_pipeline(
    factory: &mut Factory,
    glsl: GLSL,
) -> Arc<PipelineState<Resources, lane_pipe::Meta>>
{
    {
        // try reading the pipeline
        let lock = PIPELINE.read();
        if let Some(ps) = (*lock).clone() {
            return ps;
        }
    }

    {
        // try to acquire a write lock on the pipeline so we can put a value in
        // it
        let mut lock = PIPELINE.write();
        if lock.is_none() {
            *lock = Some(Arc::new(
                factory
                    .create_pipeline_simple(
                        Shaders::new()
                            .set(
                                GLSL::V3_30,
                                include_str!("shaders/lane.vert.glsl"),
                            )
                            .get(glsl)
                            .unwrap()
                            .as_bytes(),
                        Shaders::new()
                            .set(
                                GLSL::V3_30,
                                include_str!("shaders/lane.frag.glsl"),
                            )
                            .get(glsl)
                            .unwrap()
                            .as_bytes(),
                        lane_pipe::new(),
                    )
                    .unwrap(),
            ));
        }
    };

    {
        // read the pipeline again. for sure, it has a value now.
        let lock = PIPELINE.read();
        if let Some(ps) = (*lock).clone() {
            return ps;
        }
        else {
            unreachable!()
        }
    }
}

fn generate_lane_texture(
    factory: &mut Factory
) -> ShaderResourceView<Resources, [f32; 4]> {
    let image_bytes = include_bytes!("../build_assets/lane_texture.png");
    let image = image::load_from_memory(image_bytes).unwrap();

    let width = image.width() as u16;
    // let height = image.height() as u16;

    let data = image
        .to_rgba()
        .into_raw()
        .chunks(4)
        .map(|ch_iter| {
            let mut vec = [0; 4];
            vec.iter_mut().zip(ch_iter).for_each(|(to, from)| {
                *to = *from;
            });

            vec
        })
        .collect::<Vec<_>>();

    factory
        .create_texture_immutable::<gfx::format::Srgba8>(
            gfx::texture::Kind::D1(width),
            gfx::texture::Mipmap::Provided,
            &[&*data],
        )
        .unwrap()
        .1
}

pub struct LaneGraphics {
    vertex_buffer: Buffer<Resources, Vertex>,
    texture_buffer: ShaderResourceView<Resources, [f32; 4]>,
    slice: Slice<Resources>,

    rotation: Rad<f32>,
    slant:    Rad<f32>,
    zoom:     f32,

    first_person: FirstPerson,

    notes: crate::notes::Notes,
}

impl LaneGraphics {
    pub fn new(
        factory: &mut Factory,
        glsl: GLSL,
        window: &PistonWindow,
    ) -> LaneGraphics
    {
        // declare the vertices of the square of the lanes
        // front four, bl-br-tr-tl
        // back four, bl-br-tr-tl
        let vertices = [
            ([-1., -1.], 0.), // front bottom left
            ([1., -1.], 1.),  // front bottom rgiht
            ([1., 1.], 1.),   // front top right
            ([-1., 1.], 0.),  // front top left
        ]
        .into_iter()
        .map(|(p, t)| Vertex::new(*p, *t))
        .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[0, 1, 2, 2, 3, 0];

        // create the vertex buffer
        let (vbuf, slice) =
            factory.create_vertex_buffer_with_slice(&vertices, vert_order);

        let lane_texture = generate_lane_texture(factory);

        LaneGraphics {
            vertex_buffer: vbuf,
            slice,
            texture_buffer: lane_texture,

            rotation: Rad(0.),
            slant: Rad::from(Deg(36.5)),
            zoom: -0.9765625,

            // if fov = 60
            // slant = 52deg
            // zoom = -0.4375
            // len: 10.25
            first_person: FirstPerson::new(
                [0., 0., 0.],
                FirstPersonSettings::keyboard_wasd(),
            ),

            notes: crate::notes::Notes::new(
                factory,
                [vec![0.25], vec![0.1, 0.2], vec![0.09], vec![0.22]],
            ),
        }
    }

    pub fn get_transformation(&self) -> Matrix4<f32> {
        use cgmath::{
            Deg,
            PerspectiveFov,
            Quaternion,
            Rad,
            Vector3,
        };

        fn mvp(
            m: &Matrix4<f32>,
            v: &Matrix4<f32>,
            p: &Matrix4<f32>,
        ) -> Matrix4<f32>
        {
            p * (v * m)
        }

        // we only have three degrees of freedom on the transformation of the
        // lanes: rotation (the spin), slant, and zoom

        // 1. Offset upwards by half unit
        // 2. Rotate using slant
        // 3. Offset on z-axis (or just scale) using zoom
        // 3. Offset downwards by one unit
        // 4. Rotate using rotation
        // 5. Use perspective

        const BACK_OFFSET: f32 = -3.6;
        const VERT_SCALE: f32 = 10.25;

        let model = 
            // move the lanes away by a given constant
            Matrix4::from_translation(
                Vector3::new(
                    0.,
                    0.,
                    BACK_OFFSET * self.zoom.exp(),
                )
            ) *

            // slant the lanes
            Matrix4::from(
                Quaternion::from_axis_angle(
                    Vector3::new(1., 0., 0.),
                    -self.slant,
                )
            ) *

            // increase the vertical length of the lanes
            Matrix4::from_nonuniform_scale(1., VERT_SCALE, 1.) *

            // move upwards by 1 unit
            Matrix4::from_translation(Vector3::new(0., 1., 0.));

        let camera = self.first_person.camera(0.).orthogonal();
        let mut converted = [0.; 16];
        camera
            .iter()
            .flat_map(|s| s.iter())
            .zip(converted.iter_mut())
            .for_each(|(from, to)| *to = *from);
        let view = Matrix4::from(camera);

        let projection = Matrix4::from(PerspectiveFov {
            fovy:   Rad::from(Deg(90.)),
            aspect: 1.,
            near:   core::f32::MIN_POSITIVE,
            far:    1.,
        });

        let post_transform = mvp(&model, &view, &projection);

        // rotate the lanes from a center point in the camera
        Matrix4::from(
            Quaternion::from_axis_angle(
                Vector3::new(0., 0., 1.),
                self.rotation,
            )
        ) *

        // move the lanes' view downwards
        Matrix4::from_translation(Vector3::new(0., -0.975, 0.)) *

        post_transform
    }

    pub fn render_to(
        &mut self,
        window: &mut PistonWindow,
        factory: &mut Factory,
        glsl: GLSL,
    )
    {
        use gfx::texture::{
            FilterMethod,
            SamplerInfo,
            WrapMode,
        };

        // declare the sampler info
        // usually, this would be passed into here
        let sampler_info =
            SamplerInfo::new(FilterMethod::Anisotropic(4), WrapMode::Clamp);

        let transform = self.get_transformation();

        // declare the data for the pipeline
        let data = lane_pipe::Data {
            vbuf:      self.vertex_buffer.clone(),
            out_color: window.output_color.clone(),
            transform: transform.clone().into(),
            texture:   (
                self.texture_buffer.clone(),
                factory.create_sampler(sampler_info),
            ),
        };

        window.encoder.draw(&self.slice, &*get_pipeline(factory, glsl), &data);

        // render the notes
        self.notes.render_to(window, factory, glsl, 200., 0., transform.into());
    }

    pub fn adjust_rotation(
        &mut self,
        inc: bool,
    )
    {
        let increment_amt = (core::f32::consts::PI * 2.) / 180.;

        if inc {
            self.rotation.0 += increment_amt;
        }
        else {
            self.rotation.0 -= increment_amt;
        }
    }

    pub fn adjust_slant(
        &mut self,
        inc: bool,
    )
    {
        let increment_amt = (core::f32::consts::PI) / 720.;

        if inc {
            self.slant.0 += increment_amt;
        }
        else {
            self.slant.0 -= increment_amt;
        }
    }

    pub fn adjust_zoom(
        &mut self,
        inc: bool,
    )
    {
        let increment_amt = 0.0078125;

        if inc {
            self.zoom += increment_amt;
        }
        else {
            self.zoom -= increment_amt;
        }
    }
}
