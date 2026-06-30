# IoT Hub  
*A Web Backend for IoT Device Management — built with Axum, Tower, and Rust.*

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
| **Data Handling** | Serde, SQLx |
| **Build Tool** | Cargo |

## Build & Run

### Prerequisites
- Rust — pinned to 1.90 via `rust-toolchain.toml`; `rustup` will install/select it automatically
- Cargo package manager
- Docker (for Postgres via `docker-compose.yml`)
- sqlx-cli installed for database migrations (with the correct database feature)

>Tip: To install sqlx-cli for PostgreSQL:
> ```bash
> cargo install sqlx-cli --no-default-features --features postgres
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

### Run the server

```bash
cargo run
```

By default, the server will start on:

```
http://localhost:3000
```

## Authentication

Every endpoint except `/users/signup`, `/users/login`, and `/users/health` requires a JWT:

```
Authorization: Bearer <token>
```

`POST /users/signup` and `POST /users/login` return a token in the response body. The JWT's `sub` claim is the authenticated user's UUID (not their username).

New users default to the `Operator` role. There's no API path to create an `Admin` — if the `ADMIN_BOOTSTRAP_EMAIL` environment variable is set, a signup with that exact email is granted `Admin` instead. See `.env.example`.

## API Reference

Below is a summary of all major endpoints defined in the project's source code.

### User Management

**Base Path:** `/users`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `GET` | `/users/` | Bearer, **Admin only** | List all users. |
| `POST` | `/users/signup` | none | Register a new user. |
| `POST` | `/users/login` | none | Authenticate and return a token. |
| `GET` | `/users/me` | Bearer | Retrieve current user information. |
| `GET` | `/users/health` | none | Basic service health check. |

#### Request Bodies

**POST /users/signup**
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

**POST /users/login**
```json
{
    "username": "john_doe",
    "password": "StrongPassword123!"
}
```

### Device Management

**Base Path:** `/devices`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `POST` | `/devices/` | Bearer | Register a new IoT device, owned by the caller. |
| `GET` | `/devices/` | Bearer | List the caller's own devices. |
| `GET` | `/devices/{device_id}` | Bearer | Get details of a device the caller owns. |
| `DELETE` | `/devices/{device_id}` | Bearer | Delete a device the caller owns. |

Devices are scoped to their owner, which is always derived from the JWT — there's no `owner_id` field in the request body. Accessing a device you don't own returns `404` (not `403`), so its existence isn't leaked to non-owners.

#### Request Bodies

**POST /devices/**
```json
{
    "name": "Living Room Sensor",
    "description": "Monitors temperature and humidity in the living room"
}
```

### Device Readings

**Base Path:** `/devices/{device_id}/readings`

| Method | Endpoint | Auth | Description |
|--------|-----------|------|--------------|
| `POST` | `/devices/{device_id}/readings` | Bearer | Submit new telemetry readings for a device you own. |
| `GET` | `/devices/{device_id}/readings` | Bearer | Fetch readings for a device you own. |
| `GET` | `/devices/{device_id}/readings/latest` | Bearer | Retrieve the most recent reading. |

Like devices, these endpoints confirm the caller owns `{device_id}` before doing anything else — a device you don't own (or that doesn't exist) returns `404`.

#### Request Bodies

**POST /devices/{device_id}/readings** (Single Reading Example)
```json
{
    "arrived_timestamp": "2025-10-19T14:10:00Z",
    "reading_type": "temperature",
    "value": 22.5
}
```

**POST /devices/{device_id}/readings** (Multiple Readings Example)
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

**GET /devices/{device_id}/readings**

| Query | Type | Description |
|-------|------|-------------|
| from	| i64 (optional)	| Start timestamp (Unix seconds). |
| to	| i64 (optional)	| End timestamp (Unix seconds). |
| cursor |	i64 (optional)	| Pagination cursor (Unix timestamp). |
| limit	| usize (optional)	| Maximum number of readings to return.

## Architecture Overview

```lua
+------------------------------------------------------+
|                      IoT Hub                         |
+------------------------------------------------------+
|                  API Layer (Axum)                    |
|------------------------------------------------------|
|     /users     |     /devices   |     /readings      |
|------------------------------------------------------|
|   Middleware: Auth, Logging, Tracing, Rate Limiting  |
|------------------------------------------------------|
|        State: AppState (Postgres connection pool)    |
+------------------------------------------------------+
```


- **Axum Routers** organize endpoints into modules (`users`, `devices`, `readings`).
- **AppState** holds the shared Postgres connection pool used by every handler.
- **Tower middleware** adds rate limiting, concurrency limiting, and HTTP tracing layers.
- Services depend on repository *traits*, not concrete database types, so business logic can be unit-tested against mocks without a database.

## Rate Limiting

IoT Hub applies a global rate-limiting layer via [`tower_governor`](https://docs.rs/tower_governor), configured in `create_app`:

- **10 requests/second** sustained, with a **burst of 30**
- Applied globally to all routes, keyed by client IP
- Stacked alongside a limit of 100 concurrent in-flight requests (`tower::limit::ConcurrencyLimitLayer`)

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
