use anyhow::{Result as AnyResult, anyhow};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tokio::{net::TcpListener, process::Child, process::Command, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerConfig {
    profile: String,
    listen_multiaddr: String,
    satellite_url: String,
    device_name: String,
    owner_label: String,
    capacity_limit_bytes: u64,
    enabled: bool,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            profile: "worker".to_string(),
            listen_multiaddr: "/ip4/127.0.0.1/tcp/5901".to_string(),
            satellite_url: "http://127.0.0.1:7070".to_string(),
            device_name: "Local Worker".to_string(),
            owner_label: "Contributor".to_string(),
            capacity_limit_bytes: 1024 * 1024 * 1024,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct WorkerStatusResp {
    running: bool,
    pid: Option<u32>,
    started_at_ms: Option<u128>,
    last_exit_code: Option<i32>,
    last_error: Option<String>,
    config: WorkerConfig,
}

#[derive(Debug, Clone, Serialize)]
struct ActionResp {
    status: String,
    message: String,
    worker: WorkerStatusResp,
}

#[derive(Debug, Clone, Serialize)]
struct StorageResp {
    profile: String,
    used_bytes: u64,
    hosted_shards: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigUpdateReq {
    profile: Option<String>,
    listen_multiaddr: Option<String>,
    satellite_url: Option<String>,
    device_name: Option<String>,
    owner_label: Option<String>,
    capacity_limit_bytes: Option<u64>,
    enabled: Option<bool>,
    restart_if_running: Option<bool>,
}

struct AgentState {
    config: WorkerConfig,
    child: Option<Child>,
    started_at_ms: Option<u128>,
    last_exit_code: Option<i32>,
    last_error: Option<String>,
}

#[derive(Clone)]
struct AppState {
    inner: Arc<Mutex<AgentState>>,
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn to_http_err(err: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn data_dir() -> AnyResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| anyhow!("No data_dir found"))?;
    let dir = base.join("dsprout");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn config_path() -> AnyResult<PathBuf> {
    Ok(data_dir()?.join("agent-config.json"))
}

fn load_config() -> AnyResult<WorkerConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(WorkerConfig::default());
    }
    let bytes = fs::read(path)?;
    let cfg = serde_json::from_slice::<WorkerConfig>(&bytes)?;
    Ok(cfg)
}

fn save_config(cfg: &WorkerConfig) -> AnyResult<()> {
    let path = config_path()?;
    let bytes = serde_json::to_vec_pretty(cfg)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn worker_bin_path() -> AnyResult<PathBuf> {
    if let Some(v) = std::env::var_os("DSPROUT_WORKER_BIN") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Ok(p);
        }
        return Err(anyhow!("DSPROUT_WORKER_BIN is set but path does not exist"));
    }

    let current = std::env::current_exe()?;
    let dir = current
        .parent()
        .ok_or_else(|| anyhow!("cannot resolve current exe parent"))?;
    let candidate = dir.join("dsprout-worker");
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(anyhow!(
        "failed to find dsprout-worker binary next to dsprout-agent; set DSPROUT_WORKER_BIN"
    ))
}

fn refresh_process_state(state: &mut AgentState) {
    if let Some(child) = state.child.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                state.last_exit_code = status.code();
                state.started_at_ms = None;
                state.child = None;
            }
            Ok(None) => {}
            Err(err) => {
                state.last_error = Some(format!("failed to poll worker process: {err}"));
                state.started_at_ms = None;
                state.child = None;
            }
        }
    }
}

fn snapshot_status(state: &AgentState) -> WorkerStatusResp {
    WorkerStatusResp {
        running: state.child.is_some(),
        pid: state.child.as_ref().and_then(Child::id),
        started_at_ms: state.started_at_ms,
        last_exit_code: state.last_exit_code,
        last_error: state.last_error.clone(),
        config: state.config.clone(),
    }
}

