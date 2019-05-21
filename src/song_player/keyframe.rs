use crate::{
    song_player::song_timer::SongTime,
    utils::linear_map,
};
use core::cmp::Ordering;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub struct Keyframe<C>
where C: KeyframeCurveType {
    value:     f32,
    song_time: SongTime,
    curve:     C,
    tension:   f32,
}

impl<C> Keyframe<C>
where C: KeyframeCurveType
{
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn song_time(&self) -> SongTime {
        self.song_time.clone()
    }

    pub fn curve(&self) -> &C {
        &self.curve
    }

    pub fn tension(&self) -> f32 {
        self.tension
    }
}

impl<C> Keyframe<C>
where C: KeyframeCurveType
{
    pub fn interpolate_against(
        &self,
        time: &SongTime,
        next: &Keyframe<C>,
    ) -> f32
    {
        self.curve.interpolate(
            time,
            (self.value, &self.song_time),
            (next.value, &next.song_time),
            self.tension,
        )
    }
}

impl<C> PartialOrd for Keyframe<C>
where C: KeyframeCurveType + PartialEq
{
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering>
    {
        self.song_time.partial_cmp(&other.song_time)
    }
}

/*
impl<C> Ord for Keyframe<C>
where C: KeyframeCurveType {
    fn cmp(&self, other: &Self) -> Option<Ordering> {
        self.song_time.cmp(&other.song_time)
    }
}
*/

////////////////////////////////////////////////////////////////////////////////

pub trait KeyframeCurveType {
    fn interpolate(
        &self,
        time: &SongTime,
        this: (f32, &SongTime),
        next: (f32, &SongTime),
        tension: f32,
    ) -> f32;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub enum TransformationKFCurve {
    HalfSigmoid,
    Sigmoid,
    Stair,
}

impl KeyframeCurveType for TransformationKFCurve {
    fn interpolate(
        &self,
        time: &SongTime,
        this: (f32, &SongTime),
        next: (f32, &SongTime),
        tension: f32,
    ) -> f32
    {
        use crate::utils::sigmoid;
        use TransformationKFCurve::*;

        match self {
            HalfSigmoid => {
                if tension == 0. {
                    // if the tension is zero, the interpolation becomes linear
                    linear_map(
                        (time.0 as f32 - this.0) / (next.0 as f32 - this.0),
                        0.,
                        1.,
                        this.0,
                        next.0,
                    )
                }
                else if tension.is_sign_positive() {
                    let right_end_x = tension.abs().ln() - 1.;
                    let x_val = linear_map(
                        (time.0 as f32 - this.0) / (next.0 as f32 - this.0),
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
                        (time.0 as f32 - this.0) / (next.0 as f32 - this.0),
                        0.,
                        1.,
                        left_end_x,
                        0.,
                    );

                    let y_val = sigmoid(x_val);
                    let left_end_y = sigmoid(left_end_x);

                    1. - (y_val / left_end_y)
                }
            },

            Sigmoid => {
                if tension == 0. {
                    // if the tension is zero, the interpolation becomes linear
                    return (time.0 as f32 - this.0) / (next.0 as f32 - this.0);
                }

                unimplemented!()
            },

            Stair => unimplemented!(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum LaserKFCurve {
    HalfSigmoid,
    Sigmoid,
}
