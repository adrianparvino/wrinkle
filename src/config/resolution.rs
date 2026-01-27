use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub height: i32,
    pub width: i32,
}

impl Resolution {
    pub fn new(width: impl Into<i32>, height: impl Into<i32>) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
        }
    }
}
