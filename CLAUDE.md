# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Toolchain

No pinned version — any recent stable toolchain works. This repo previously pinned Rust 1.90 via `rust-toolchain.toml` because `serde 1.0.228` failed to compile under newer stable compilers (`E0223 ambiguous associated type`); bumping to `serde 1.0.229`+ fixed the regression upstream, so the pin was removed. If a similarly obscure compile failure shows up again, check whether it's a pinned dependency needing a bump before reaching for a toolchain pin.

## Commands

```bash
# Run the server
cargo run

# Build
cargo build

# Run all tests (unit + integration)
cargo test

# Run integration tests only (requires test DB running)
cargo test --test integration

# Run a single integration suite by module path substring
cargo test --test integration -- users::
cargo test --test integration -- devices::
cargo test --test integration -- readings::

# Run unit tests only (no DB needed)
cargo test --lib

# Run a single test by name
cargo test test_signup_should_create_user

# Run with mock-auth feature (bypasses JWT for integration tests)
cargo test --features mock-auth

# Database migrations
sqlx migrate run

# Seed dev data (optional, never run automatically -- see Infrastructure below)
psql "$DATABASE_URL" -f scripts/seed_dev.sql

# Install sqlx-cli (postgres only)
# Pinned to 0.8.6 -- newer sqlx-cli releases require a newer rustc than the 1.90 this repo pins.
cargo install sqlx-cli --version "=0.8.6" --no-default-features --features postgres

# Regenerate the offline sqlx query cache (.sqlx/) after changing any query!/query_as! call --
# required for Docker builds, which have no live DB to check queries against. Needs a running
# DB with migrations applied (see Infrastructure below).
cargo sqlx prepare

# Build and run the production container image
docker build -t iot-hub .
docker run --rm -p 3000:3000 --env-file .env iot-hub

# Run the app + its own Postgres together (separate from the dev/CI docker-compose.yml)
docker compose -f docker-compose.prod.yml up -d db
DATABASE_URL=postgres://postgres:password@localhost:5433/iot_monitoring sqlx migrate run
docker compose -f docker-compose.prod.yml up -d app
```

## Infrastructure

Start the database with Docker Compose before running the server or integration tests:

```bash
docker compose up -d
```

The `init-db.sql` is mounted into the container and creates both `iot_monitoring` (production) and `iot_monitoring_test` databases along with the `test_user` role used by integration tests.

Required `.env` variables:
- `DATABASE_URL` — postgres connection string for the main DB
- `JWT_SECRET` — secret for signing JWTs

Optional `.env` variables:
- `ADMIN_BOOTSTRAP_EMAIL` — if set, a signup with this exact email is granted `Admin` instead of `Operator`. There's no other API path to create an Admin (only `scripts/seed_dev.sql` or a direct SQL insert otherwise); this is the supported way to bootstrap one.
- `LOG_FORMAT` — set to `json` to emit JSON-only logs (for production log pipelines). Unset defaults to pretty-printed logs for local dev. Only one format is ever emitted, never both.
- `BIND_ADDR` — socket address the server binds to. Defaults to `0.0.0.0:3000` when unset, so the server is reachable from outside a container without any config.
- `DB_MAX_CONNECTIONS` — max size of the Postgres connection pool (`PgPoolOptions::max_connections`). Defaults to `5` when unset.

Integration tests hardcode `postgres://test_user:test_password@localhost/iot_monitoring_test`.

