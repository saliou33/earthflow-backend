use axum::{
    routing::get,
    Router,
    Json,
};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Use modules from the library
use backend::{routes, nodes, nodes::NodeRegistry, AppState};

use std::sync::Arc;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "earthflow_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            if std::env::var("RUST_ENV").unwrap_or_default() == "production" {
                anyhow::bail!("DATABASE_URL is not defined! Production requires this environment variable.");
            } else {
                tracing::warn!("DATABASE_URL is not defined. Falling back to localhost for development.");
                "postgres://postgres:postgres@localhost:5432/earthflow".to_string()
            }
        }
    };
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
        
    // Insert mock user for POC testing
    let mock_user_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, display_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (id) DO NOTHING
        "#,
        mock_user_id,
        "test@earthflow.local",
        "Test User"
    )
    .execute(&pool)
    .await?;

    seed_demo_data(&pool, mock_user_id).await?;

    let mut registry = NodeRegistry::new();
    registry.register(Box::new(nodes::core::VariableNode));
    registry.register(Box::new(nodes::core::ExpressionNode::new()));
    registry.register(Box::new(nodes::io::AssetInputNode));
    registry.register(Box::new(nodes::io::DrawNode));
    
    // Register Vector nodes
    registry.register(Box::new(nodes::vector_nodes::BufferNode));
    registry.register(Box::new(nodes::vector_nodes::CentroidNode));
    registry.register(Box::new(nodes::vector_nodes::ConvexHullNode));
    registry.register(Box::new(nodes::vector_nodes::SimplifyNode));
    registry.register(Box::new(nodes::vector_nodes::IntersectionNode));
    
    // Register Raster nodes
    registry.register(Box::new(nodes::raster_nodes::RasterClipNode));
    registry.register(Box::new(nodes::raster_nodes::RasterStatisticsNode));
    registry.register(Box::new(nodes::raster_nodes::HillshadeNode));
    registry.register(Box::new(nodes::raster_nodes::SlopeNode));
    registry.register(Box::new(nodes::raster_nodes::AspectNode));
    registry.register(Box::new(nodes::raster_nodes::BandMathNode));
    
    // Register Analysis nodes
    registry.register(Box::new(nodes::analysis_nodes::KernelDensityNode));
    registry.register(Box::new(nodes::analysis_nodes::ViewshedNode));
    
    // Register Table nodes
    registry.register(Box::new(nodes::table_nodes::TableJoinNode));
    registry.register(Box::new(nodes::table_nodes::TableFilterNode));
    
    // Register Style nodes
    registry.register(Box::new(nodes::style_nodes::SimpleFillNode));
    registry.register(Box::new(nodes::style_nodes::ChoroplethNode));
    registry.register(Box::new(nodes::postgres_nodes::SourcePostgresNode));
    
    let minio_endpoint = std::env::var("MINIO_ENDPOINT").unwrap_or_else(|_| {
        tracing::warn!("MINIO_ENDPOINT is not defined. Falling back to localhost for development.");
        "http://localhost:9000".to_string()
    });
    let minio_access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "admin".to_string());
    let minio_secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "password".to_string());
    
    let s3_config = aws_sdk_s3::config::Builder::new()
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            minio_access_key,
            minio_secret_key,
            None,
            None,
            "Static",
        ))
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .endpoint_url(minio_endpoint)
        .force_path_style(true)
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
    
    // Ensure default bucket exists
    let bucket_name = std::env::var("MINIO_BUCKET_NAME").unwrap_or_else(|_| "earthflow".to_string());
    let _ = s3_client.create_bucket().bucket(&bucket_name).send().await;

    let state = AppState {
        pool,
        registry: Arc::new(registry),
        s3_client,
    };

    // Tower layers are applied bottom-to-top. But `axum::Router::layer` applies it to all routes defined BEFORE it.
    let app = Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/v1", routes::v1_routes())
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    
    tracing::info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn seed_demo_data(pool: &sqlx::PgPool, owner_id: uuid::Uuid) -> anyhow::Result<()> {
    let demo_workflow_id = uuid::Uuid::parse_str("d0000000-0000-0000-0000-000000000000").unwrap();
    
    let existing = sqlx::query!("SELECT id FROM workflows WHERE id = $1", demo_workflow_id)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        tracing::info!("Demo workflow already exists, skipping seed.");
        return Ok(());
    }

    let graph = serde_json::json!({
        "nodes": [
            {
                "id": "node-v1",
                "type": "variable",
                "position": { "x": 100, "y": 100 },
                "data": { "value": "10.0", "label": "a", "inputType": "float" }
            },
            {
                "id": "node-v2",
                "type": "variable",
                "position": { "x": 100, "y": 250 },
                "data": { "value": "5.0", "label": "b", "inputType": "float" }
            },
            {
                "id": "node-exp",
                "type": "expression",
                "position": { "x": 400, "y": 175 },
                "data": { "expression": "a + b" }
            }
        ],
        "edges": [
            {
                "id": "edge-1",
                "source": "node-v1",
                "target": "node-exp",
                "sourceHandle": "value",
                "targetHandle": "a"
            },
            {
                "id": "edge-2",
                "source": "node-v2",
                "target": "node-exp",
                "sourceHandle": "value",
                "targetHandle": "b"
            }
        ]
    });

    sqlx::query!(
        r#"
        INSERT INTO workflows (id, owner_id, name, description, graph)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        demo_workflow_id,
        owner_id,
        "Math Expression Demo",
        "A demo showing variables being combined in a Rhai expression.",
        graph
    )
    .execute(pool)
    .await?;

    tracing::info!("Demo workflow seeded successfully.");
    Ok(())
}
