use core::iter::repeat;
use gfx::{
    self,
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
use piston_window::PistonWindow;
use shader_version::{
    glsl::GLSL,
    Shaders,
};
use std::sync::Arc;

////////////////////////////////////////////////////////////////////////////////

lazy_static! {
    static ref PIPELINE: RwLock<Option<Arc<PipelineState<Resources, note_pipe::Meta>>>> =
        RwLock::new(None);
}

fn get_pipeline(
    factory: &mut Factory,
    glsl: GLSL,
) -> Arc<PipelineState<Resources, note_pipe::Meta>>
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
                                include_str!("shaders/bt_chip_notes.vert.glsl"),
                            )
                            .get(glsl)
                            .unwrap()
                            .as_bytes(),
                        Shaders::new()
                            .set(
                                GLSL::V3_30,
                                include_str!("shaders/bt_chip_notes.frag.glsl"),
                            )
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
            vec.iter_mut().zip(ch_iter).for_each(|(to, from)| {
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
    // 2019-04-18_19:00 on second thought, maybe we should put this here. we
    // just have to ask the caller of new() to put a copy of the handle to the
    // resource over here
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
                        index: idx as i32,
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
        let indices = (0 .. 5)
            .flat_map(|note_idx| {
                [0u32, 1, 3, 3, 2, 0]
                    .into_iter()
                    .map(move |v_idx| v_idx + note_idx * 4)
            })
            .collect::<Vec<_>>();
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
            out_color: window.output_color.clone(),
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
