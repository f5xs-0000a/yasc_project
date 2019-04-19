
////////////////////////////////////////////////////////////////////////////////

// TODO: maybe this should be an actor too since it handles renders and inputs?
pub struct LaneGovernor {
    // keyframes
    rotation_events: Vec<(SongTime, Keyframe<Rad<f32>>)>,
    slant_events: Vec<(SongTime, Keyframe<Rad<f32>>)>,
    zoom_events: Vec<(SongTime, Keyframe<f32>)>,

    
}

pub struct Keyframe<T: Clone, C: KeyframeCurveType<T>> {
    value: T,
    curve: C,
    tension: f32,
}

pub trait KeyframeCurveType<T> {
    fn interpolate(
        time: &SongTime,
        kf1: &(SongTime, Keyframe<T, Self>),
        kf2: &(SongTime, Keyframe<T, Self>),
    ) -> T;
}

pub enum TransformationKFCurve {
    Linear,
    Stair,
}

pub enum LaserKFCurve {
    Linear,
}

impl LaneGovernor {

            rotation: Rad(0.),
            slant: Rad::from(Deg(36.5)),
            zoom: -0.9765625,
    pub fn get_current_rotation(&self, time: &SongTime) -> Rad(f32) {
        if self.rotation_events.is_empty() {
            return Rad(0.);
        }

        let search = self
            .rotation_events
            .binary_search_by_key(time, |(t, _)| t);

        match search {
            Ok(idx) => self.rotation_events[idx].1.value.clone(),
            Err(idx) => {
                if idx == self.rotation_events.len() {
                    self.rotation_events[idx - 1].1.value.clone()
                }

                else {
                    TransformationKFCurve::interpolate(
                        time,
                        &self.rotation_events[idx - 1],
                        &self.rotation_events[idx],
                    )
                }
            }
        }
    }

    pub fn get_current_slant(&self, time: &SongTime) -> Rad(f32) {
        if self.slant_events.is_empty() {
            // this assumes an FOV of 90
            return Rad::from(Deg(36.5));
            // if FOV = 60, use Deg(52)
        }

        let search = self
            .slant_events
            .binary_search_by_key(time, |(t, _)| t);

        match search {
            Ok(idx) => self.slant_events[idx].1.value.clone(),
            Err(idx) => {
                if idx == self.slant_events.len() {
                    self.slant_events[idx - 1].1.value.clone()
                }

                else {
                    TransformationKFCurve::interpolate(
                        time,
                        &self.slant_events[idx - 1],
                        &self.slant_events[idx],
                    )
                }
            }
        }
    }

    pub fn get_current_zoom(&self, time: &SongTime) -> f32 {
        if self.zoom_events.is_empty() {
            // this assumes an FOV of 90
            return -0.9765625;
            // if FOV = 60, use -0.4375
        }

        let search = self
            .zoom_events
            .binary_search_by_key(time, |(t, _)| t);

        match search {
            Ok(idx) => self.zoom_events[idx].1.value.clone(),
            Err(idx) => {
                if idx == self.zoom_events.len() {
                    self.zoom_events[idx - 1].1.value.clone()
                }

                else {
                    TransformationKFCurve::interpolate(
                        time,
                        &self.zoom_events[idx - 1],
                        &self.zoom_events[idx],
                    )
                }
            }
        }
    }

    pub fn calculate_matrix(&self, time: &SongTime) -> Matrix4<f32> {
        const BACK_OFFSET: f32 = -3.6;
        const VERT_SCALE: f32 = 10.25;

        let rotation = self.get_current_rotation(time);
        let slant = self.get_current_slant(time);
        let zoom = self.get_current_zoom(time);

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

        let post_mvp = mvp(&model, &view, &projection);

        // rotate the lanes from a center point in the camera
        Matrix4::from(
            Quaternion::from_axis_angle(
                Vector3::new(0., 0., 1.),
                self.rotation,
            )
        ) *

        // move the lanes' view downwards
        Matrix4::from_translation(
            Vector3::new(0., -0.975, 0.)
        ) *

        mvp
    }
}

fn mvp(
    m: &Matrix4<f32>,
    v: &Matrix4<f32>,
    p: &Matrix4<f32>,
) -> Matrix4<f32>
{
    p * (v * m)
}
