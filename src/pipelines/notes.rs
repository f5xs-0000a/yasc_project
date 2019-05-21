use gfx;

///////////////////////////////////////////////////////////////////////////////

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
    note_pos:    [f32; 2] = "note_pos",
    corner_type: i32 = "corner_type",
});

#[derive(Debug, Clone, PartialEq)]
pub struct HostNote {
    pub position: f32,
    pub index:    usize,
    pub is_hit:   bool,
}

////////////////////////////////////////////////////////////////////////////////

impl NoteLocation {
    pub fn new(
        note_kind: u8,
        position: f32,
    ) -> [NoteLocation; 4]
    {
        assert!(
            note_kind > 4,
            "`note_kind` should have a value from 0 to 4, inclusive!"
        );

        let x_pos =
            crate::utils::linear_map(note_kind as f64, 0., 3., -1.5, 1.5);

        // define the center of the note
        let note_pos = [x_pos as f32, position];

        // create four "vertices", one for each of the corner of the note
        // rectangle
        [
            NoteLocation {
                note_pos:    note_pos.clone(),
                corner_type: 0,
            },
            NoteLocation {
                note_pos:    note_pos.clone(),
                corner_type: 1,
            },
            NoteLocation {
                note_pos:    note_pos.clone(),
                corner_type: 2,
            },
            NoteLocation {
                note_pos,
                corner_type: 3,
            },
        ]
    }

    /// Returns a vector of NoteLocation and a remapped [Vec; 4] of the inputs
    ///
    /// The first part of the return is the vector that will be uploaded into
    /// the vertex buffer.
    /// The second part is an ordered vector of the buttons. Their visibility
    /// can be edited
    pub fn from_vec4(
        positions: [Vec<f32>; 4]
    ) -> (Vec<NoteLocation>, [Vec<HostNote>; 4]) {
        let mut indexable = [vec![], vec![], vec![], vec![]];

        // collect the notes' positions
        let mut flattened_positions = positions
            .into_iter()
            .enumerate()
            .flat_map(|(bt, vec)| core::iter::repeat(bt).zip(vec.into_iter()))
            .collect::<Vec<_>>();

        // sort the notes' ordering
        flattened_positions.sort_unstable_by(|(ax, ay), (bx, by)| {
            ay.partial_cmp(by)
                .unwrap()
                .then_with(|| ax.partial_cmp(bx).unwrap())
        });

        let buffer = flattened_positions
            .into_iter()
            .enumerate()
            .flat_map(|(idx, (bt, tpos))| {
                // generate the indexable entries
                let hostnote = HostNote {
                    position: tpos.clone(),
                    index:    idx,
                    is_hit:   false,
                };
                indexable[bt].push(hostnote);

                // generate the buffer entries
                let mut retval = Vec::new();
                retval.extend_from_slice(&NoteLocation::new(
                    bt as u8,
                    tpos.clone(),
                ));
                retval
            })
            .collect::<Vec<_>>();

        (buffer, indexable)
    }
}
