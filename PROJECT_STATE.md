# PROJECT_STATE

Last updated after Milestone 6.

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
- axum registry/index service for workers, shard locations (including replicas), and signed manifests.

- Network protocol (request-response over private libp2p transport):
- `Hello` / `HelloAck`
- `Prepare` / `PrepareAck`
- `VerifyGet` / `VerifyGetOk`
- `StoreShard` / `StoreShardAck`
- `Error`

- Satellite HTTP API:
- `POST /register_worker`
- `POST /heartbeat`
- `GET /workers`
- `POST /register_shard`
- `GET /locate?file_id=...`
- `POST /register_manifest`
- `GET /manifest?file_id=...`

- libp2p transport stack:
- Ed25519 identity (profile-scoped key files)
- PSK private network via `server/swarm.key`
- TCP + pnet + noise + yamux
- identify + request_response behaviours

## files changed

- `server/dsprout-uplink/src/main.rs`
- Added upload-time replication factor support via `--replication-factor` (default `2`).
- Upload now stores each shard on multiple workers per replication factor.
- Upload now registers all shard replica locations with satellite.
- Added validation guards: replication factor must be `>= 1` and `<= connected workers`.
- Download continues using any reachable replica per shard (now with richer satellite shard records due to replication).

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

### 3) Start workers (example with 5 workers for stronger recovery test)

```bash
cd server
cargo run -p dsprout-worker -- --profile w1 --listen /ip4/127.0.0.1/tcp/5201 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w2 --listen /ip4/127.0.0.1/tcp/5202 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w3 --listen /ip4/127.0.0.1/tcp/5203 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w4 --listen /ip4/127.0.0.1/tcp/5204 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w5 --listen /ip4/127.0.0.1/tcp/5205 --satellite-url http://127.0.0.1:7070
```

### 4) Upload with replication factor 2

```bash
cd server
cargo run -p dsprout-uplink -- upload \
  --satellite-url http://127.0.0.1:7070 \
  --input /tmp/input.bin \
  --file-id milestone6-e2e \
  --replication-factor 2 \
  --worker /ip4/127.0.0.1/tcp/5201 \
  --worker /ip4/127.0.0.1/tcp/5202 \
  --worker /ip4/127.0.0.1/tcp/5203 \
  --worker /ip4/127.0.0.1/tcp/5204 \
  --worker /ip4/127.0.0.1/tcp/5205
```

### 5) Download and reconstruct

```bash
cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone6-e2e \
  --output /tmp/output.bin
```

### 6) Byte equality check

```bash
cmp -s /tmp/input.bin /tmp/output.bin && echo "MATCH" || echo "MISMATCH"
```

### 7) Stronger recovery validation (take two workers offline)

```bash
# stop any 2 workers, then:
cd server
cargo run -p dsprout-uplink -- download \
  --satellite-url http://127.0.0.1:7070 \
  --file-id milestone6-e2e \
  --output /tmp/output.bin
cmp -s /tmp/input.bin /tmp/output.bin && echo "MATCH" || echo "MISMATCH"
```

## validations passed

Milestone 6 validation executed successfully:

- Build passed for `dsprout-uplink`, `dsprout-satellite`, and `dsprout-worker`.
- Upload with `--replication-factor 2` succeeded.
- Satellite registered replica locations for shards.
- Download succeeded with two workers offline.
- Restored file matched original exactly (`cmp` success).

Observed result set from validation run:
- `FILE_ID=milestone6-e2e-1772966812`
- `REPLICATION_FACTOR=2`
- `OFFLINE_WORKERS=2`
- `UPLOAD_OK=1`
- `DOWNLOAD_OK=1`
- `CMP_OK=1`

## remaining warnings/issues

- Satellite manifest and shard indexes are currently in-memory only (not persistent across satellite restart).
- `server/swarm.key` is local secret material and intentionally git-ignored at repo root.
- Replication strategy is simple deterministic placement; no adaptive policy/rebalancing yet.
- No Kademlia/bootstrap/gossipsub/discovery yet (intentionally out of scope).
- No frontend integration yet (intentionally out of scope).
- Download reconnects to workers each run (acceptable for current milestone scope).
- No advanced retry/backoff/telemetry around worker request failures yet.

## next milestone start guidance

When opening a new Codex session, paste this file first and ask for the next milestone only.
