//! Mock frontend simulator: generates 3D coordinates and descriptor vectors at 60 Hz.
//! Can run standalone (prints to stdout) or push to a gRPC map server.

use std::time::{Duration, Instant};
use rand::Rng;
use cssism::MapPoint;

const HZ: u64 = 60;
const DESCRIPTOR_LEN: usize = 32;
const POINTS_PER_FRAME: usize = 100;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let frame_duration = Duration::from_millis(1000 / HZ);
    let mut next = Instant::now();
    let mut id: u64 = 0;

    eprintln!("Simulator running at {} Hz, {} points/frame", HZ, POINTS_PER_FRAME);
    eprintln!("Output: one batch of MapPoint (id, x, y, z, descriptor_len) per line");

    loop {
        let batch: Vec<MapPoint> = (0..POINTS_PER_FRAME)
            .map(|_| {
                let x = rng.gen_range(-10.0..10.0);
                let y = rng.gen_range(-10.0..10.0);
                let z = rng.gen_range(-5.0..5.0);
                let descriptor: Vec<u8> = (0..DESCRIPTOR_LEN).map(|_| rng.gen()).collect();
                id += 1;
                MapPoint::new(x, y, z, descriptor, id)
            })
            .collect();

        let n = batch.len();
        let first = batch.first().unwrap();
        println!(
            "batch id_start={} count={} sample=({}, {}, {}) descriptor_len={}",
            first.id,
            n,
            first.x,
            first.y,
            first.z,
            first.descriptor.len()
        );

        next += frame_duration;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
    }
}
