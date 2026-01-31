use std::{
    fmt::Display,
    num::{ParseIntError, TryFromIntError},
    str::FromStr,
};
use thiserror::Error;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct XY {
    pub x: i32,
    pub y: i32,
}

impl XY {
    pub fn new(width: impl Into<i32>, height: impl Into<i32>) -> Self {
        Self {
            x: width.into(),
            y: height.into(),
        }
    }
}

impl Display for XY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.x, self.y)
    }
}

impl std::ops::Div<i32> for XY {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl std::ops::Add<Self> for XY {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub<Self> for XY {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseResolutionError {
    #[error("parse int error")]
    ParseIntError(#[from] ParseIntError),

    #[error("try from int error")]
    TryFromIntError(#[from] TryFromIntError),

    #[error("invalid format")]
    InvalidFormat,
}

impl FromStr for XY {
    type Err = ParseResolutionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split('x');

        let Some(width) = it.next() else {
            return Err(ParseResolutionError::InvalidFormat);
        };
        let Some(height) = it.next() else {
            return Err(ParseResolutionError::InvalidFormat);
        };
        let None = it.next() else {
            return Err(ParseResolutionError::InvalidFormat);
        };

        let width: i32 = u32::from_str_radix(width, 10)?.try_into()?;
        let height: i32 = u32::from_str_radix(height, 10)?.try_into()?;

        Ok(XY::new(width, height))
    }
}