fn apply_config_update(
    cfg: &mut WorkerConfig,
    req: &ConfigUpdateReq,
) -> Result<(), (StatusCode, String)> {
    if let Some(v) = &req.profile {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "profile cannot be empty".to_string(),
            ));
        }
        cfg.profile = trimmed.to_string();
    }
    if let Some(v) = &req.listen_multiaddr {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "listen_multiaddr cannot be empty".to_string(),
            ));
        }
        cfg.listen_multiaddr = trimmed.to_string();
    }
    if let Some(v) = &req.satellite_url {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "satellite_url cannot be empty".to_string(),
            ));
        }
        cfg.satellite_url = trimmed.to_string();
    }
    if let Some(v) = &req.device_name {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "device_name cannot be empty".to_string(),
            ));
        }
        cfg.device_name = trimmed.to_string();
    }
    if let Some(v) = &req.owner_label {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "owner_label cannot be empty".to_string(),
            ));
        }
        cfg.owner_label = trimmed.to_string();
    }
    if let Some(v) = req.capacity_limit_bytes {
        cfg.capacity_limit_bytes = v;
    }
    if let Some(v) = req.enabled {
        cfg.enabled = v;
    }
    Ok(())
}

fn spawn_worker(cfg: &WorkerConfig) -> AnyResult<Child> {
    let bin = worker_bin_path()?;
    let mut cmd = Command::new(bin);
    cmd.arg("--profile")
        .arg(&cfg.profile)
        .arg("--listen")
        .arg(&cfg.listen_multiaddr)
        .arg("--satellite-url")
        .arg(&cfg.satellite_url)
        .arg("--device-name")
        .arg(&cfg.device_name)
        .arg("--owner-label")
        .arg(&cfg.owner_label)
        .arg("--capacity-limit-bytes")
        .arg(cfg.capacity_limit_bytes.to_string())
        .arg("--enabled")
        .arg(if cfg.enabled { "true" } else { "false" })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    Ok(cmd.spawn()?)
}

fn worker_store_base(profile: &str) -> AnyResult<PathBuf> {
    Ok(data_dir()?.join("worker_store").join(profile))
}

fn scan_storage(profile: &str) -> AnyResult<StorageResp> {
    let base = worker_store_base(profile)?;
    if !base.exists() {
        return Ok(StorageResp {
            profile: profile.to_string(),
            used_bytes: 0,
            hosted_shards: 0,
        });
    }

    let mut used_bytes = 0u64;
    let mut hosted_shards = 0usize;

    fn walk(path: &Path, used: &mut u64, shards: &mut usize) -> AnyResult<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let p = entry.path();
            let ftype = entry.file_type()?;
            if ftype.is_dir() {
                walk(&p, used, shards)?;
                continue;
            }
            if ftype.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".bin") {
                    *shards += 1;
                    *used += entry.metadata()?.len();
                }
            }
        }
        Ok(())
    }

    walk(&base, &mut used_bytes, &mut hosted_shards)?;

    Ok(StorageResp {
        profile: profile.to_string(),
        used_bytes,
        hosted_shards,
    })
}

async fn status(State(state): State<AppState>) -> Json<WorkerStatusResp> {
    let mut guard = state.inner.lock().await;
    refresh_process_state(&mut guard);
    Json(snapshot_status(&guard))
}

async fn start(State(state): State<AppState>) -> Result<Json<ActionResp>, (StatusCode, String)> {
    let cfg = {
        let mut guard = state.inner.lock().await;
        refresh_process_state(&mut guard);
        if guard.child.is_some() {
            return Ok(Json(ActionResp {
                status: "ok".to_string(),
                message: "worker already running".to_string(),
                worker: snapshot_status(&guard),
            }));
        }
        guard.config.clone()
    };

    let child = spawn_worker(&cfg).map_err(to_http_err)?;

    let mut guard = state.inner.lock().await;
    guard.last_error = None;
    guard.last_exit_code = None;
    guard.started_at_ms = Some(now_ms());
    guard.child = Some(child);

    Ok(Json(ActionResp {
        status: "ok".to_string(),
        message: "worker started".to_string(),
        worker: snapshot_status(&guard),
    }))
}

