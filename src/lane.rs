#[macro_use] use gfx;

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
    out float texture_coord;

    uniform mat4 transform;

    void main() {
        vec4 padded_vec = vec4(
            vertex_pos[0],
            vertex_pos[1],
            vertex_pos[2],
            1.
        );

        texture_coord = vertex_pos[0];

        gl_Position = transform * padded_vec;
    }
"#;

const fragment_shader: &str = r#"
    #version 330

    in float texture_coord;
    out vec4 color;
    
    uniform mat4 transform;
    uniform sampler1D raster_texture;

    void main() {
        vec4 tex = texture(raster_texture, texture_coord);
        //color = tex;
        color = vec4(1., 1., 1., 0.);
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
});

impl Vertex {
    fn new(pos: [f32; 3]) -> Vertex {
        Vertex {
            vertex_pos: [pos[0], pos[1], pos[2]],
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
    const resolution: usize = 2048;

    pub fn to_f(space: &[u8; 3]) -> [f32; 3] {
        [
            space[0] as f32 / 255.,
            space[1] as f32 / 255.,
            space[2] as f32 / 255.,
        ]
    }

    pub fn blend(a: &[f32; 3], b: &[f32; 3], a_rate: f32) -> [f32; 3] {
        fn proper_blend(a: f32, b: f32, a_rate: f32) -> f32 {
            let sqrtable = a.powi(2) * a_rate + b.powi(2) * (1. - a_rate);

            if sqrtable == 0. {
                0.
            }

            else {
                sqrtable.sqrt()
            }
        }

        [
            proper_blend(a[0], b[0], a_rate),
            proper_blend(a[1], b[1], a_rate),
            proper_blend(a[2], b[2], a_rate),
        ]
    }

    pub fn fill(
        texture: &mut [[f32; 3]; 2048],
        color: &[u8; 3],
        area: (f32, f32), // within [-1, 1]
    ) {
        let res_size = (resolution as f32).recip();
        let color = to_f(color);

        texture.iter_mut()
            .enumerate()
            .map(|(i, x)| (i as f32 * res_size * 2. - 1., x))
            .for_each(|(i, tex)| {
                if i < area.0 - res_size {
                    return;
                }

                else if i > area.1 + res_size {
                    return;
                }

                else {
                    if i < area.0 && area.0 < i + res_size {
                        // assume texture is A in blending
                        let blend_amt = 1. - ((i - area.0) / res_size);
                        *tex = blend(tex, &color, blend_amt);
                    }

                    else if i < area.1 && area.1 < i + res_size {
                        // assume texture is A in blending
                        let blend_amt = (i - area.1) / res_size;
                        *tex = blend(tex, &color, blend_amt);
                    }

                    else {
                        *tex = color.clone();
                    }
                }
            })
    }

    let bt_fill = [0, 0, 0];
    let left_fill = [1, 11, 20];
    let right_fill = [22, 0, 3];

    let bt_line = [176, 176, 176];
    let bt_left_line = [15, 255, 243];
    let bt_right_line = [248, 27, 132];

    let left_line = [35, 142, 158];
    let right_line = [176, 16, 86];

    let bc_line = 0.;
    let cd_line = (267 - 17) as f32 / (382 - 17) as f32;
    let laser_bt_line = (337 - 17) as f32 / (382 - 17) as f32;
    let edge_line = 1.;

    let bt_line_thickness = 0.05;
    let bt_laser_thickness = 0.15;
    let edge_thickness = 0.075;

    let mut texture = [[0.; 3]; resolution];
    // fill the fills first
    fill(&mut texture, &bt_fill, (-cd_line, cd_line));
    fill(&mut texture, &left_fill, (-1., -cd_line));
    fill(&mut texture, &right_fill, (cd_line, 1.));

    // then the bt lines
    fill(
        &mut texture,
        &bt_line,
        (
            -bt_line_thickness / 2.,
            bt_line_thickness / 2.,
        ),
    );
    fill(
        &mut texture,
        &bt_line,
        (
            cd_line - bt_line_thickness / 2.,
            cd_line + bt_line_thickness / 2.,
        ),
    );
    fill(
        &mut texture,
        &bt_line,
        (
            -cd_line - bt_line_thickness / 2.,
            -cd_line + bt_line_thickness / 2.,
        ),
    );

    // then the fx lines
    fill(
        &mut texture,
        &bt_left_line,
        (
            -laser_bt_line + bt_laser_thickness / 2.,
            -laser_bt_line - bt_laser_thickness / 2.,
        ),
    );
    fill(
        &mut texture,
        &bt_right_line,
        (
            laser_bt_line + bt_laser_thickness / 2.,
            laser_bt_line - bt_laser_thickness / 2.,
        ),
    );

    // then the edge lines
    fill(
        &mut texture,
        &left_line,
        (
            0.,
            edge_thickness,
        )
    );

    // then the edge lines
    fill(
        &mut texture,
        &right_line,
        (
            1. - edge_thickness,
            1.,
        )
    );

    // convert the texture
    let mut conv_texture = [[0u8; 4]; resolution];
    conv_texture
        .iter_mut()
        .zip(texture.into_iter())
        .for_each(|(to, from)| {
            to[0] = (from[0] * 255.).round() as u8;
            to[1] = (from[1] * 255.).round() as u8;
            to[2] = (from[2] * 255.).round() as u8;
            to[3] = 255;
        });

    // then generate the texture buffer
    factory.create_texture_immutable::<gfx::format::Rgba8>(
        gfx::texture::Kind::D1(resolution as u16),
        gfx::texture::Mipmap::Provided,
        &[&conv_texture]
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
}

impl LaneGraphics {
    pub fn new(
        factory: &mut Factory,
        glsl: GLSL,
        window: &PistonWindow
    ) -> LaneGraphics {
        // declare the vertices of the square of the lanes
        let vertices = [
            [-0.5, -0.5, 0.],
            [ 0.5, -0.5, 0.],
            [ 0.5,  0.5, 0.],
            [-0.5,  0.5, 0.],

        ]
            .into_iter()
            .map(|x| Vertex::new(*x))
            .collect::<Vec<_>>();

        // declare the ordering of indices how we're going to render the
        // triangle
        let vert_order: &[u16] = &[
            0, 1, 2,
            2, 3, 0,
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

            rotation: (60f32).to_radians(),
            slant: (30f32).to_radians(),
            zoom: 0.,
        }
    }

    pub fn get_transformation(&self) -> Matrix4<f32> {
        use nalgebra::geometry::{Transform, Rotation3};
        use nalgebra::base::Vector3;
        use nalgebra::base::Matrix4;

        // 1. Offset upwards by half unit
        // 2. Rotate using slant
        // 5. Offset on z-axis (or just scale) using zoom
        // 3. Offset downwards by one unit
        // 4. Rotate using rotation
        // 5. Use perspective

        Rotation3::from_euler_angles(0., 0., self.rotation)
            .matrix()
            .to_homogeneous() *
        Matrix4::new_translation(&Vector3::new(0., -1., 0.)) *
        Matrix4::new_scaling(self.zoom + 1.) *
        Rotation3::from_euler_angles(self.slant, 0., 0.)
            .matrix()
            .to_homogeneous() *
        Matrix4::new_translation(&Vector3::new(0., 0.5, 0.))
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
            .for_each(|(from, mut to)| {
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

    pub fn adjust_zoom(&mut self, inc: bool) {
        // this does nothing as of the moment
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
    let mut window: PistonWindow =
        WindowSettings::new("YAUSC Project", [360, 360])
        .exit_on_esc(true)
        .samples(4)
        .opengl(opengl)
        .vsync(true)
        .srgb(true)
        .build()
        .expect("Failed to create Piston window");

    // get the factory from the window. we'll be needing this.
    let ref mut factory = window.factory.clone();
    let glsl = opengl.to_glsl();

    // declare the graphics for the lanes
    let mut lanes = LaneGraphics::new(factory, glsl, &window);

    while let Some(e) = window.next() {
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

                match key {
                    K::R => lanes.adjust_rotation(true),
                    K::F => lanes.adjust_rotation(false),
                    K::E => lanes.adjust_slant(true),
                    K::D => lanes.adjust_slant(false),
                    K::W => lanes.adjust_zoom(true),
                    K::S => lanes.adjust_zoom(false),
                    _ => continue,
                }
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