**Seed data:** `scripts/seed_dev.sql` seeds sample `admin`/`operator1` users, devices, and readings (referencing each other by subquery, so they're referentially valid). It is never run automatically — not by `sqlx migrate run`, `cargo run`, or CI — only by explicitly invoking `psql "$DATABASE_URL" -f scripts/seed_dev.sql`. This is deliberate: migrations run unconditionally in every environment including production, so seed data must never live in `migrations/`.

## Architecture

The codebase follows a layered architecture with these modules:

```
api/          — Axum route handlers; thin wrappers that call services and return ApiResponse
service/      — Business logic; takes repository trait impls as generic parameters
repository/   — Database access; traits implemented on PgPool, mock impls for unit tests
domain/       — Core structs (User, Device, Reading)
dto/          — Request/response types that cross the API boundary
auth/         — JWT encode/decode (jwt.rs) + AuthUser extractor (extractor.rs)
error.rs      — AppError (domain errors) → ApiError (HTTP errors) conversion via From<AppError>
app_state.rs  — AppState { db_pool: PgPool } shared across handlers via Axum State extractor
```

**Request flow:** Handler extracts `AuthUser` (JWT validation) → calls `Service::new(state.db_pool)` → service calls repository trait → returns `HandlerResult<T>` (`Result<Json<ApiResponse<T>>, ApiError>`).

**Error handling:** Services return `AppError`. Handlers convert via `?` using `From<AppError> for ApiError`. `ApiError` implements `IntoResponse` with appropriate HTTP status codes.

**Auth:** The JWT `sub` claim holds the authenticated user's UUID, not their username (`auth/jwt.rs::Claims`, plus `Claims::user_id()` to parse it). Handlers derive `owner_id`/`requester_id` from this — never from client-supplied request fields.

**Authorization:** Devices and their readings are scoped to the authenticated caller. `DeviceService::get_device`/`delete_device` treat an ownership mismatch the same as not-found (`404`, not `403`, so existence isn't leaked to non-owners); `get_devices` lists only the caller's own devices. Reading endpoints (`api/readings.rs`) confirm device ownership via `DeviceService::get_device` before touching `ReadingService`.

**Testing strategy:**
- Unit tests (in `src/`) mock the repository trait using `mockall` and test service logic in isolation
- Integration tests (in `tests/`) use the real test database and the `mock-auth` feature flag, which replaces JWT validation with an `x-mock-user` header

**`mock-auth` feature:** When compiled with `--features mock-auth`, `AuthUser` extractor reads `x-mock-user` header — a user UUID, not a username — instead of validating a JWT Bearer token. If the header is absent, it falls back to a fixed identity (`DEFAULT_MOCK_USER_ID` in `auth/extractor.rs`). Used exclusively by integration tests — never enable in production builds.

**Routing:** each `api::{devices,readings,users}::routes()` builds a `utoipa_axum::router::OpenApiRouter<AppState>` (not a plain `axum::Router`) via `.routes(routes!(handler, ...))` calls, grouping handlers that share a path (e.g. `routes!(get_device, delete_device, update_device)` for `/{device_id}`) into one call. All API routes are mounted under `/api/v1` (`.nest("/api/v1/devices", ...)`, `.nest("/api/v1/users", ...)` in `build_router_and_openapi`); `/metrics` is the one exception, mounted unprefixed since it's a Prometheus scrape endpoint, not a versioned resource. `readings::routes()` shares the `/api/v1/devices` prefix with `devices::routes()` since a reading always belongs to a device — they're combined via `.merge()` *before* the single `.nest()` call, not via two separate `.nest()` calls at the same path. Verified against this repo's pinned axum 0.8.6 / utoipa-axum 0.2.0 directly (a prior version of this doc claimed `nest()` "silently drops" routes on a repeated literal prefix — that's wrong): two `nest()` calls at the same prefix work fine as long as the two routers' inner paths don't collide (true here — devices owns `/` and `/{device_id}`, readings owns `/{device_id}/readings...`), and if they *did* collide on an identical full path *and* HTTP method, axum panics at router-build time (`Overlapping method route ... already exists`), not a silent drop — same-path-different-method just merges into one `MethodRouter`. `merge()`-then-`nest()` is still used here because it's the structurally correct way to combine two independently-built routers under one prefix without relying on axum's overlap panic as a safety net, not because the two-`nest()` alternative silently loses routes. Separately, note that a nested router's own `/` route (e.g. `devices::routes()`'s list/register handlers) is only reachable at the bare prefix (`/api/v1/devices`), not `/api/v1/devices/` with a trailing slash — an Axum `nest()` quirk, not a bug in this app.

