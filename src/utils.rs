use num_traits::{
    Float,
    One,
};
use std::ops::{
    Add,
    Div,
    Mul,
    Neg,
    Sub,
};

////////////////////////////////////////////////////////////////////////////////

pub fn linear_map<T>(
    x_i: T,
    x_min: T,
    x_max: T,
    y_min: T,
    y_max: T,
) -> T
where
    T: Copy
        + Add<T, Output = T>
        + Sub<T, Output = T>
        + Div<T, Output = T>
        + Mul<T, Output = T>,
{
    let output = (x_i - x_min) / (x_max - x_min) * (y_max - y_min) + y_min;
    output
}

pub fn sigmoid<T>(x: T) -> T
where T: Float + Neg + Add<T, Output = T> + One {
    (T::one() + (-x).exp()).recip()
}

pub fn block_fn<F, T>(f: F) -> T
where F: FnOnce() -> T {
    use tokio_threadpool::blocking;

    // check if it is already available
    match f.poll() {
        Ok(futures::Async::Ready(smthng)) => return smthng,
        Err(_) => unreachable!(),
        _ => {},
    }

    // perform blocking
    let blocker = blocking(f).expect(
        "block_fn() must be called if the calling thread is on a ThreadPool.",
    );

    // extract data
    match blocker {
        futures::Async::Ready(smthng) => smthng,
        _ => panic!(
            "Maximum number of blocking threads reached!. You may want to \
             consider increasing this."
        ),
    }
}
