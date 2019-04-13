#[macro_use] use gfx;

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

const vertex_shader: &str = r#"
    #version 330

    layout (location = 0) in vec3 vertex_pos;

    uniform mat4 transform;

    void main() {
        vec4 padded_vec = vec4(
            vertex_pos[0],
            vertex_pos[1],
            vertex_pos[2],
            1.
        );

        gl_Position = transform * padded_vec;
    }
"#;

const fragment_shader: &str = r#"
    #version 330

    out vec4 color;

    uniform mat4 transform;

    void main() {
       color = vec4(1.0f, 0.5f, 0.2f, 1.0f);
    }
"#;

gfx_pipeline!( lane_pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),

    // the name must be the same as declared in the glslf file
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",

    // the name must be the same as declared in the shaders
    transform: gfx::Global<[[f32; 4]; 4]> = "transform",
    
    //t_color: gfx::TextureSampler<[f32; 4]> = "t_color",
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

pub struct LaneGraphics {
    vertex_buffer: Buffer<Resources, Vertex>,
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

        let transform = [
            [1., 0., 0., 0.],
            [0., 1.5, 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.]
        ];

        LaneGraphics {
            vertex_buffer: vbuf,
            slice,

            rotation: 0.,
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

        Rotation3::from_euler_angles(self.rotation, 0., 0.)
            .matrix()
            .to_homogeneous() *
        Matrix4::new_translation(&Vector3::new(0., -1., 0.)) *
        Matrix4::new_scaling(self.zoom + 1.) *
        Rotation3::from_euler_angles(0., self.slant, 0.)
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
        };

        window.encoder.draw(&self.slice, &*get_pipeline(factory, glsl), &data);
    }
}

pub fn yeah() {
    // declare which version of opengl to use
    let opengl = OpenGL::V3_3;

    // declare the window
    let mut window: PistonWindow =
        WindowSettings::new("YAUSC Project", [1366, 768])
        .exit_on_esc(true)
        .samples(4)
        .opengl(opengl)
        .build()
        .expect("Failed to create Piston window");

    // get the factory from the window. we'll be needing this.
    let ref mut factory = window.factory.clone();
    let glsl = opengl.to_glsl();

    // declare the graphics for the lanes
    let mut lanes = LaneGraphics::new(factory, glsl, &window);

    while let Some(e) = window.next() {
        window.draw_3d(&e, |mut window| {
            // clear the window
            window.encoder.clear(&window.output_color, [0.2, 0.3, 0.3, 1.0]);

            lanes.render_to(&mut window, factory, glsl);
        });
    }
}
