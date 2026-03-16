//! Concurrent spatial index: k-d tree + point store with RwLock.

use kiddo::{KdTree, NearestNeighbour, SquaredEuclidean};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{MapPoint, PointId};

/// Thread-safe spatial index: k-d tree (position -> index) + vector of points.
/// Uses f32 for succinct storage; descriptors in MapPoint for compact layout.
pub struct ConcurrentMapIndex {
    next_id: AtomicU64,
    /// k-d tree: [x,y,z] -> index into points vec (stored as u64 for kiddo default type).
    tree: RwLock<KdTree<f32, 3>>,
    points: RwLock<Vec<MapPoint>>,
}

impl Default for ConcurrentMapIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrentMapIndex {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            tree: RwLock::new(KdTree::new()),
            points: RwLock::new(Vec::new()),
        }
    }

    /// Insert a single point; returns assigned id.
    pub fn insert(&self, mut point: MapPoint) -> PointId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        point.id = id;
        let pos = point.position();
        let mut points = self.points.write();
        let mut tree = self.tree.write();
        let idx = points.len();
        points.push(point);
        tree.add(&pos, idx as u64);
        id
    }

    /// Insert multiple points; returns number inserted and first assigned id.
    pub fn insert_batch(&self, mut batch: Vec<MapPoint>) -> (usize, PointId) {
        if batch.is_empty() {
            return (0, 0);
        }
        let n = batch.len();
        let start_id = self.next_id.fetch_add(n as u64, Ordering::SeqCst);
        let mut tree = self.tree.write();
        let mut points = self.points.write();
        for (i, mut p) in batch.drain(..).enumerate() {
            let id = start_id + i as u64;
            p.id = id;
            let pos = p.position();
            let idx = points.len();
            tree.add(&pos, idx as u64);
            points.push(p);
        }
        (n, start_id)
    }

    /// Nearest k neighbors to (x,y,z). Returns points with descriptors.
    pub fn nearest(&self, x: f32, y: f32, z: f32, k: usize) -> Vec<MapPoint> {
        let query = [x, y, z];
        let guard = self.tree.read();
        let nearest: Vec<NearestNeighbour<f32, u64>> = guard.nearest_n::<SquaredEuclidean>(&query, k);
        let indices: Vec<usize> = nearest.into_iter().map(|n| n.item as usize).collect();
        drop(guard);
        let points_guard = self.points.read();
        indices
            .into_iter()
            .filter_map(|i| points_guard.get(i).cloned())
            .collect()
    }

    /// Total number of points.
    pub fn len(&self) -> usize {
        self.points.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
