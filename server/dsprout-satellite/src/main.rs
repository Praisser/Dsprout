use axum::{
    Json, Router,
    routing::{get, post},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
struct AppState {
    // worker_id -> worker info
    workers: Arc<DashMap<String, WorkerInfo>>,
    // file_id -> list of shard records
    shard_index: Arc<DashMap<String, Vec<ShardRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerInfo {
    worker_id: String,
    multiaddr: String,
    last_seen: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterWorkerReq {
    worker_id: String,
    multiaddr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeartbeatReq {
    worker_id: String,
    multiaddr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShardRecord {
    worker_id: String,
    worker_multiaddr: String,
    file_id: String,
    segment_index: u32,
    shard_index: u8,
    shard_hash_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterShardReq {
    record: ShardRecord,
}

#[derive(Debug, Serialize)]
struct LocateResp {
    file_id: String,
    shards: Vec<ShardRecord>,
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

async fn register_worker(
    state: axum::extract::State<AppState>,
    Json(req): Json<RegisterWorkerReq>,
) -> Json<&'static str> {
    let worker = WorkerInfo {
        worker_id: req.worker_id.clone(),
        multiaddr: req.multiaddr,
        last_seen: now_ms(),
    };
    state.workers.insert(req.worker_id, worker);
    Json("ok")
}

async fn workers(state: axum::extract::State<AppState>) -> Json<Vec<WorkerInfo>> {
    let out: Vec<WorkerInfo> = state.workers.iter().map(|e| e.value().clone()).collect();
    Json(out)
}

async fn heartbeat(
    state: axum::extract::State<AppState>,
    Json(req): Json<HeartbeatReq>,
) -> Json<&'static str> {
    let now = now_ms();
    state
        .workers
        .entry(req.worker_id.clone())
        .and_modify(|w| {
            w.last_seen = now;
            w.multiaddr = req.multiaddr.clone();
        })
        .or_insert(WorkerInfo {
            worker_id: req.worker_id,
            multiaddr: req.multiaddr,
            last_seen: now,
        });
    Json("ok")
}

async fn register_shard(
    state: axum::extract::State<AppState>,
    Json(req): Json<RegisterShardReq>,
) -> Json<&'static str> {
    state
        .shard_index
        .entry(req.record.file_id.clone())
        .and_modify(|v| v.push(req.record.clone()))
        .or_insert(vec![req.record]);
    Json("ok")
}

async fn locate(
    state: axum::extract::State<AppState>,
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<LocateResp> {
    let file_id = q.get("file_id").cloned().unwrap_or_default();
    let shards = state
        .shard_index
        .get(&file_id)
        .map(|r| r.clone())
        .unwrap_or_default();

    Json(LocateResp { file_id, shards })
}

#[tokio::main]
async fn main() {
    let state = AppState {
        workers: Arc::new(DashMap::new()),
        shard_index: Arc::new(DashMap::new()),
    };

    let app = Router::new()
        .route("/register_worker", post(register_worker))
        .route("/workers", get(workers))
        .route("/heartbeat", post(heartbeat))
        .route("/register_shard", post(register_shard))
        .route("/locate", get(locate))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7070").await.unwrap();
    println!("Satellite running on http://localhost:7070");
    axum::serve(listener, app).await.unwrap();
}