**Middleware stack** (applied in `create_app`): Prometheus metrics and TraceLayer HTTP tracing wrap the whole app once, globally. Rate limiting (`tower_governor`, 10 req/s, burst 30) and concurrency limiting (`tower::limit::ConcurrencyLimitLayer`, 100) are applied per route group via the `rate_limited_group` helper — devices (incl. nested readings) and users each get their own layer instances — instead of one shared pair for the whole app, since `ConcurrencyLimitLayer` has no per-key variant and would otherwise let one group starve another. `/metrics` and `/docs`/`/api-docs/openapi.json` sit outside any rate-limited group.

**Rate-limit keying:** `GovernorLayer` uses a custom `UserOrIpKeyExtractor` (`auth/rate_limit_key.rs`), not the default `PeerIpKeyExtractor` — it decodes the JWT directly from the `Authorization` header (via `auth::jwt::decode_jwt`) and keys by the `sub` claim when present, falling back to peer IP otherwise. This has to decode the token itself rather than reading `AuthUser`'s output: `GovernorLayer` is router-level middleware that runs *before* `AuthUser` (a per-handler Axum extractor) ever fires. Under the `mock-auth` feature, `decode_jwt` isn't even compiled in, so the extractor always falls back to the IP key — harmless, since mock-auth is dev/test-only and integration tests bypass `create_app`'s middleware entirely.

