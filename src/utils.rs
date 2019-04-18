pub fn linear_map(
    x_i: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> f64
{
    let output = (x_i - x_min) / (x_max - x_min) * (y_max - y_min) + y_min;
    output
}
