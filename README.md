# url-shortener

Rust project to implement the Chapter 8 URL shortener architecture from *System Design Interview* with:

- `axum` HTTP API
- Redis for `<short_code, long_url>` cache
- Postgres as the source of truth
- base62 short-code generation from unique IDs
- redirect flow backed by cache-first lookup
- file-based local configuration in `config.toml`
- Docker Compose services for Postgres and Redis
- real integration tests against live Postgres and Redis instances

## Architecture Goal

Replicate the chapter 8 shape as closely as practical:

- `POST /api/v1/data/shorten`
- `GET /:short_code`
- database stores canonical mappings
- Redis accelerates redirect reads
- short codes are created from a unique numeric ID plus base62 conversion

## Database Choice

Use a relational database first, not NoSQL.

Why:

- the data model is simple and strongly structured
- uniqueness constraints matter for `short_code`
- repeated `long_url` values are deduplicated in the service layer via exact-match lookup
- chapter 8 itself models the system with a relational table
- Redis already handles the high-read redirect path, so the primary DB does not need to be a document store

Recommended source of truth:

- `Postgres`

Recommended cache:

- `Redis`

## Core Flows

Shortening flow:

1. accept `long_url`
2. check whether the URL already exists in the database
3. if yes, return the existing short code
4. if no, generate a unique numeric ID
5. convert the ID to base62
6. insert the row into Postgres
7. optionally warm Redis

Redirect flow:

1. receive `short_code`
2. check Redis
3. if cache hit, redirect immediately
4. if cache miss, fetch from Postgres
5. populate Redis
6. return redirect

## Local Setup

1. Start infrastructure:
   - `docker compose up -d`
2. Run the app:
   - `cargo run`
3. Run the full test suite:
   - `cargo test`

All local settings live in [config.toml](/Users/bilesimo/Development/url-shortener/config.toml). Environment overrides are still supported using the `APP_` prefix with `__` separators, for example `APP_APPLICATION__PORT=4000`.

## Project Layout

- `src/lib.rs`: library entrypoint for the app and integration tests
- `src/main.rs`: binary entrypoint
- `src/startup.rs`: application wiring and server bootstrap
- `src/configuration.rs`: config loading from `config.toml`
- `src/http/`: handlers, routing, request/response DTOs
- `src/domain/`: core models and business logic
- `src/storage/`: Postgres and Redis repositories
- `src/id/`: unique ID generation and base62 conversion
- `src/error.rs`: shared error types
- `tests/api/`: integration tests using real Postgres and Redis instances
- `docker-compose.yml`: local Postgres and Redis stack
- `docs/implementation-plan.md`: phased implementation plan

See [docs/implementation-plan.md](/Users/bilesimo/Development/url-shortener/docs/implementation-plan.md) for the detailed build plan.
