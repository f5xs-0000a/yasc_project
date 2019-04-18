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
use core::iter::repeat;
use gfx::{
    self,
    {
        handle::{
            Buffer,
            ShaderResourceView,
        },
        traits::FactoryExt as _,
        Factory as _,
        PipelineState,
        Slice,
        VertexBuffer,
    },
};
use gfx_device_gl::{
    Factory,
    Resources,
};
use image::GenericImageView;
use parking_lot::RwLock;
use piston_window::{
    AdvancedWindow,
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

const vertex_shader: &str = r#"
    #version 330

    layout (location = 0) in vec2 vertex_pos;
    layout (location = 1) in int note_index;

    out vec2 gs_vertex_pos;

    //uniform bool* note_index_renderable;
    //uniform int note_index_len;
    //uniform int note_index_offset;

    void main() {
    /*
        // accept only those within the range
        if (
            note_index >= note_index_offset &&
            note_index < note_index_offset + note_index_len
        ) {

            // iterate through the list, finding the note that has the same
            // index as the current invocation's note_index
            // TODO: it's possible to use a binary search for this but not now.
            for (
                int idx = note_index_offset;
                idx < note_index_offset + note_index_len;
                idx += 1
            ) {

                // forward to the geometry shader only if it's flagged for
                // render
                if (idx == note_index) {
                    gl_Position = vec4(
                        vertex_pos,
                        0.,
                        1.
                    );

                    break;
                }
            }
        }
        */

        gs_vertex_pos = vertex_pos;
    }
"#;

const geometry_shader: &str = r#"
    #version 330

    layout (points) in;
    
    layout (
        triangle_strip,
        max_vertices = 4
    ) out;
    out vec2 texture_coord;

    uniform float song_offset;
    uniform float hi_speed;
    uniform float note_graphic_height;
    uniform mat4 transform;

    void main() {
        // determine the vertex' real center
        vec4 cur_pos = gl_in[0].gl_Position;
        cur_pos[1] = (cur_pos[1] - song_offset) * hi_speed;

        // draw the upper left corner of the rectangle
        gl_Position = transform * (cur_pos + vec4(-0.5, note_graphic_height, 0., 0.));
        texture_coord = vec2(0., 1.);
        EmitVertex();

        // draw the upper right corner of the rectangle
        gl_Position = transform * (cur_pos + vec4(0.5, note_graphic_height, 0., 0.));
        texture_coord = vec2(1., 1.);
        EmitVertex();

        // draw the lower left corner of the rectangle
        gl_Position = transform * (cur_pos + vec4(-0.5, 0., 0., 0.));
        texture_coord = vec2(0., 0.);
        EmitVertex();

        // draw the lower right corner of the rectangle
        gl_Position = transform * (cur_pos + vec4(0.5, 0., 0., 0.));
        texture_coord = vec2(1., 0.);
        EmitVertex();

        EndPrimitive();
    }
"#;

const fragment_shader: &str = r#"
    #version 330
    
    in vec2 texture_coord;
    out vec4 color;
    
    uniform sampler2D raster_texture;

    void main() {
        vec4 tex = texture(raster_texture, texture_coord);
        color = tex;
    }
"#;

gfx_pipeline!( note_pipe {
    note_buffer: gfx::VertexBuffer<NoteLocation> = (),
    transform: gfx::Global<[[f32; 4]; 4]> = "transform",
    //note_index_offset: gfx::Global<i32> = "note_index_offset",
    //note_index_renderable: gfx::Global<[bool]> = "note_index_renderable",
    //note_index_len: gfx::Global<u32> = "note_index_len",
    hi_speed: gfx::Global<f32> = "hi_speed",
    song_offset: gfx::Global<f32> = "song_offset",
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",
    texture_buffer: gfx::TextureSampler<[f32; 4]> = "raster_texture",

    note_graphic_height: gfx::Global<f32> = "note_graphic_height",
});

gfx_vertex_struct!(NoteLocation {
    vertex_pos: [f32; 2] = "vertex_pos",
    index:      u32 = "note_index",
});

lazy_static! {
    static ref PIPELINE: RwLock<Option<Arc<PipelineState<Resources, note_pipe::Meta>>>> =
        RwLock::new(None);
}

fn get_pipeline(
    factory: &mut Factory,
    glsl: GLSL,
) -> Arc<PipelineState<Resources, note_pipe::Meta>>
{
    use gfx::{
        state::Rasterizer,
        Primitive,
    };

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
            let rasterizer = Rasterizer::new_fill();
            let prim_type = Primitive::TriangleList;
            let shader_set = factory
                .create_shader_set_geometry(
                    Shaders::new()
                        .set(GLSL::V3_30, vertex_shader)
                        .get(glsl)
                        .unwrap()
                        .as_bytes(),
                    Shaders::new()
                        .set(GLSL::V3_30, geometry_shader)
                        .get(glsl)
                        .unwrap()
                        .as_bytes(),
                    Shaders::new()
                        .set(GLSL::V3_30, fragment_shader)
                        .get(glsl)
                        .unwrap()
                        .as_bytes(),
                )
                .unwrap();

            *lock = Some(Arc::new(
                factory
                    .create_pipeline_state(
                        &shader_set,
                        prim_type,
                        rasterizer,
                        note_pipe::new(),
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

fn generate_note_texture(
    factory: &mut Factory
) -> ShaderResourceView<Resources, [f32; 4]> {
    use gfx::texture::AaMode;

    let image_bytes = include_bytes!("../build_assets/note.png");
    let image = image::load_from_memory(image_bytes).unwrap();

    let width = image.width() as u16;
    let height = image.height() as u16;

    let data = image
        .to_rgba()
        .into_raw()
        .chunks(4)
        .map(|ch_iter| {
            let mut vec = [0; 4];
            vec.iter_mut().zip(ch_iter).for_each(|(mut to, from)| {
                *to = *from;
            });

            vec
        })
        .collect::<Vec<_>>();

    factory
        .create_texture_immutable::<gfx::format::Srgba8>(
            gfx::texture::Kind::D2(width, height, AaMode::Single),
            gfx::texture::Mipmap::Provided,
            &[&*data],
        )
        .unwrap()
        .1
}

pub struct Notes {
    note_buffer: Buffer<Resources, NoteLocation>,

    // the note texture should not be here since the game will play multiple
    // songs in a row, we don't want to be loading the same image every time
    // we play a new song
    note_texture: ShaderResourceView<Resources, [f32; 4]>,
}

impl Notes {
    pub fn new(
        factory: &mut Factory,
        notes: [Vec<f32>; 4],
    ) -> Notes
    {
        // collect the notes' positions
        let mut reordered_notes = notes
            .into_iter()
            .enumerate()
            .flat_map(|(pos, vec)| {
                let x_pos =
                    crate::utils::linear_map(pos as f64, 0., 4., -1.5, 1.5)
                        as f32;
                repeat(x_pos).zip(vec.into_iter())
            })
            .collect::<Vec<_>>();

        // sort the notes' ordering
        reordered_notes.sort_unstable_by(|(ax, ay), (bx, by)| {
            ay.partial_cmp(by)
                .unwrap()
                .then_with(|| ax.partial_cmp(bx).unwrap())
        });

        let vertices = reordered_notes
            .into_iter()
            .enumerate()
            .map(|(idx, (x_pos, y_pos))| {
                NoteLocation {
                    vertex_pos: [x_pos, *y_pos],
                    index:      idx as u32,
                }
            })
            .collect::<Vec<_>>();

        let note_buffer = factory.create_vertex_buffer(&vertices);

        Notes {
            note_buffer,
            note_texture: generate_note_texture(factory),
        }
    }

    pub fn get_slice(
        &self,
        factory: &mut Factory,
        hi_speed: f32,
        song_offset: f32,
    ) -> Slice<Resources>
    {
        use gfx::IntoIndexBuffer as _;

        // TODO: it's not really 0 1 2 3 4. it's there for testing purposes
        let idxbuf = [0u16, 1, 2, 3, 4];
        let len = idxbuf.len();

        Slice {
            start:       0u32,
            end:         idxbuf.len() as u32,
            base_vertex: 0,
            instances:   None,
            buffer:      idxbuf.into_index_buffer(factory),
        }
    }

    pub fn render_to(
        &self,
        window: &mut PistonWindow,
        factory: &mut Factory,
        glsl: GLSL,

        hi_speed: f32,
        offset: f32,
        transform: [[f32; 4]; 4],
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

        let slice = self.get_slice(factory, hi_speed, offset);

        // declare the sampler info
        // usually, this would be passed into here
        let sampler_info =
            SamplerInfo::new(FilterMethod::Anisotropic(4), WrapMode::Clamp);

        // declare the data for the pipeline
        let data = note_pipe::Data {
            note_buffer: self.note_buffer.clone(),
            transform,
            out_color:   window.output_color.clone(),
            //note_index_offset: 0,
            //note_index_renderable: &[true, true, true, true],
            //note_index_len: 4,
            hi_speed: 200.,
            song_offset: 0.,
            note_graphic_height: 0.005,
            texture_buffer: (
                self.note_texture.clone(),
                factory.create_sampler(sampler_info),
            ),
        };

        window.encoder.draw(&slice, &*get_pipeline(factory, glsl), &data);
    }
}
