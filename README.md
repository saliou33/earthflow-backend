# EarthFlow Backend

Fast and scalable API service for the EarthFlow geospatial workflow editor.

## Tech Stack

- **Runtime**: Rust
- **Web Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database**: PostgreSQL with [SQLx](https://github.com/launchbadge/sqlx)
- **Networking**: Tokio

## Implemented API Endpoints (v1)

### Workflows
- `GET /api/v1/workflows` - List all workflows
- `POST /api/v1/workflows` - Create a new workflow
- `GET /api/v1/workflows/:id` - Fetch a specific workflow
- `PUT /api/v1/workflows/:id` - Update workflow name and graph state (nodes/edges)

### Health
- `GET /api/health` - Service health check

## Setup

1. Ensure you have Rust and Cargo installed.
2. Set up the database: `docker-compose up -d`.
3. Run migrations.
4. Start the server: `cargo run`.
