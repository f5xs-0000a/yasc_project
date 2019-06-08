use core::ops::{
    Add,
    Sub,
};

////////////////////////////////////////////////////////////////////////////////

pub type SongTicker = i64;

// NOTE: I forgot the reasoning behind i and not u
#[derive(Hash, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SongTime(pub i64);

impl Add for SongTime {
    type Output = Self;

    fn add(
        self,
        other: Self,
    ) -> Self::Output
    {
        SongTime(self.0 + other.0)
    }
}

impl Sub for SongTime {
    type Output = Self;

    fn sub(
        self,
        other: Self,
    ) -> Self::Output
    {
        SongTime(self.0 - other.0)
    }
}
