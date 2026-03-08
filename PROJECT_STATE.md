# PROJECT_STATE

Last updated after Milestone 11.

## current architecture

- Monorepo root:
- `app/` (Next.js dashboard)
- `server/` (Rust backend workspace)

- Backend crates:
- `server/dsprout-common`:
- shared identity, pnet loader, crypto, sharding, hashing, libp2p request/response protocol, shared durable manifest/signed-manifest models.
- `server/dsprout-worker`:
- libp2p worker node with profile-scoped local shard storage, RAM hot-cache, shard store/prepare/verify handlers, startup shard inventory scan + re-registration, registration + heartbeat.
- `server/dsprout-uplink`:
- libp2p client with satellite-driven upload worker discovery, health filtering, shard replication, signed manifest handling, download from `/locate`, and file-scoped repair command.
- `server/dsprout-satellite`:
- axum registry/index service for workers, shard locations, and signed manifests with SQLite persistence and startup reload.

- Frontend dashboard (`app/`):
- Minimal read-only Next.js page for cluster visibility.
- Reads satellite endpoints directly from server-rendered page:
- `GET /workers`
- `GET /manifest?file_id=...`
- `GET /locate?file_id=...`
- Shows worker table with health status computed from `last_seen` lag.
- Provides file lookup form (`file_id`) and per-file summary.
- No upload/download/repair actions in UI yet.

## files changed

- `app/app/page.tsx`
- Replaced default starter page with dashboard view.
- Added worker list section using `/workers`.
- Added health status derived from `last_seen` lag.
- Added file lookup section for `/manifest` and `/locate`.
- Added per-file summary:
- `file_id`
- segment count
- shard record count
- unique shard count
- replica counts (min/max/avg)

- `PROJECT_STATE.md`
- Updated for Milestone 11.

## commands to run

All commands below are from repository root (`dsprout`).

### 1) Start backend services

```bash
cd server
cargo run -p dsprout-satellite
cargo run -p dsprout-worker -- --profile w1 --listen /ip4/127.0.0.1/tcp/5701 --satellite-url http://127.0.0.1:7070
cargo run -p dsprout-worker -- --profile w2 --listen /ip4/127.0.0.1/tcp/5702 --satellite-url http://127.0.0.1:7070
```

### 2) Run dashboard

```bash
cd app
npm install
npm run dev
```

### 3) Open dashboard

```text
http://localhost:3000
```

### 4) Optional satellite URL override

```bash
cd app
SATELLITE_URL=http://127.0.0.1:7070 npm run dev
```

## validations passed

Milestone 11 validation executed:

- `npm run lint` passed for dashboard changes.
- Dashboard page compiles under lint/type checks and uses required endpoints.
- Functional scope implemented:
- worker list with health status
- file lookup for manifest + locate
- per-file summary with replica counts

Validation note:
- `npm run build` failed in sandbox because existing `next/font/google` setup could not fetch external fonts (network-restricted environment), not due dashboard code logic.

## remaining warnings/issues

- Dashboard is read-only by design (no upload/download/repair actions yet).
- Worker health in UI is based on relative `last_seen` lag from latest worker timestamp.
- Next.js production build in this sandbox may fail due blocked external font fetch from Google Fonts.
- No Kademlia/bootstrap/gossipsub/cloud deployment/performance optimization yet (intentionally out of scope).

## next milestone start guidance

When opening a new Codex session, paste this file first and ask for the next milestone only.
