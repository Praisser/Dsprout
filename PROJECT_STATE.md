# PROJECT_STATE

Last updated after Milestone 4.

## Current Architecture

- Monorepo root:
- `app/` (Next.js frontend, not integrated yet)
- `server/` (Rust backend workspace)

- Backend crates:
- `server/dsprout-common`:
- shared identity, pnet loader, crypto, sharding, hashing, network protocol/types.
- `server/dsprout-worker`:
- libp2p worker node, local shard storage, RAM hot-cache, shard store/prepare/verify handlers, CLI profile/listen config, satellite registration/heartbeat.
- `server/dsprout-uplink`:
- libp2p client, upload/download pipeline, satellite client, round-robin multi-worker placement, multi-worker retrieval and reconstruction.
- `server/dsprout-satellite`:
- axum registry/index service for workers and shard locations.

- Network protocol (request-response over private libp2p transport):
- `Hello` / `HelloAck`
- `Prepare` / `PrepareAck`
- `VerifyGet` / `VerifyGetOk`
- `StoreShard` / `StoreShardAck`
- `Error`

- libp2p transport stack:
- Ed25519 identity (profile-scoped key files)
- PSK private network via `server/swarm.key`
- TCP + pnet + noise + yamux
- identify + request_response behaviours

## Files Changed (Milestone 4)

- `server/dsprout-worker/src/main.rs`
- Added `--profile`, `--listen`, `--satellite-url` runtime args.
- Added satellite self-register + periodic heartbeat.
- Kept shard handlers (`Prepare`, `VerifyGet`, `StoreShard`, `Hello`).

- `server/dsprout-worker/Cargo.toml`
- Added `reqwest` + `serde` for satellite HTTP calls.

- `server/dsprout-satellite/src/main.rs`
- Added `POST /register_worker`, `GET /workers`.
- Worker registry now stores `worker_id`, `multiaddr`, `last_seen`.
- Heartbeat updates worker metadata.
- Shard records now include `worker_multiaddr`.

- `server/dsprout-uplink/src/main.rs`
- Upload now supports multiple workers:
- repeated `--worker <multiaddr>` OR fallback to satellite `/workers`.
- round-robin shard placement across connected workers.
- shard registration includes correct worker id + multiaddr.
- Download now supports multi-worker retrieval:
- queries satellite shard locations.
- dials relevant workers.
- sends per-worker `Prepare`.
- fetches sufficient shards across online workers.
- reconstructs + decrypts with offline worker tolerance.

- `server/Cargo.lock`
- Updated due dependency graph changes.

## Commands To Run

All commands below are from repository root (`dsprout`).

### 1) Build backend

```bash
cd server
cargo build -p dsprout-satellite -p dsprout-worker -p dsprout-uplink
```

### 2) Start satellite

```bash
cd server
cargo run -p dsprout-satellite
```

### 3) Start multiple workers locally

```bash
cd server
cargo run -p dsprout-worker -- --profile w1 --listen /ip4/127.0.0.1/tcp/4101 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w2 --listen /ip4/127.0.0.1/tcp/4102 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w3 --listen /ip4/127.0.0.1/tcp/4103 --satellite-url http://127.0.0.1:7070
```

### 4) Upload with explicit workers (round-robin placement)

```bash
cd server
cargo run -p dsprout-uplink -- upload \
  --satellite-url http://127.0.0.1:7070 \
  --input /tmp/input.bin \
  --file-id milestone4-e2e \
  --worker /ip4/127.0.0.1/tcp/4101 \
  --worker /ip4/127.0.0.1/tcp/4102 \
  --worker /ip4/127.0.0.1/tcp/4103
```

### 5) Download and reconstruct

```bash
cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone4-e2e \
  --output /tmp/output.bin
```

### 6) Byte equality check

```bash
cmp -s /tmp/input.bin /tmp/output.bin && echo "MATCH" || echo "MISMATCH"
```

### 7) Optional: seed worker shard manually (debug path)

```bash
cd server
cargo run -p dsprout-worker -- seed --file-id demo-file --segment 0 --shard 7 --data "test-bytes"
```

## Validations Passed

Milestone 4 validation executed successfully:

- Upload across multiple workers succeeded.
- Download with all workers online succeeded.
- Download with one worker offline succeeded (as enough shards remained online).
- Hash and byte equality checks matched.

Observed result set from validation run:
- `UPLOAD_EXIT=0`
- `DOWNLOAD_ALL_EXIT=0`
- `CMP_ALL_EXIT=0`
- `DOWNLOAD_OFFLINE_EXIT=0`
- `CMP_OFFLINE_EXIT=0`
- `equal=true` for both online and one-offline download runs.

## Remaining Warnings / Issues

- `server/swarm.key` is local secret material and intentionally git-ignored at repo root.
- Current placement strategy is simple round-robin, no replication policy tuning yet.
- No Kademlia/bootstrap/gossipsub/discovery yet (intentionally out of scope).
- No frontend integration yet (intentionally out of scope).
- Download currently reconnects to workers each run (acceptable for milestone scope).
- No advanced retry/backoff/telemetry around worker request failures yet.

## Next Milestone Start Guidance

When opening a new Codex session, paste this file first and ask for the next milestone only.
