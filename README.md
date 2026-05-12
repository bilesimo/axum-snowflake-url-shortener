# url-shortener

Rust project to implement the Chapter 8 URL shortener architecture from *System Design Interview* with:

- `axum` HTTP API
- Redis for `<short_code, long_url>` cache
- Postgres as the source of truth
- base62 short-code generation from unique IDs
- redirect flow backed by cache-first lookup

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
- repeated `long_url` values can be deduplicated by lookup before insert
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

## Project Layout

- `src/main.rs`: application bootstrap
- `src/http/`: handlers, routing, request/response DTOs
- `src/domain/`: core models and business logic
- `src/storage/`: Postgres and Redis repositories
- `src/id/`: unique ID generation and base62 conversion
- `src/config/`: environment configuration
- `src/error.rs`: shared error types
- `docs/implementation-plan.md`: phased implementation plan

See [docs/implementation-plan.md](/Users/bilesimo/Development/url-shortener/docs/implementation-plan.md) for the detailed build plan.
