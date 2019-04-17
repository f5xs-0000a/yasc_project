#[macro_use] use gfx;

use piston_window::AdvancedWindow;
use camera_controllers::FirstPerson;
use camera_controllers::FirstPersonSettings;
use image::GenericImageView;
use gfx::Factory as _;
use nalgebra::Matrix4;
use gfx::handle::Buffer;
use gfx::VertexBuffer;
use std::sync::Arc;
use parking_lot::RwLock;
use gfx::PipelineState;
use gfx::Slice;
use gfx_device_gl::Resources;
use gfx_device_gl::Factory;
use gfx::traits::FactoryExt as _;
use piston_window::PistonWindow;
use piston_window::OpenGL;
use piston_window::WindowSettings;
use shader_version::Shaders;
use shader_version::glsl::GLSL;
use gfx::handle::ShaderResourceView;

const vertex_shader: &str = r#"
    #version 330

    layout (location = 0) in vec3 vertex_pos;
    layout (location = 1) in float texture_coord;

    uniform mat4 transform;

    out float into_frag_tex_coord;

    void main() {
        vec4 padded_vec = vec4(
            vertex_pos[0],
            vertex_pos[1],
            vertex_pos[2],
            1.
        );

        into_frag_tex_coord = texture_coord;

        gl_Position = transform * padded_vec;
    }
"#;

const fragment_shader: &str = r#"
    #version 330

    in float into_frag_tex_coord;
    out vec4 color;
    
    uniform sampler1D raster_texture;

    void main() {
        vec4 tex = texture(raster_texture, into_frag_tex_coord);
        color = tex;
    }
"#;

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

gfx_vertex_struct!( Vertex {
    // the name must e the same as declared in the glslv file
    vertex_pos: [f32; 3] = "vertex_pos",
    tex_coord: f32 = "texture_coord",
});

impl Vertex {
    fn new(vertex_pos: [f32; 3], tex_coord: f32) -> Vertex {
        Vertex {
            vertex_pos,
            tex_coord,
        }
    }
}

lazy_static! {
    static ref PIPELINE
    : RwLock<Option<Arc<PipelineState<Resources, lane_pipe::Meta>>>>
    = RwLock::new(None);
}

fn get_pipeline(
    factory: &mut Factory,
    glsl: GLSL
) -> Arc<PipelineState<Resources, lane_pipe::Meta>> {
    { // try reading the pipeline
        let lock = PIPELINE.read();
        if let Some(ps) = (*lock).clone() {
            return ps;
        }
    }

    { // try to acquire a write lock on the pipeline so we can put a value in it
        let mut lock = PIPELINE.write();
        if lock.is_none() {
            // the pipeline should be inside a lazy static
            *lock = Some(Arc::new(
                factory.create_pipeline_simple(
                    Shaders::new()
                        .set(GLSL::V3_30, vertex_shader)
                        .get(glsl).unwrap().as_bytes(),
                    Shaders::new()
                        .set(GLSL::V3_30, fragment_shader)
                        .get(glsl).unwrap().as_bytes(),
                    lane_pipe::new()
                ).unwrap()
            ));
        }
    };

    { // read the pipeline again. for sure, it has a value now.
        let lock = PIPELINE.read();
        if let Some(ps) = (*lock).clone() {
            return ps;
        }

        else {
            unreachable!()
        }
    }
}

