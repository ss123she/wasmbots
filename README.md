# WASM Bot Arena

3D-Arena with bots, compiled to WASM, that fight each other.

## Structure

- `backend/` - Server written in Rust (`axum` + `hecs` + `wasmtime` + `glam`).
- `frontend/` - Client written in Next.js + Three.js with OrbitControls.
- `bots/` - Source code for presets on Rust. (target wasm32-unknown-unknown)

## Quick start

### 1. Backend

```bash
cd backend
cargo run
# Will run on http://0.0.0.0:8000
```

> Presets are in `backend/presets`

### 2. Frontend

```bash
cd frontend
bun install
bun run dev
```

OR

### 2+3. Just

```bash
just
# Runs whole frontend + backend via justfile (requires just installed).
```

### 3. Build bots to WASM

```bash
rustup target add wasm32-unknown-unknown
cd bots/hunter
cargo build --target wasm32-unknown-unknown --release
```

## Server API

- `GET /ws` - Websocket with Arena Snapshot every tick.
- `POST /api/bots?name=MyBot` - Load `.wasm`-bot. Body is WASM bytes.
- `POST /api/presets/{kind}` - Spawn preset (`hunter` | `sniper`).
- `POST /api/reset` - Reset arena.

## WASM ABI for bots

### Bot exports

- `tick()` _(required)_ - calls every tick.
- `build()` _(optional)_ - calls once on spawn.

### Bot imports from `env`

- **Build:** `credits() -> i32`, `place_module(gx, gy, kind, dir_deg) -> i32`
- `kind`: `0` Core, `1` Armor, `2` Thruster, `3` Cannon, `4` Radar, `5` Reactor
- **State:** `get_self(stat) -> f32` (`0` hp, `1` x, `2` y, `3` angle, `4` cooldown, `5` credits, `6` modules), `module_hp(id) -> f32`
- **Sensors:** `scan_enemy(offset_deg, fov_deg) -> f32`, `scan_from(id, offset_deg, fov_deg) -> f32`, `scan_orb() -> f32`
- **Controls:** `drive(speed, turn)`, `fire() -> i32`, `fire_module(id) -> i32`
