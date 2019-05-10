pub struct Keyframe<C>
where C: KeyframeCurveType {
    value: f32,
    song_time: SongTime,
    curve: C,
    tension: f32,
}

impl<C> Keyframe
where C: KeyframeCurveType {
    pub fn interpolate_against(
        &self,
        time: &SongTime,
        next: &Keyframe<T, C>
    ) -> T {
        self.curve.interpolate(
            time,
            (self.value, &self.song_time),
            (next.value, &next.song_time),
            self.tension,
        )
    }
}

impl PartialOrd for Keyframe<C>
where C: KeyframeCurveType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.song_time.partial_cmp(&other.song_time)
    }
}

impl Ord for Keyframe<C>
where C: KeyframeCurveType {
    fn cmp(&self, other: &Self) -> Option<Ordering> {
        self.song_time.cmp(&other.song_time)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait KeyframeCurveType {
    fn interpolate(
        &self,
        time: &SongTime,
        this: (f32, &SongTime),
        next: (f32, &SongTime),
        tension: f32
    ) -> T;
}

////////////////////////////////////////////////////////////////////////////////

pub enum TransformationKFCurve {
    HalfSigmoid,
    Sigmoid,
    Stair,
}

impl KeyframeCurveType for TransformationKFCurve {
    pub fn interpolate(
        &self,
        time: &SongTime,
        this: (f32, &SongTime),
        next: (f32, &SongTime),
        tension: f32
    ) -> T {
        use TransformationKFCurve::*;
        use crate::utils::sigmoid;

        match self {
            HalfSigmoid => {
                if tension.is_zero() {
                    // if the tension is zero, the interpolation becomes linear
                    linear_map(
                        (time.0 - this.0) as f32 / (next.0 - this.0) as f32,
                        0.,
                        1.,
                        this.0,
                        next.0,
                    )
                }

                else if tension.is_sign_positive() {
                    let right_end_x = tension.abs().ln() - 1.;
                    let x_val = linear_map(
                        (time.0 - this.0) as f32 / (next.0 - this.0) as f32,
                        0.,
                        1.,
                        0.,
                        right_end_x,
                    );

                    let y_val = sigmoid(x_val);
                    let right_end_y = sigmoid(right_end_x);

                    y_val / right_end_y
                }

                else {
                    let left_end_x = 1. - tension.abs().ln();
                    let x_val = linear_map(
                        (time.0 - this.0) as f32 / (next.0 - this.0) as f32,
                        0.,
                        1.,
                        left_end_x,
                        0.,
                    );

                    let y_val = sigmoid(x_val);
                    let left_end_y = sigmoid(left_end_y);

                    1. - (y_val / left_end_y)
                }
            },

            Sigmoid => {
                if tension.is_zero() {
                    // if the tension is zero, the interpolation becomes linear
                    return (time.0 - this.0) as f32 / (next.0 - this.0) as f32;
                }

                unimplemented!()
            },

            Stair => {
                unimplemented!()
            },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum LaserKFCurve {
    HalfSigmoid,
    Sigmoid,
}