fn generate_lane_texture(factory: &mut Factory) -> ShaderResourceView<Resources, [f32; 4]> {
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
            vec.iter_mut()
                .zip(ch_iter)
                .for_each(|(mut to, from)| {
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

    rotation: f32,
    slant: f32,
    zoom: f32,
    perspective_amount: f32,

    first_person: FirstPerson,
}

impl LaneGraphics {
    pub fn new(
        factory: &mut Factory,
        glsl: GLSL,
        window: &PistonWindow
    ) -> LaneGraphics {
        // declare the vertices of the square of the lanes
        // front four, bl-br-tr-tl
        // back four, bl-br-tr-tl
        let vertices = [
            ([-1., -1., 0.], 0.), // front bottom left
            ([ 1., -1., 0.], 1.), // front bottom rgiht
            ([ 1.,  1., 0.], 1.), // front top right
            ([-1.,  1., 0.], 0.), // front top left
            /*
            ([-0.5, -0.5, 1.], 0.), // back bottom left
            ([ 0.5, -0.5, 1.], 1.), // back bottom right
            ([ 0.5,  0.5, 1.], 1.), // back top right
            ([-0.5,  0.5, 1.], 0.), // back top left
            */
        ]
            .into_iter()
            .map(|(p, t)| Vertex::new(*p, *t))
            .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[
            0, 1, 2, 2, 3, 0, // front
            //3, 2, 7, 7, 8, 2, // top
            //4, 0, 3, 3, 7, 4, // left
            //1, 5, 6, 6, 2, 1, // right
            //4, 5, 2, 2, 0, 4, // bottom
            //7, 6, 5, 5, 4, 7, // back
        ];

        // create the vertex buffer
        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(
            &vertices,
            vert_order
        );

        let lane_texture = generate_lane_texture(factory);

        LaneGraphics {
            vertex_buffer: vbuf,
            slice,
            texture_buffer: lane_texture,

            rotation: (0f32).to_radians(),
            slant: (30f32).to_radians(),
            zoom: 0.,
            perspective_amount: 0.,

            first_person: FirstPerson::new(
                [0., 0., 0.],
                FirstPersonSettings::keyboard_wasd(),
            ),
        }
    }

    pub fn get_transformation(&self) -> Matrix4<f32> {
        use nalgebra::geometry::{Transform, Rotation3, Perspective3, Isometry3};
        use nalgebra::base::Vector3;
        use nalgebra::base::Vector4;
        use nalgebra::base::Matrix4;
        use nalgebra::geometry::Point3;

        fn mvp(m: &Matrix4<f32>, v: &Matrix4<f32>, p: &Matrix4<f32>)
        -> Matrix4<f32> {
            p * (v * m)
        }

        // we only have three degrees of freedom on the transformation of the lanes:
        // rotation (the spin), slant, and zoom

        // 1. Offset upwards by half unit
        // 2. Rotate using slant
        // 3. Offset on z-axis (or just scale) using zoom
        // 3. Offset downwards by one unit
        // 4. Rotate using rotation
        // 5. Use perspective

        let model = 
            // rotate the lanes from a center point in the camera
            Rotation3::from_euler_angles(0., 0., self.rotation)
                .matrix()
                .to_homogeneous() *
            
            // move the lanes downward by 2 units
            Matrix4::new_translation(&Vector3::new(0., -2., 0.)) *
            //Matrix4::new_scaling(self.zoom + 1.) *
            
            // move the lanes away by a given constant
            Matrix4::new_translation(&Vector3::new(0., 0., -3.6)) *

            // slant the lanes
            Rotation3::from_euler_angles(-self.slant, 0., 0.)
                .matrix()
                .to_homogeneous() *

            // increase the vertical length of the lanes
            //Transform3::new_scaling(&Vector4::new(1., 2., 1., 1.)) *

            // move upwards by 1 unit
            Matrix4::new_translation(&Vector3::new(0., 1., 0.));
    
        let camera = self.first_person.camera(0.).orthogonal();
        let mut converted = [0.; 16];
        camera.iter().flat_map(|s| s.iter()).zip(converted.iter_mut())
            .for_each(|(from, to)| *to = *from);
        let view = Matrix4::from_column_slice(&converted);

        let projection = Perspective3::new(1., (60f32).to_radians(), 0., 1.);

        let post_transform = mvp(&model, &view, projection.as_matrix());

        post_transform
    }

    pub fn render_to(
        &mut self,
        window: &mut PistonWindow,
        factory: &mut Factory,
        glsl: GLSL
    ) {
        use gfx::texture::SamplerInfo;
        use gfx::texture::FilterMethod;
        use gfx::texture::WrapMode;

        // declare the sampler info
        // usually, this would be passed into here
        let sampler_info = SamplerInfo::new(
            FilterMethod::Anisotropic(4),
            WrapMode::Clamp,
        );

        // get the transformation of the lane
        let mut transform = [[0.; 4]; 4];
        self.get_transformation()
            .as_slice()
            .iter()
            .zip(transform.iter_mut().flat_map(|x| x.iter_mut()))
            .for_each(|(from, to)| {
                *to = *from;
            });

        // declare the data for the pipeline
        let data = lane_pipe::Data {
            vbuf: self.vertex_buffer.clone(),
            out_color: window.output_color.clone(),
            transform: transform,
            texture: (self.texture_buffer.clone(), factory.create_sampler(sampler_info)),
        };

        window.encoder.draw(&self.slice, &*get_pipeline(factory, glsl), &data);
    }

    pub fn adjust_rotation(&mut self, inc: bool) {
        let increment_amt = (core::f32::consts::PI * 2.) / 180.;

        if inc {
            self.rotation += increment_amt;
        }

        else {
            self.rotation -= increment_amt;
        }
    }

    pub fn adjust_slant(&mut self, inc: bool) {
        let increment_amt = (core::f32::consts::PI * 2.) / 180.;

        if inc {
            self.slant += increment_amt;
        }

        else {
            self.slant -= increment_amt;
        }
    }

    pub fn adjust_persp(&mut self, inc: bool) {
        let increment_amt = 0.0078125;

        if inc {
            self.perspective_amount += increment_amt;
        }

        else {
            self.perspective_amount -= increment_amt;
        }
    }

    pub fn adjust_zoom(&mut self, inc: bool) {
        let increment_amt = 0.0078125;

        if inc {
            self.zoom += increment_amt;
        }

        else {
            self.zoom -= increment_amt;
        }
    }
}

pub fn yeah() {
    use piston_window::Event as E;
    use piston_window::Loop;
    use piston_window::Input;
    use piston_window::Button::Keyboard;
    use piston_window::keyboard::Key as K;
    use piston_window::ButtonState;

    // declare which version of opengl to use
    let opengl = OpenGL::V3_3;

    // declare the window
    let mut window =
        WindowSettings::new("YAUSC Project", [360, 360])
        .exit_on_esc(true)
        .samples(4)
        .opengl(opengl)
        .vsync(true)
        .srgb(true)
        .build()
        .map(|w: PistonWindow| w.capture_cursor(true))
        .expect("Failed to create Piston window");

    // get the factory from the window. we'll be needing this.
    let ref mut factory = window.factory.clone();
    let glsl = opengl.to_glsl();

    // declare the graphics for the lanes
    let mut lanes = LaneGraphics::new(factory, glsl, &window);

    while let Some(e) = window.next() {
        lanes.first_person.event(&e);

        match &e {
            E::Input(b) => {

                // take only the buttons
                let b = match b {
                    Input::Button(b) => b,
                    _ => continue,
                };

                if b.state != ButtonState::Press {
                    continue;
                }

                // accept only the keyboard inputs
                let key = match b.button {
                    Keyboard(k) => k,
                    _ => continue,
                };

                dbg!(&lanes.first_person.position);

                match key {
                    K::O => lanes.adjust_rotation(true),
                    K::L => lanes.adjust_rotation(false),
                    K::I => lanes.adjust_slant(true),
                    K::K => lanes.adjust_slant(false),
                    K::U => lanes.adjust_zoom(true),
                    K::J => lanes.adjust_zoom(false),
                    K::Y => lanes.adjust_persp(true),
                    K::H => lanes.adjust_persp(false),
                    K::Return => {
                        lanes.first_person = FirstPerson::new(
                            [0., 0., 0.],
                            FirstPersonSettings::keyboard_wasd(),
                        );
                    },
                    _ => continue,
                }

                dbg!(lanes.rotation);
                dbg!(lanes.slant);
                dbg!(lanes.zoom);
                dbg!(lanes.perspective_amount);
            },

            E::Loop(r) => {
                match &r {
                    Loop::Render(r) => {},
                    _ => continue,
                }

                window.draw_3d(&e, |mut window| {
                    // clear the window
                    window.encoder.clear(&window.output_color, [0., 0., 0., 1.0]);

                    lanes.render_to(&mut window, factory, glsl);
                });
            },

            _ => {}
        }
    }
}