async fn stop(State(state): State<AppState>) -> Result<Json<ActionResp>, (StatusCode, String)> {
    let mut child_opt = {
        let mut guard = state.inner.lock().await;
        refresh_process_state(&mut guard);
        guard.child.take()
    };

    if child_opt.is_none() {
        let mut guard = state.inner.lock().await;
        refresh_process_state(&mut guard);
        return Ok(Json(ActionResp {
            status: "ok".to_string(),
            message: "worker is not running".to_string(),
            worker: snapshot_status(&guard),
        }));
    }

    if let Some(child) = child_opt.as_mut() {
        let _ = child.kill().await;
        let status = child.wait().await.map_err(to_http_err)?;
        let mut guard = state.inner.lock().await;
        guard.started_at_ms = None;
        guard.last_exit_code = status.code();
    }

    let guard = state.inner.lock().await;
    Ok(Json(ActionResp {
        status: "ok".to_string(),
        message: "worker stopped".to_string(),
        worker: snapshot_status(&guard),
    }))
}

async fn config(
    State(state): State<AppState>,
    Json(req): Json<ConfigUpdateReq>,
) -> Result<Json<ActionResp>, (StatusCode, String)> {
    let (cfg, restart_if_running, mut old_child) = {
        let mut guard = state.inner.lock().await;
        refresh_process_state(&mut guard);

        apply_config_update(&mut guard.config, &req)?;
        save_config(&guard.config).map_err(to_http_err)?;

        let restart = req.restart_if_running.unwrap_or(true) && guard.child.is_some();
        let old_child = if restart { guard.child.take() } else { None };
        (guard.config.clone(), restart, old_child)
    };

    if let Some(child) = old_child.as_mut() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    if restart_if_running {
        let new_child = match spawn_worker(&cfg) {
            Ok(child) => child,
            Err(err) => {
                let mut guard = state.inner.lock().await;
                guard.started_at_ms = None;
                guard.last_error = Some(format!(
                    "failed to restart worker with updated config: {err}"
                ));
                return Err(to_http_err(err));
            }
        };

        let mut guard = state.inner.lock().await;
        guard.last_error = None;
        guard.last_exit_code = None;
        guard.started_at_ms = Some(now_ms());
        guard.child = Some(new_child);
    }

    let mut guard = state.inner.lock().await;
    refresh_process_state(&mut guard);
    Ok(Json(ActionResp {
        status: "ok".to_string(),
        message: if restart_if_running {
            "config updated and worker restarted".to_string()
        } else {
            "config updated".to_string()
        },
        worker: snapshot_status(&guard),
    }))
}

async fn storage(State(state): State<AppState>) -> Result<Json<StorageResp>, (StatusCode, String)> {
    let profile = {
        let guard = state.inner.lock().await;
        guard.config.profile.clone()
    };
    let summary = scan_storage(&profile).map_err(to_http_err)?;
    Ok(Json(summary))
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    let cfg = load_config().unwrap_or_default();
    let state = AppState {
        inner: Arc::new(Mutex::new(AgentState {
            config: cfg,
            child: None,
            started_at_ms: None,
            last_exit_code: None,
            last_error: None,
        })),
    };

    let bind_addr =
        std::env::var("DSPROUT_AGENT_BIND").unwrap_or_else(|_| "127.0.0.1:7081".to_string());
    let app = Router::new()
        .route("/status", get(status))
        .route("/start", post(start))
        .route("/stop", post(stop))
        .route("/config", post(config))
        .route("/storage", get(storage))
        .with_state(state);

    let listener = TcpListener::bind(&bind_addr).await?;
    println!("dsprout-agent listening on http://{bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
