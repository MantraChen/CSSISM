//! gRPC server for map insert and nearest-neighbor queries.

use std::sync::Arc;
use tonic::{Request, Response, Status};
use cssism::{ConcurrentMapIndex, MapPoint};

/// Generated gRPC types for the map service (from `proto/map.proto`).
pub mod rpc {
    tonic::include_proto!("map");
}

use rpc::map_service_server::{MapService, MapServiceServer};
use rpc::{
    InsertPointsRequest, InsertPointsResponse, MapPoint as ProtoMapPoint,
    NearestQueryRequest, NearestQueryResponse,
};

#[derive(Default)]
pub struct MapServiceImpl {
    index: Arc<ConcurrentMapIndex>,
}

impl MapServiceImpl {
    pub fn new(index: Arc<ConcurrentMapIndex>) -> Self {
        Self { index }
    }
}

#[tonic::async_trait]
impl MapService for MapServiceImpl {
    async fn insert_points(
        &self,
        request: Request<InsertPointsRequest>,
    ) -> Result<Response<InsertPointsResponse>, Status> {
        let msg = request.into_inner();
        let batch: Vec<MapPoint> = msg
            .points
            .into_iter()
            .map(|p| MapPoint::new(p.x, p.y, p.z, p.descriptor, p.id))
            .collect();
        let (inserted, _) = self.index.insert_batch(batch);
        Ok(Response::new(InsertPointsResponse {
            inserted: inserted as u32,
        }))
    }

    async fn nearest_neighbors(
        &self,
        request: Request<NearestQueryRequest>,
    ) -> Result<Response<NearestQueryResponse>, Status> {
        let q = request.into_inner();
        let k = q.k as usize;
        let neighbors = self.index.nearest(q.x, q.y, q.z, k.max(1));
        let points: Vec<ProtoMapPoint> = neighbors
            .into_iter()
            .map(|p| ProtoMapPoint {
                x: p.x,
                y: p.y,
                z: p.z,
                descriptor: p.descriptor,
                id: p.id,
            })
            .collect();
        Ok(Response::new(NearestQueryResponse { neighbors: points }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: std::net::SocketAddr = "[::1]:50051".parse()?;
    let index = Arc::new(ConcurrentMapIndex::new());
    let service = MapServiceImpl::new(index);
    let svc = MapServiceServer::new(service);
    tonic::transport::Server::builder()
        .add_service(svc)
        .serve(addr)
        .await?;
    Ok(())
}
