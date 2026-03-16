//! Concurrent succinct octree for 3D map points.
//!
//! The octree topology is stored in a succinct bit-vector form, separating
//! structure from payload. For now, nearest-neighbor queries are implemented
//! as a linear scan over points; the succinct layout provides a compact,
//! cache-friendly representation that can be used for future spatial pruning.

use bitvec::prelude::*;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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

/// Succinct 8-ary tree topology backed by a bit-vector.
///
/// Nodes are stored in breadth-first (level-order) sequence. For each node i:
/// - bits[i] == true  -> internal node with exactly 8 children
/// - bits[i] == false -> leaf node with no children
///
/// Children of internal nodes are stored implicitly rather than via pointers.
/// If `rank1(i)` is the number of internal nodes in [0, i), then the first
/// child of node i (assuming it is internal) is at index:
///     1 + 8 * rank1(i)
///
/// This layout is succinct and highly cache-friendly, and supports O(1)
/// navigation given rank/select operations on the bit-vector.
#[derive(Clone, Debug, Default)]
pub struct SuccinctOctreeLayout {
    bits: BitVec<u64, Lsb0>,
}

impl SuccinctOctreeLayout {
    pub fn new() -> Self {
        Self {
            bits: BitVec::new(),
        }
    }

    /// Build a layout from an iterator of node flags (internal = true).
    pub fn from_internal_flags<I: IntoIterator<Item = bool>>(flags: I) -> Self {
        let mut bits = BitVec::<u64, Lsb0>::new();
        bits.extend(flags);
        Self { bits }
    }

    /// Total number of nodes encoded in the layout.
    pub fn len(&self) -> usize {
        self.bits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    /// Returns true if the node at index i is internal.
    pub fn is_internal(&self, i: usize) -> bool {
        self.bits.get(i).copied().unwrap_or(false)
    }

    /// Number of internal nodes in [0, i).
    #[inline]
    pub fn rank1(&self, i: usize) -> usize {
        if i == 0 {
            0
        } else {
            self.bits[..i].count_ones()
        }
    }

    /// Returns the index of the first child of node i if it is internal.
    /// Children occupy indices [first_child, first_child + 8).
    pub fn first_child_index(&self, i: usize) -> Option<usize> {
        if !self.is_internal(i) {
            return None;
        }
        let r = self.rank1(i);
        Some(1 + 8 * r)
    }
}

/// Thread-safe octree index over 3D points.
///
/// The succinct layout encodes the tree topology, while point storage is kept
/// in a separate contiguous array. The current implementation uses a linear
/// scan for queries, using the octree only as a compact structural scaffold.
pub struct ConcurrentOctree {
    next_id: AtomicU64,

    /// Live points buffer, used for writes.
    points: RwLock<Vec<MapPoint>>,

    /// Immutable snapshot used by readers for lock-free scans.
    points_snapshot: RwLock<Arc<Vec<MapPoint>>>,

    /// Optional spatial hierarchy in explicit node form. This can be used to
    /// derive or validate the succinct layout.
    nodes: RwLock<Vec<OctreeNode>>,

    /// Succinct bit-vector representation of the node topology.
    layout: RwLock<SuccinctOctreeLayout>,

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

        // At creation time, only the root exists and is a leaf.
        let layout = SuccinctOctreeLayout::from_internal_flags([false]);

        Self {
            next_id: AtomicU64::new(0),
            points: RwLock::new(Vec::new()),
            points_snapshot: RwLock::new(Arc::new(Vec::new())),
            nodes: RwLock::new(vec![root]),
            layout: RwLock::new(layout),
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
        // Rebuild snapshot so new readers see an up-to-date immutable view.
        let snapshot = Arc::new(points.clone());
        *self.points_snapshot.write() = snapshot;
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
        // Single snapshot rebuild amortized over the whole batch.
        let snapshot = Arc::new(points.clone());
        *self.points_snapshot.write() = snapshot;

        (n, start_id)
    }

    /// Returns the succinct topology layout (read-only snapshot).
    pub fn layout_snapshot(&self) -> SuccinctOctreeLayout {
        self.layout.read().clone()
    }

    /// Brute-force k-nearest neighbors to (x, y, z).
    ///
    /// This is intentionally simple and numerically robust. It provides a
    /// baseline for correctness and can be used to validate future optimized
    /// octree-based search implementations.
    pub fn nearest(&self, x: f32, y: f32, z: f32, k: usize) -> Vec<MapPoint> {
        // Take a cheap clone of the current immutable snapshot and then drop locks,
        // allowing writers to continue updating the live buffer concurrently.
        let snapshot = self.points_snapshot.read().clone();
        let pts: &Vec<MapPoint> = &snapshot;
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

