//! Core types for map points and descriptors.

use serde::{Deserialize, Serialize};

/// Unique id for a map point (used as item in k-d tree).
pub type PointId = u64;

/// A 3D map point with optional descriptor for loop closure.
/// Stored with f32 for succinct memory layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Compact descriptor; typical length 32--256 bytes.
    pub descriptor: Vec<u8>,
    pub id: PointId,
}

impl MapPoint {
    pub fn new(x: f32, y: f32, z: f32, descriptor: Vec<u8>, id: PointId) -> Self {
        Self { x, y, z, descriptor, id }
    }

    /// Position as 3-element array for k-d tree.
    #[inline]
    pub fn position(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}
