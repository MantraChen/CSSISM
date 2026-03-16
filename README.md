# CSSISM: Concurrent Succinct Spatial Index for SLAM Mapping

Backend data structuring for a visual-inertial SLAM system, focused on **memory efficiency** and **high-throughput point cloud querying**.

## Scenario

A SLAM backend receives high-frequency 3D map points. Points are stored to support:

- **Rapid, thread-safe nearest-neighbor queries** (e.g., for loop closure).
- **Minimal memory overhead** (succinct representation where applicable).

## What This Repo Provides

1. **Mock frontend simulator** – Generates 3D coordinates `(x, y, z)` and descriptor vectors at **60 Hz**.
2. **Benchmarking suite** – Measures read/write throughput and memory usage of different spatial indexes.
3. **gRPC server** – Basic service for inserting points and querying the map (e.g., nearest neighbors).
4. **Concurrent k-d tree index** – `ConcurrentMapIndex` built on top of `kiddo` for high-throughput kNN queries.
5. **Concurrent succinct octree** – `ConcurrentOctree` with:
   - **Succinct topology** encoded as a bit-vector (`SuccinctOctreeLayout`) separating structure from payload.
   - **Immutable snapshots** of point storage for lock-free read scans while writes continue in the background.

## Building and Running

Requires a Rust toolchain (e.g. [rustup](https://rustup.rs/)).

```bash
cargo build --release
cargo run --bin server     # Start gRPC map server (default: [::1]:50051)
cargo run --bin simulator  # Run 60 Hz mock data generator (prints batch summaries)
cargo bench                # Run throughput and memory benchmarks
```

## License

MIT License.
