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

    layout (location = 0) in vec2 note_pos;
    layout (location = 1) in int note_index;
    layout (location = 2) in int corner_type;

    uniform float song_offset;
    uniform float hi_speed;
    uniform float note_graphic_height;
    uniform mat4 transform;

    out vec2 texture_coord;

    void main() {
        // determine the vertex' real center
        vec2 cur_pos = note_pos;
        cur_pos[1] = (cur_pos[1] - song_offset) * hi_speed;

        vec2 new_note_pos;
        switch (corner_type) {
            case 0: // upper left
                new_note_pos = cur_pos + vec2(-0.5, note_graphic_height);
                texture_coord = vec2(0., 1.);
                break;

            case 1: // upper right
                new_note_pos = cur_pos + vec2(0.5, note_graphic_height);
                texture_coord = vec2(1., 1.);
                break;

            case 2: // lower left
                new_note_pos = cur_pos + vec2(-0.5, 0.);
                texture_coord = vec2(0., 0.);
                break;

            case 3: // lower right
                new_note_pos = cur_pos + vec2(0.5, 0.);
                texture_coord = vec2(1., 0.);
                break;
        }

        gl_Position = transform * vec4(new_note_pos, 0., 1.);
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
    hi_speed: gfx::Global<f32> = "hi_speed",
    song_offset: gfx::Global<f32> = "song_offset",
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "color",
    texture_buffer: gfx::TextureSampler<[f32; 4]> = "raster_texture",

    note_graphic_height: gfx::Global<f32> = "note_graphic_height",
});

gfx_vertex_struct!(NoteLocation {
    vertex_pos: [f32; 2] = "note_pos",
    index:      i32 = "note_index",
    corner_type: i32 = "corner_type",
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
            *lock = Some(Arc::new(
                factory
                    .create_pipeline_simple(
                        Shaders::new()
                            .set(GLSL::V3_30, vertex_shader)
                            .get(glsl)
                            .unwrap()
                            .as_bytes(),
                        Shaders::new()
                            .set(GLSL::V3_30, fragment_shader)
                            .get(glsl)
                            .unwrap()
                            .as_bytes(),
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
                    crate::utils::linear_map(pos as f64, 0., 3., -1.5, 1.5)
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
            .flat_map(|(idx, (x_pos, y_pos))| {
                (0 .. 4).map(move |corner_type| {
                    NoteLocation {
                        vertex_pos: [x_pos, *y_pos],
                        index:      idx as i32,
                        corner_type,
                    }
                })
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

        // TODO: it's not really (0..4). it's there for testing purposes
        let indices = (0 .. 5).flat_map(|note_idx| {
            [0u32, 1, 3, 3, 2, 0].into_iter()
                .map(move |v_idx| v_idx + note_idx * 4)
        }).collect::<Vec<_>>();
        let len = indices.len() as u32;

        Slice {
            start:       0u32,
            end:         len,
            base_vertex: 0,
            instances:   None,
            buffer:      (&*indices).into_index_buffer(factory),
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
            hi_speed: 1.25,
            song_offset: 0.75,
            note_graphic_height: 0.03,
            texture_buffer: (
                self.note_texture.clone(),
                factory.create_sampler(sampler_info),
            ),
        };

        window.encoder.draw(&slice, &*get_pipeline(factory, glsl), &data);
    }
}
