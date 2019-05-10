pub struct Bt {
    chip: BtChips,
    long: BtLongs,
}

pub struct BtChips {
    note_buffer: Buffer<Resources, DeviceBtChip>,
    notes: [Vec<HostBtChip>; 4],
}

pub struct BtLongs {
    begin_note_buffer: Buffer<Resources, DeviceNoteLongBegin>,
    end_note_buffer: Buffer<Resources, DeviceNoteLongEnd>,
    body_note_buffer: Buffer<Resources, DeviceNoteLongBody>,
    notes: [Vec<HostBtLong>; 4],
}

struct HostBtChip {
    slice_index: i32,
    position: f32,
    is_hit: bool,
}

struct HostBtLong {
    slice_index: i32,
    position: f32,
    hit_type: LongHitType,
}

pub enum LongHitType {
    Incoming,
    Active,
    Miss,
    Cleared,
}

impl Bt {
    pub fn new(
    ) -> Bt {
        unimplemented!()
    }
}

impl BtChips {
    pub fn new(
        notes: [Vec<f32>; 4],
        factory: Arc<Mutex<Factory>>,
    ) -> BtChip {
        // collect the notes' positions
        let mut reordered_notes = notes
            .into_iter()
            .enumerate()
            .flat_map(|(pos, vec)| {
                vec.into_iter().zip(move repeat(pos as u8))
            })
            .collect::<Vec<_>>();

        // sort the notes' ordering
        reordered_notes.sort_unstable_by(|(ax, ay), (bx, by)|
            ay.partial_cmp(by)
                .unwrap()
                .then_with(|| ax.cmp(bx))
        );

        let mut host_verts = [vec![], vec![], vec![], vec![]);
        let mut device_verts = vec![];

        // insert the vertices in order
        for (idx, (x_pos, y_pos) in reordered_notes.into_iter().enumerate() {
            hist_verts[x_pos].push(
                HostBtChip {
                    slice_idx: idx,
                    position: y_pos,
                    is_hit: false,
                }
            );

            for corner in (0 .. 4) {
                device_verts.push(
                    DeviceBtChip {
                        vertex_pos: [x_pos, *y_pos],
                        corner_type: corner,
                    }
                );
            }
        }

        let note_buffer = factory.create_vertex_buffer(&device_verts);

        BtChips {
            note_buffer,
            notes: host_verts,
        }
    }
}

impl BtLongs {
    pub fn new(
        // the first f32 is the start position and
        // the second f32 is the end position
        notes: [Vec<(f32, f32)>; 4],
        factory: Arc<Mutex<Factory>>,
    ) -> BtLongs {
        let mut host_verts = ([vec![], vec![], vec![], vec![]);
        let mut device_end_verts = vec![];
        let mut device_start_verts = vec![];
        let mut device_body_verts = vec![];

        // collect the notes' positions
        let mut reordered_notes = notes
            .into_iter()
            .enumerate()
            .flat_map(|(pos, vec)| {
                vec.into_iter().zip(move repeat(pos as u8))
            })
            .collect::<Vec<_>>();

        // sort the notes' ordering
        reordered_notes.sort_unstable_by(|(ax, ay), (bx, by)|
            // sort only using the start of the hold
            ay.0.partial_cmp(by.0)
                .unwrap()
                .then_with(|| ax.cmp(bx))
        );

        // insert the vertices in order
        for (idx, (x_pos, y_pos) in reordered_notes.into_iter().enumerate() {
            host_verts[x_pos].push(
                HostBtLong {
                    slice_idx: idx,
                    position: y_pos,
                    state: LongHitType,
                }
            );

            for corner in (0 .. 4) {
                device_start_verts.push(
                    DeviceNoteLongBegin {
                        vertex_pos: [x_pos, y_pos.0],
                        corner_type: corner,
                    }
                );

                device_end_verts.push(
                    DeviceNoteLongEnd {
                        vertex_pos: [x_pos, y_pos.1],
                        corner_type: corner,
                    }
                );

                device_body_verts.push(
                    DeviceNoteLongHold {
                        start_vertex_pos: [x_pos, y_pos.0],
                        end_vertex_pos: [x_pos, y_pos.1],
                        corner_type: corner,
                    }
                );
            }
        }

        let begin_note_buffer = factory.create_vertex_buffer(&device_start_verts);
        let end_note_buffer = factory.create_vertex_buffer(&device_end_verts);
        let body_note_buffer = factory.create_vertex_buffer(&device_hold_verts);

        BtLongs {
            notes: host_verts,
            begin_note_buffer,
            end_note_buffer,
            body_note_buffer,
        }
    }
}

impl HostBtChip {
    pub fn get_device_indices(&self) -> [i32; 4] {
        let start = self.slice_index * 4;

        [
            start + 0,
            start + 1,
            start + 2,
            start + 3,
        ]
    }
}

impl HostBtLong {
}
