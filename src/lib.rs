//! Concurrent succinct spatial index for SLAM map points.
//!
//! Provides thread-safe insertion and nearest-neighbor queries with
//! memory-efficient storage (f32 coordinates, compact descriptors).

pub mod index;
pub mod octree;
pub mod types;

pub use index::ConcurrentMapIndex;
pub use octree::ConcurrentOctree;
pub use types::{MapPoint, PointId};

/// Generated gRPC types (built from proto/map.proto).
pub mod proto {
    tonic::include_proto!("map");
}
