pub enum LaserSegment {
    // the f32 is the bt-position of the lanes
    Slam(f32),
    Curve(f32, LaserKFCurve),
}

pub struct HostLaserSegment {
    slice_indices: Vec<i32>,
    starting_pos: f32,
    ending_pos: f32,
}

pub struct LaserPath {
    start_time: f32,
    start_pos: f32,
    segments: Vec<LaserSegment>,
}

pub struct HostLaserPath {
    segments: Vec<HostLaserSegment>,
    is_focused: bool,
}

impl LaserPath {
    fn new(
        start: f32
        segments: Vec<(LaserSegment, f32)>,
    ) -> LaserPath {
        // iterate through each of the segments

        for (s_type, time) in segments.into_iter() {
            
        }
    }
}
