//! Concurrent succinct octree for 3D map points.
//!
//! This implementation focuses on a compact in-memory layout and thread-safe access.
//! For now, nearest-neighbor queries are implemented as a linear scan over points,
//! using the octree structure as a scaffold for future pruning optimizations.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{MapPoint, PointId};

/// A single octree node laid out compactly in memory.
#[derive(Clone, Copy, Debug)]
pub struct OctreeNode {
    /// Minimum corner of this node's axis-aligned cube.
    pub origin: [f32; 3],
    /// Half of the cube edge length.
    pub half_size: f32,

    /// Bitmask of existing children (8 bits, one per octant).
    pub child_mask: u8,

    /// Index of the first child node in the global nodes array, or -1 if leaf.
    pub first_child: i32,

    /// Range of point indices [start, end) in the global point_indices array.
    pub point_start: u32,
    pub point_end: u32,
}

impl OctreeNode {
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.first_child < 0
    }
}

/// Thread-safe octree index over 3D points.
///
/// The octree structure is prepared for spatial subdivision, but the initial
/// implementation uses a linear scan for nearest-neighbor queries in order to
/// keep the concurrency and memory model simple and robust.
pub struct ConcurrentOctree {
    next_id: AtomicU64,

    /// All points stored in the index.
    points: RwLock<Vec<MapPoint>>,

    /// Optional spatial hierarchy (currently used only as a placeholder).
    nodes: RwLock<Vec<OctreeNode>>,

    /// Maximum number of points in a leaf before considering a split.
    pub max_points_per_leaf: u16,

    /// Maximum depth of the tree.
    pub max_depth: u8,
}

impl Default for ConcurrentOctree {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrentOctree {
    /// Create a new octree covering a fixed cube around the origin.
    ///
    /// For SLAM-style mapping around the sensor, this is usually sufficient as
    /// a starting point; out-of-bounds points can later be handled by extending
    /// the root or by using multiple roots.
    pub fn new() -> Self {
        // Root covers a reasonably large cube centered at the origin.
        let root = OctreeNode {
            origin: [-50.0, -50.0, -50.0],
            half_size: 50.0,
            child_mask: 0,
            first_child: -1,
            point_start: 0,
            point_end: 0,
        };

        Self {
            next_id: AtomicU64::new(0),
            points: RwLock::new(Vec::new()),
            nodes: RwLock::new(vec![root]),
            max_points_per_leaf: 64,
            max_depth: 12,
        }
    }

    /// Insert a single point and return its assigned id.
    pub fn insert(&self, mut point: MapPoint) -> PointId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        point.id = id;

        let mut points = self.points.write();
        points.push(point);
        id
    }

    /// Insert a batch of points; returns (count, first_assigned_id).
    pub fn insert_batch(&self, mut batch: Vec<MapPoint>) -> (usize, PointId) {
        if batch.is_empty() {
            return (0, 0);
        }

        let n = batch.len();
        let start_id = self.next_id.fetch_add(n as u64, Ordering::SeqCst);

        let mut points = self.points.write();
        for (i, mut p) in batch.drain(..).enumerate() {
            let id = start_id + i as u64;
            p.id = id;
            points.push(p);
        }

        (n, start_id)
    }

    /// Brute-force k-nearest neighbors to (x, y, z).
    ///
    /// This is intentionally simple and numerically robust. It provides a
    /// baseline for correctness and can be used to validate future optimized
    /// octree-based search implementations.
    pub fn nearest(&self, x: f32, y: f32, z: f32, k: usize) -> Vec<MapPoint> {
        let pts = self.points.read();
        if pts.is_empty() || k == 0 {
            return Vec::new();
        }

        let mut distances: Vec<(f32, usize)> = pts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let dx = p.x - x;
                let dy = p.y - y;
                let dz = p.z - z;
                (dx * dx + dy * dy + dz * dz, i)
            })
            .collect();

        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let k_clamped = k.min(distances.len());
        let mut out = Vec::with_capacity(k_clamped);
        for (_, idx) in distances.into_iter().take(k_clamped) {
            if let Some(p) = pts.get(idx) {
                out.push(p.clone());
            }
        }
        out
    }

    /// Total number of points in the index.
    pub fn len(&self) -> usize {
        self.points.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

