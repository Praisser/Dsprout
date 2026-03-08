# PROJECT_STATE

Last updated after Milestone 7.

## current architecture

- Monorepo root:
- `app/` (Next.js frontend, not integrated yet)
- `server/` (Rust backend workspace)

- Backend crates:
- `server/dsprout-common`:
- shared identity, pnet loader, crypto, sharding, hashing, libp2p request/response protocol, shared durable manifest/signed-manifest models.
- `server/dsprout-worker`:
- libp2p worker node, local shard storage, RAM hot-cache, shard store/prepare/verify handlers, CLI profile/listen config, satellite registration/heartbeat.
- `server/dsprout-uplink`:
- libp2p client, upload/download pipeline, satellite client, multi-worker placement, shard replication (`--replication-factor`, default `2`), local signed-manifest cache, manifest register/fetch logic.
- `server/dsprout-satellite`:
- axum registry/index service for workers, shard locations, and signed manifests, now with SQLite-backed persistence and startup reload.

- Network protocol (request-response over private libp2p transport):
- `Hello` / `HelloAck`
- `Prepare` / `PrepareAck`
- `VerifyGet` / `VerifyGetOk`
- `StoreShard` / `StoreShardAck`
- `Error`

- Satellite HTTP API (compatible behavior retained):
- `POST /register_worker`
- `POST /heartbeat`
- `GET /workers`
- `POST /register_shard`
- `GET /locate?file_id=...`
- `POST /register_manifest`
- `GET /manifest?file_id=...`

- Satellite persistence:
- SQLite database at `~/Library/Application Support/dsprout/satellite.sqlite3`
- Persisted tables: workers, shard records, signed manifests
- On startup, satellite loads persisted records into in-memory indexes
- On writes, endpoints persist updates and keep in-memory state in sync

## files changed

- `server/dsprout-satellite/src/main.rs`
- Added `PersistentStore` with SQLite schema creation and CRUD for workers/shards/manifests.
- Added startup load path to rebuild in-memory maps from persisted records.
- Updated write handlers to persist on:
- `register_worker`
- `heartbeat`
- `register_shard`
- `register_manifest`
- Kept HTTP API routes and response semantics compatible.

- `server/dsprout-satellite/Cargo.toml`
- Added `rusqlite` (bundled SQLite) and `dirs` dependencies.

- `server/Cargo.lock`
- Updated for new satellite dependencies.

## commands to run

All commands below are from repository root (`dsprout`).

### 1) Build backend

```bash
cd server
cargo build -p dsprout-common -p dsprout-satellite -p dsprout-worker -p dsprout-uplink
```

### 2) Start satellite

```bash
cd server
cargo run -p dsprout-satellite
```

### 3) Start workers (example)

```bash
cd server
cargo run -p dsprout-worker -- --profile w1 --listen /ip4/127.0.0.1/tcp/5301 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w2 --listen /ip4/127.0.0.1/tcp/5302 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w3 --listen /ip4/127.0.0.1/tcp/5303 --satellite-url http://127.0.0.1:7070
```

### 4) Upload (replication factor 2)

```bash
cd server
cargo run -p dsprout-uplink -- upload \
  --satellite-url http://127.0.0.1:7070 \
  --input /tmp/input.bin \
  --file-id milestone7-e2e \
  --replication-factor 2 \
  --worker /ip4/127.0.0.1/tcp/5301 \
  --worker /ip4/127.0.0.1/tcp/5302 \
  --worker /ip4/127.0.0.1/tcp/5303
```

### 5) Restart-safety validation

```bash
# 1) stop satellite process
# 2) start satellite again
cd server
cargo run -p dsprout-satellite
```

### 6) Download after restart

```bash
# optional to force satellite-manifest path:
rm -f "$HOME/Library/Application Support/dsprout/uplink_meta/milestone7-e2e.json"

cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone7-e2e \
  --output /tmp/output.bin
```

### 7) Byte equality check

```bash
cmp -s /tmp/input.bin /tmp/output.bin && echo "MATCH" || echo "MISMATCH"
```

## validations passed

Milestone 7 validation executed successfully:

- Build passed for `dsprout-satellite`, `dsprout-uplink`, and `dsprout-worker`.
- Upload succeeded.
- Satellite was stopped and restarted.
- Local uplink manifest was deleted.
- Download succeeded after satellite restart (using persisted satellite state).
- Restored file matched original exactly (`cmp` success).

Observed result set from validation run:
- `FILE_ID=milestone7-e2e-1772967128`
- `UPLOAD_OK=1`
- `SATELLITE_RESTARTED=1`
- `LOCAL_MANIFEST_DELETED=1`
- `DOWNLOAD_AFTER_RESTART_OK=1`
- `CMP_OK=1`
- `DB_PATH=/Users/apple/Library/Application Support/dsprout/satellite.sqlite3`

## remaining warnings/issues

- SQLite writes are synchronous and optimized for simplicity, not throughput.
- Shard record table currently appends entries; no dedup/pruning/compaction policy yet.
- `server/swarm.key` is local secret material and intentionally git-ignored at repo root.
- No Kademlia/bootstrap/gossipsub/discovery yet (intentionally out of scope).
- No frontend integration yet (intentionally out of scope).
- No cloud deployment/performance optimization yet (intentionally out of scope).

## next milestone start guidance

When opening a new Codex session, paste this file first and ask for the next milestone only.
