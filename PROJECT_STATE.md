# PROJECT_STATE

Last updated after Milestone 9.

## current architecture

- Monorepo root:
- `app/` (Next.js frontend, not integrated yet)
- `server/` (Rust backend workspace)

- Backend crates:
- `server/dsprout-common`:
- shared identity, pnet loader, crypto, sharding, hashing, libp2p request/response protocol, shared durable manifest/signed-manifest models.
- `server/dsprout-worker`:
- libp2p worker node with profile-scoped local shard storage, RAM hot-cache, shard store/prepare/verify handlers, startup shard inventory scan + re-registration, registration + heartbeat.
- `server/dsprout-uplink`:
- libp2p client with satellite-driven worker discovery for upload (`GET /workers`), health filtering by `last_seen`, shard replication, signed manifest handling, and shard retrieval via `/locate` metadata.
- `server/dsprout-satellite`:
- axum registry/index service for workers, shard locations, and signed manifests with SQLite-backed persistence and startup reload.

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
- Persisted workers, shard records, signed manifests
- Startup restores in-memory maps from SQLite
- Shard registration remains idempotent/upsert-safe

## files changed

- `server/dsprout-uplink/src/main.rs`
- Upload worker selection now uses satellite discovery (`GET /workers`) only.
- Removed manual `--worker` argument parsing from upload flow.
- Added worker health filtering via `last_seen` (exclude workers older than 30 seconds).
- Upload placement uses only discovered healthy workers.
- Download flow continues dialing workers from `/locate` shard metadata.

- `PROJECT_STATE.md`
- Updated architecture, commands, validations, and current milestone status.

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

### 3) Start workers

```bash
cd server
cargo run -p dsprout-worker -- --profile w1 --listen /ip4/127.0.0.1/tcp/5601 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w2 --listen /ip4/127.0.0.1/tcp/5602 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w3 --listen /ip4/127.0.0.1/tcp/5603 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w4 --listen /ip4/127.0.0.1/tcp/5604 --satellite-url http://127.0.0.1:7070
```

### 4) Upload using only satellite discovery (no `--worker` args)

```bash
cd server
cargo run -p dsprout-uplink -- upload \
  --satellite-url http://127.0.0.1:7070 \
  --input /tmp/input.bin \
  --file-id milestone9-e2e \
  --replication-factor 2
```

### 5) Download using only satellite URL + file ID

```bash
cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone9-e2e \
  --output /tmp/output.bin
```

### 6) Byte equality check

```bash
cmp -s /tmp/input.bin /tmp/output.bin && echo "MATCH" || echo "MISMATCH"
```

### 7) Offline-worker recovery check

```bash
# stop some workers, then run:
cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone9-e2e \
  --output /tmp/output_offline.bin
cmp -s /tmp/input.bin /tmp/output_offline.bin && echo "MATCH" || echo "MISMATCH"
```

## validations passed

Milestone 9 validation executed successfully:

- Upload works with only `--satellite-url` (no manual `--worker` args).
- Download works with only `--satellite-url` + `--file-id` + `--output`.
- Download still succeeds with some workers offline.
- Worker discovery health filtering is active (stale workers excluded by `last_seen`).

Observed result set from validation run:
- `FILE_ID=milestone9-e2e-1772968430`
- `UPLOAD_ONLY_SATELLITE_URL_OK=1`
- `DOWNLOAD_ONLY_SATELLITE_URL_OK=1`
- `DOWNLOAD_WITH_OFFLINE_WORKERS_OK=1`
- `UNHEALTHY_FILTER_LOG=worker discovery filtered: unhealthy=4 invalid_multiaddr=0`
- `CMP_ALL_OK=1`
- `CMP_OFFLINE_OK=1`

## remaining warnings/issues

- Health threshold is fixed at 30 seconds in uplink (`WORKER_HEALTH_MAX_AGE_MS`), not yet configurable.
- SQLite writes are synchronous and optimized for simplicity, not throughput.
- No shard compaction/retention policy yet.
- `server/swarm.key` is local secret material and intentionally git-ignored.
- No Kademlia/bootstrap/gossipsub/discovery protocol yet (intentionally out of scope).
- No frontend integration yet (intentionally out of scope).
- No cloud deployment/performance optimization yet (intentionally out of scope).

## next milestone start guidance

When opening a new Codex session, paste this file first and ask for the next milestone only.
