# IoT Hub  
*A Web Backend for IoT Device Management — built with Axum, OpenTelemetry, and Rust.*

## Overview

**IoT Hub** is a web-based backend service designed to manage IoT devices, users, and telemetry readings in a structured and observable way.  
This project demonstrates practical usage of the **Axum** web framework, **Tower** middleware, and **OpenTelemetry** tracing in a production-style backend architecture.

The system provides RESTful APIs for:
- **User Management** — Authentication, registration, and health check endpoints.  
- **Device Management** — Register, query, and remove IoT devices.  
- **Readings** — Store and fetch telemetry data from registered devices.

This project is primarily built to explore and showcase:
- **Axum ecosystem**: request routing, extractors, middleware, and layered architecture.  
- **OpenTelemetry integration**: structured tracing and metrics collection.  
- **Clean backend design** using state management, modular routing, and async Rust.

## Tech Stack

| Component | Description |
|------------|--------------|
| **Language** | Rust |
| **Framework** | Axum |
| **Async Runtime** | Tokio |
| **Middleware** | Tower Layers |
| **Observability** | OpenTelemetry + Tracing |
| **Data Handling** | Serde, SQLx |
| **Build Tool** | Cargo |

## Build & Run

### Prerequisites
- Rust (latest stable)
- Cargo package manager
- sqlx-cli installed for database migrations (with the correct database feature)

>Tip: To install sqlx-cli for PostgreSQL:
> ```bash
> cargo install sqlx-cli --no-default-features --features postgres
>```

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

## API Reference

Below is a summary of all major endpoints defined in the project’s source code.

### User Management

**Base Path:** `/users`

| Method | Endpoint | Description |
|--------|-----------|--------------|
| `GET` | `/users/` | List all users. |
| `POST` | `/users/signup` | Register a new user. |
| `POST` | `/users/login` | Authenticate and return a token. |
| `GET` | `/users/me` | Retrieve current user information (authenticated). |
| `GET` | `/users/health` | Basic service health check. |

#### Request Bodies

**POST /users/signup**
```json
{
    "username": "john_doe",
    "email": "john@example.com",
    "password": "StrongPassword123!"
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

| Method | Endpoint | Description |
|--------|-----------|--------------|
| `POST` | `/devices/` | Register a new IoT device. |
| `GET` | `/devices/` | Retrieve all registered devices. |
| `GET` | `/devices/{device_id}` | Get details of a specific device. |
| `DELETE` | `/devices/{device_id}` | Delete a registered device. |

#### Request Bodies

**POST /devices/**
```json
{
    "name": "Living Room Sensor",
    "owner_id": "b07b2c4e-9d75-4f54-8e9d-4b0d37e624af",
    "description": "Monitors temperature and humidity in the living room"
}
```

### Device Readings

**Base Path:** `/devices/{device_id}/readings`

| Method | Endpoint | Description |
|--------|-----------|--------------|
| `POST` | `/devices/{device_id}/readings` | Submit new telemetry readings for a device. |
| `GET` | `/devices/{device_id}/readings` | Fetch all readings for a device. |
| `GET` | `/devices/{device_id}/readings/latest` | Retrieve the most recent reading. |

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
|     State: AppState (DB pool, config, telemetry)     |
+------------------------------------------------------+
```


- **Axum Routers** organize endpoints into modules (`users`, `devices`, `readings`).
- **AppState** centralizes shared data (database pool, configuration, tracing context).
- **OpenTelemetry** enables distributed tracing for performance and debugging.
- **Tower middleware** adds authentication, logging, rate limiting, and error handling layers.

## Rate Limiting

To ensure fair usage and protect the backend, IoT Hub applies a configurable Rate Limiting layer using Tower’s RateLimitLayer or a custom middleware.

**Purpose**:

Restricts how many requests a client can make within a given time window to prevent overload or abuse.

**Implementation**:

Applied globally or per route using:

```rust
RateLimitLayer::new(requests, per)
```

- requests: max number of allowed requests
- per: time interval (Duration) before reset

Integrates seamlessly with Axum and OpenTelemetry for tracing and observability.

## Observability

This project integrates **OpenTelemetry** and **Tracing** to provide deep visibility into the system’s behavior and performance.

**Features**:
- Request span tracing  
- Structured logs with correlation IDs  
- Performance metrics collection  

**Metrics**:

IoT Hub exposes runtime metrics that help monitor API performance and system health.

Collected metrics include:

- HTTP request count & latency per endpoint
- Active connections and error rates
- Rate limit events (throttled requests)
- Database query duration (if enabled)

**Example:**
```bash
RUST_LOG=info,otel=debug cargo run
```

Metrics are exposed at:
```bash
GET /metrics
```

## License

This project is distributed under the MIT License.