**OpenAPI:** the spec is generated via `utoipa` + `utoipa-axum`, with paths and schemas collected automatically from the same `#[utoipa::path(...)]`-annotated handlers that `routes!(...)` registers — there is no separate hand-maintained list; adding/changing/removing a route updates the spec by construction. `#[utoipa::path]`'s own `path = "..."` is *relative to the handler's module* (e.g. `"/"`, `"/{device_id}"`), matching what `.route()` used to take — `OpenApiRouter::nest(...)` prepends the right prefix to both the axum routes and the spec paths, same as axum's own `nest()` does for routing alone. `src/api/openapi.rs`'s `ApiDoc` now only seeds tag descriptions (`#[openapi(tags(...))]`); `lib.rs`'s `build_router_and_openapi()` does `OpenApiRouter::with_openapi(ApiDoc::openapi())...split_for_parts()` to get back a plain `(Router<AppState>, OpenApi)` pair, which is why it's split out from `create_app` — the OpenAPI half can be built and tested (`lib.rs`'s `openapi_spec_builds_from_real_routes` test) without needing a real `AppState`/`PgPool`, since `.with_state(...)` only happens after `split_for_parts()`. Handlers are `pub(crate)` (not private) so `routes!(...)` can reference them across modules. Mounted via `utoipa_swagger_ui::SwaggerUi` at `/docs` (UI) and `/api-docs/openapi.json` (raw spec), both public and outside `/api/v1`. Non-obvious gotchas hit while wiring this up:
- A generic type argument that isn't a `syn::Type::Path` (e.g. `ApiResponse<()>`, the unit type) panics utoipa's alias-resolution code at macro-expansion time — omit `body = ...` on responses that don't need one rather than reaching for `()`.
- utoipa 5.x's generic-type aliasing (the old `#[aliases(...)]` derive attribute) moved to a separate `utoipa-config` mechanism, so just reference concrete instantiations like `GenericDeviceResponse<String>` directly instead.
- `utoipa-swagger-ui` **must** have the `vendored` feature enabled (`Cargo.toml`) — without it, its build script tries to `curl`-download the Swagger UI dist bundle from GitHub at compile time, which fails outright in the `Dockerfile`'s minimal `rust:1.90-slim-bookworm` builder (no `curl` installed) and would otherwise make every build depend on that download succeeding. `vendored` bundles the same assets as a regular crate dependency instead.
- `utoipa_axum::routes!(a, b, ...)` groups multiple handlers into one router registration and expects them all to share the same `path` — it's for "same resource, different methods" (e.g. `GET`+`POST` on `/`), not for registering unrelated paths in bulk.

**Retry:** read-only repository methods (`find_device_by_id`, `list_all_device`, `list_devices_by_owner`, `get_readings_filtered_paginated`, `find_user_by_username`, `find_user_by_id`, `list_all_users`) retry transient `sqlx::Error`s (`Io`, `PoolTimedOut`, `PoolClosed`, `WorkerCrashed`, and `Database` errors with a Postgres SQLSTATE connection-exception code, class `08`) with jittered exponential backoff, via the shared `repository::retry::read` helper (`src/repository/retry.rs`). Row-not-found, decode, constraint, and config/protocol errors are not retried — retrying those can't ever succeed. This has to live *inside* each repository method, wrapping the raw `sqlx` call, because every method's `.map_err(...)` immediately stringifies the error into `AppError::DatabaseError(String)`, discarding the `sqlx::Error` variant retry classification needs. Write methods (`insert_*`/`update_*`/`delete_*`) are deliberately excluded: retrying a lost-ack `INSERT` (e.g. `insert_reading`, called by IoT devices pushing data) risks silent duplicates, since none of these tables have an idempotency key to make that safe. `UserRepository::health_check` is also excluded even though it's a read — it backs the `/health` liveness endpoint, and retrying it would delay/mask a real outage from whatever's polling it instead of surfacing it.

## Deployment

The app is designed to run as a container behind a TLS-terminating reverse proxy or load balancer — it only ever serves plain HTTP itself and binds a plain TCP listener (`tokio::net::TcpListener` in `src/main.rs`); TLS and any non-TCP transport is the proxy's job, not the app's.

**Graceful shutdown:** `axum::serve(...)` in `src/main.rs` is wired to `.with_graceful_shutdown(shutdown_signal())`, which races SIGTERM (what container orchestrators send on redeploy/scale-down) against Ctrl+C/SIGINT (for local `cargo run`). On either signal, in-flight requests are allowed to finish before the process exits — the orchestrator's shutdown grace period needs to be long enough for the slowest in-flight request to drain.

**`Dockerfile`:** multi-stage build — `rust:1.90-slim-bookworm` builder produces a release binary, `debian:bookworm-slim` runtime stage runs it as a non-root user. The builder sets `SQLX_OFFLINE=true` and relies on the committed `.sqlx/` query cache (see `cargo sqlx prepare` above) instead of a live DB connection, since the image build has no database to check `query!`/`query_as!` calls against. **The `.sqlx/` cache must be regenerated (`cargo sqlx prepare`) and recommitted whenever a query changes** — a stale cache doesn't fail loudly, it just lets the Docker build compile against outdated query shapes.

Migrations are not run by the container on startup — `sqlx migrate run` against the target database remains a separate, explicit step before/during rollout, consistent with how migrations and seed data are already kept as distinct concerns (see Infrastructure above).

**`docker-compose.prod.yml`:** runs the app container alongside its own Postgres (distinct container names, host port `5433`, and volume `prod_pgdata` so it can coexist with the dev `docker-compose.yml` on the same host). Separate from the dev/CI compose file for the same reason seed data is kept out of migrations — explicit and opt-in, so it never affects `cargo test`/CI's `docker compose up -d`. It declares its own top-level `name: iot-hub-prod` — without that, both compose files default to the same project name (the repo directory name) and both define a service called `db`, so running one can recreate/destroy the other's `db` container even though `container_name` differs.

**CD:** `.github/workflows/ci.yml`'s `docker` job builds the `Dockerfile` and pushes to `ghcr.io/zhshih/iot-hub` (tags: commit SHA + `latest`), gated to `needs: test` and `push` events on `main` only (never on PRs — forks can't get `packages: write` anyway). No live DB is spun up for this job; the image build is already fully offline via `SQLX_OFFLINE=true` and the committed `.sqlx/` cache. `packages: write` is scoped to just this job, not the whole workflow.
