# IoT Hub  
*A Web Backend for IoT Device Management — built with Axum, Tower, and Rust.*

[![CI](https://github.com/zhshih/iot-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/zhshih/iot-hub/actions/workflows/ci.yml)

## Overview

**IoT Hub** is a web-based backend service designed to manage IoT devices, users, and telemetry readings in a structured and observable way.  
This project demonstrates practical usage of the **Axum** web framework, **Tower** middleware, and **SQLx**/Postgres in a production-style backend architecture.

The system provides RESTful APIs for:
- **User Management** — Authentication, registration, and health check endpoints.  
- **Device Management** — Register, query, and remove IoT devices.  
- **Readings** — Store and fetch telemetry data from registered devices.

This project is primarily built to explore and showcase:
- **Axum ecosystem**: request routing, extractors, middleware, and layered architecture.  
- **Observability**: structured request tracing (`tracing`) and Prometheus metrics.  
- **Clean backend design** using state management, modular routing, and async Rust.

## Tech Stack

| Component | Description |
|------------|--------------|
| **Language** | Rust |
| **Framework** | Axum |
| **Async Runtime** | Tokio |
| **Middleware** | Tower Layers (rate limiting via `tower_governor`, concurrency limiting, tracing) |
| **Auth** | JWT (`jsonwebtoken`) + Argon2 password hashing |
| **Observability** | `tracing` (structured logs/spans) + `axum-prometheus` (metrics) |
| **API Docs** | `utoipa` + `utoipa-swagger-ui` (OpenAPI 3 spec, Swagger UI) |
| **Data Handling** | Serde, SQLx |
| **Build Tool** | Cargo |

## Build & Run

### Prerequisites
- Rust — no specific version pinned; a recent stable toolchain works (`rustup` will use your default)
- Cargo package manager
- Docker (for Postgres via `docker-compose.yml`)
- sqlx-cli installed for database migrations (with the correct database feature)

>Tip: To install sqlx-cli for PostgreSQL (pinned to 0.8.6 — newer releases require a newer rustc than this repo has historically targeted; untested against 0.9.x):
> ```bash
> cargo install sqlx-cli --version "=0.8.6" --no-default-features --features postgres
>```

### Configure environment

```bash
cp .env.example .env
# then edit .env with your own values
```

### Start the database

```bash
docker compose up -d
```

### Migrate the Database

```bash
sqlx migrate run
```

### Seed dev data (optional)

Not run automatically by migrations or CI. To populate local list/read endpoints with sample users, devices, and readings:

```bash
psql "$DATABASE_URL" -f scripts/seed_dev.sql
```

### Run the server

```bash
cargo run
```

By default, the server binds to `0.0.0.0:3000`, so it's reachable at:

```
http://localhost:3000
```

Set `BIND_ADDR` in `.env` to override the bind address/port.

## Deployment

The app is built to run as a container behind a TLS-terminating reverse proxy or load balancer — it only ever serves plain HTTP itself.

```bash
docker build -t iot-hub .
docker run --rm -p 3000:3000 --env-file .env iot-hub
```

Or pull the image CI already built and pushed on the latest merge to `main`:

```bash
docker pull ghcr.io/zhshih/iot-hub:latest
```

Or run the app alongside its own Postgres via `docker-compose.prod.yml` (separate from the dev/CI `docker-compose.yml`, which only stands up a database):

```bash
docker compose -f docker-compose.prod.yml up -d db
JWT_SECRET=... DATABASE_URL=postgres://postgres:password@localhost:5433/iot_monitoring sqlx migrate run
docker compose -f docker-compose.prod.yml up -d app
```

On shutdown, the server listens for SIGTERM (sent by container orchestrators on redeploy/scale-down) as well as Ctrl+C/SIGINT locally, and lets in-flight requests finish before exiting — give it a shutdown grace period long enough for your slowest request.

Database migrations (`sqlx migrate run`) are not run automatically by the container; apply them against the target database as a separate step before/during rollout, as above. In production, also set `LOG_FORMAT=json` for log pipeline compatibility.

## Authentication

Every endpoint except `/api/v1/users/signup`, `/api/v1/users/login`, and `/api/v1/users/health` requires a JWT:

```
Authorization: Bearer <token>
```

`POST /api/v1/users/signup` and `POST /api/v1/users/login` return a token in the response body. The JWT's `sub` claim is the authenticated user's UUID (not their username).

New users default to the `Operator` role. There's no API path to create an `Admin` — if the `ADMIN_BOOTSTRAP_EMAIL` environment variable is set, a signup with that exact email is granted `Admin` instead. See `.env.example`.

## API Reference

Below is a summary of all major endpoints defined in the project's source code. All routes are versioned under `/api/v1` (the `/metrics` endpoint is the one exception — see Observability).

### User Management

**Base Path:** `/api/v1/users`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `GET` | `/api/v1/users` | Bearer, **Admin only** | List all users. |
| `POST` | `/api/v1/users/signup` | none | Register a new user. |
| `POST` | `/api/v1/users/login` | none | Authenticate and return a token. |
| `GET` | `/api/v1/users/me` | Bearer | Retrieve current user information. |
| `PATCH` | `/api/v1/users/me` | Bearer | Update the caller's own `username`/`email` (partial update). |
| `GET` | `/api/v1/users/health` | none | Basic service health check. |

#### Request Bodies

**POST /api/v1/users/signup**
```json
{
    "username": "john_doe",
    "email": "john@example.com",
    "password": "StrongPassword123!"
}
```
Response includes both the token and the new user's id:
```json
{
    "status": "success",
    "data": { "token": "...", "user_id": "b07b2c4e-9d75-4f54-8e9d-4b0d37e624af" }
}
```

**POST /api/v1/users/login**
```json
{
    "username": "john_doe",
    "password": "StrongPassword123!"
}
```

**PATCH /api/v1/users/me** (all fields optional; omitted fields are left unchanged)
```json
{
    "email": "new-address@example.com"
}
```

### Device Management

**Base Path:** `/api/v1/devices`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `POST` | `/api/v1/devices` | Bearer | Register a new IoT device, owned by the caller. |
| `GET` | `/api/v1/devices` | Bearer | List the caller's own devices. |
| `GET` | `/api/v1/devices/{device_id}` | Bearer | Get details of a device the caller owns. |
| `PATCH` | `/api/v1/devices/{device_id}` | Bearer | Update `name`/`description`/`is_active` on a device the caller owns (partial update). |
| `DELETE` | `/api/v1/devices/{device_id}` | Bearer | Delete a device the caller owns. |

Devices are scoped to their owner, which is always derived from the JWT — there's no `owner_id` field in the request body. Accessing a device you don't own returns `404` (not `403`), so its existence isn't leaked to non-owners.

> Note: the collection-root endpoints (`/api/v1/devices`, `/api/v1/users`) only match without a trailing slash — this is how Axum's `nest()` maps a nested router's own `/` route. Every other endpoint below is unaffected.

#### Request Bodies

**POST /api/v1/devices**
```json
{
    "name": "Living Room Sensor",
    "description": "Monitors temperature and humidity in the living room"
}
```

**PATCH /api/v1/devices/{device_id}** (all fields optional; omitted fields are left unchanged)
```json
{
    "is_active": false
}
```

### Device Readings

**Base Path:** `/api/v1/devices/{device_id}/readings`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `POST` | `/api/v1/devices/{device_id}/readings` | Bearer | Submit new telemetry readings for a device you own. |
| `GET` | `/api/v1/devices/{device_id}/readings` | Bearer | Fetch readings for a device you own. |
| `GET` | `/api/v1/devices/{device_id}/readings/latest` | Bearer | Retrieve the most recent reading. |

Like devices, these endpoints confirm the caller owns `{device_id}` before doing anything else — a device you don't own (or that doesn't exist) returns `404`.

#### Request Bodies

**POST /api/v1/devices/{device_id}/readings** (Single Reading Example)
```json
{
    "arrived_timestamp": "2025-10-19T14:10:00Z",
    "reading_type": "temperature",
    "value": 22.5
}
```

**POST /api/v1/devices/{device_id}/readings** (Multiple Readings Example)
```json
[
    {
        "arrived_timestamp": "2025-10-19T14:10:00Z",
        "reading_type": "temperature",
        "value": 22.5
    },
    {
        "arrived_timestamp": "2025-10-19T14:11:00Z",
        "reading_type": "humidity",
        "value": 44.8
    }
]
```

#### Query Parameters

**GET /api/v1/devices/{device_id}/readings**

| Query | Type | Description |
|-------|------|-------------|
| from	| i64 (optional)	| Start timestamp (Unix seconds). |
| to	| i64 (optional)	| End timestamp (Unix seconds). |
| cursor |	i64 (optional)	| Pagination cursor (Unix timestamp). |
| limit	| usize (optional)	| Maximum number of readings to return.

## API Documentation

An OpenAPI 3 spec is generated from the handler/DTO annotations via [`utoipa`](https://docs.rs/utoipa) and [`utoipa-axum`](https://docs.rs/utoipa-axum):

- `GET /docs` — interactive Swagger UI
- `GET /api-docs/openapi.json` — the raw spec

Both are public/unauthenticated (read-only documentation, no sensitive data), and sit outside `/api/v1` and outside any rate-limited group, same treatment as `/metrics`. The spec's paths and schemas come directly from the same `OpenApiRouter`/`routes!(...)` calls that register the real routes — there's no separate hand-maintained list to fall out of sync; adding, removing, or changing a route automatically updates the spec.

## Architecture Overview

```lua
+----------------------------------------------------------------+
|                            IoT Hub                              |
+----------------------------------------------------------------+
|                      API Layer (Axum, /api/v1)                  |
|------------------------------------------------------------------|
| /api/v1/users  |  /api/v1/devices  (+ nested .../readings)       |
|------------------------------------------------------------------|
| Per-group: rate limiting + concurrency limit | Auth (per-handler) |
|------------------------------------------------------------------|
|      Global: Prometheus metrics + HTTP tracing (TraceLayer)      |
|------------------------------------------------------------------|
|          State: AppState (Postgres connection pool)              |
+----------------------------------------------------------------+
```
`/metrics` is mounted outside `/api/v1` and outside any rate-limited group, since it's a Prometheus scrape endpoint, not a versioned client-facing resource.

- **Axum Routers** organize endpoints into modules (`users`, `devices`, `readings`); `readings` nests under the same `/api/v1/devices` prefix as `devices` since a reading always belongs to a device.
- **AppState** holds the shared Postgres connection pool used by every handler.
- **Tower middleware**: rate limiting and concurrency limiting are applied independently per route group (see Rate Limiting below); HTTP tracing and Prometheus metrics wrap the whole app once, globally.
- Services depend on repository *traits*, not concrete database types, so business logic can be unit-tested against mocks without a database.

## Rate Limiting

IoT Hub applies rate limiting via [`tower_governor`](https://docs.rs/tower_governor) and a concurrency cap via `tower::limit::ConcurrencyLimitLayer`, configured independently **per route group** (`/api/v1/users`, `/api/v1/devices` — which also covers nested readings routes) rather than shared globally:

- **10 requests/second** sustained, with a **burst of 30**
- A limit of **100 concurrent in-flight requests** per group
- Each group has its own independent budget, so a hot endpoint in one group can't starve unrelated groups
- Keyed by the authenticated caller (the JWT's `sub` claim), not just client IP — two users behind the same IP/proxy get independent budgets. Requests with no valid token (signup, login, health, or any request that's simply missing/invalid) fall back to peer-IP keying.

## Observability

This project uses **`tracing`** for structured request logging and **Prometheus** for metrics.

**Features**:
- Per-request tracing spans (method, URI, HTTP version, status, latency)
- Structured logs (pretty-printed to stdout, plus a JSON-formatted layer)
- Prometheus metrics collection via `axum-prometheus`

**Metrics**:

Collected metrics include HTTP request count, latency, and status per endpoint.

**Example:**
```bash
RUST_LOG=info cargo run
```

Metrics are exposed at:
```bash
GET /metrics
```

## License

This project is distributed under the MIT License.
