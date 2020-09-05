use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MeanAndStddev<T = f64> {
    pub mean: T,
    pub stddev: T,
}
