use backend::nodes::{NodeRegistry, NodeContext};
use backend::engine::executor::WorkflowExecutor;
use backend::nodes::{io::AssetInputNode, io::DrawNode, core::{VariableNode, ExpressionNode}};
use std::sync::Arc;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TestHarness {
    pub registry: Arc<NodeRegistry>,
    pub ctx: NodeContext,
    pub executor: WorkflowExecutor<'static>,
}

impl TestHarness {
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();
        
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&db_url).await.expect("Failed to connect to DB");
        
        let mut registry = NodeRegistry::new();
        registry.register(Box::new(AssetInputNode));
        registry.register(Box::new(DrawNode));
        registry.register(Box::new(VariableNode));
        registry.register(Box::new(ExpressionNode::new()));
        // We'll add more nodes to registry here as needed, or use a helper that registers ALL
        
        // Register all currently implemented nodes for "brute force" testing
        registry.register(Box::new(backend::nodes::vector_nodes::BufferNode));
        registry.register(Box::new(backend::nodes::vector_nodes::CentroidNode));
        registry.register(Box::new(backend::nodes::vector_nodes::ConvexHullNode));
        registry.register(Box::new(backend::nodes::vector_nodes::SimplifyNode));
        registry.register(Box::new(backend::nodes::vector_nodes::IntersectionNode));
        
        registry.register(Box::new(backend::nodes::raster_nodes::RasterClipNode));
        registry.register(Box::new(backend::nodes::raster_nodes::RasterStatisticsNode));
        registry.register(Box::new(backend::nodes::raster_nodes::HillshadeNode));
        
        registry.register(Box::new(backend::nodes::analysis_nodes::KernelDensityNode));
        registry.register(Box::new(backend::nodes::analysis_nodes::ViewshedNode));
        
        registry.register(Box::new(backend::nodes::table_nodes::TableJoinNode));
        registry.register(Box::new(backend::nodes::table_nodes::TableFilterNode));
        
        registry.register(Box::new(backend::nodes::style_nodes::SimpleFillNode));
        registry.register(Box::new(backend::nodes::style_nodes::ChoroplethNode));
        registry.register(Box::new(backend::nodes::postgres_nodes::SourcePostgresNode));

        let registry = Arc::new(registry);
        
        let minio_endpoint = std::env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
        let minio_access_key = std::env::var("MINIO_ROOT_USER").unwrap_or_else(|_| "admin".to_string());
        let minio_secret_key = std::env::var("MINIO_ROOT_PASSWORD").unwrap_or_else(|_| "password".to_string());
        
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
        let s3_client = S3Client::from_conf(s3_config);
        
        let ctx = NodeContext {
            pool,
            s3_client,
        };
        
        // Leaking registry for 'static lifetime to simplify executor setup in tests
        let registry_ref = Box::leak(Box::new(registry.clone()));
        let executor = WorkflowExecutor::new(registry_ref, ctx.clone());
        
        Self {
            registry,
            ctx,
            executor,
        }
    }

    pub async fn get_mock_user_id(&self) -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
    }

    pub async fn create_test_asset(&self, name: &str, asset_type: &str, content: &[u8]) -> backend::models::asset::Asset {
        let user_id = self.get_mock_user_id().await;
        let asset_id = Uuid::new_v4();
        let key = format!("{}/{}", user_id, asset_id);
        
        self.ctx.s3_client.put_object()
            .bucket("earthflow")
            .key(&key)
            .body(content.to_owned().into())
            .send()
            .await
            .expect("Failed to upload test asset to S3");

        sqlx::query_as!(
            backend::models::asset::Asset,
            r#"
            INSERT INTO assets (id, owner_id, name, asset_type, storage_uri)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, owner_id, name, description, asset_type as "asset_type: _", storage_uri, connection_id, metadata, created_at, updated_at
            "#,
            asset_id,
            user_id,
            name,
            asset_type as _,
            format!("s3://earthflow/{}", key)
        )
        .fetch_one(&self.ctx.pool)
        .await
        .expect("Failed to insert test asset into DB")
    }

    pub async fn download_test_raster(&self) -> (String, Vec<u8>) {
        let url = "https://raw.githubusercontent.com/rasterio/rasterio/master/tests/data/RGB.byte.tif";
        let client = reqwest::Client::new();
        let resp = client.get(url).send().await.expect("Failed to download test raster");
        let bytes = resp.bytes().await.expect("Failed to get raster bytes").to_vec();
        ("RGB.byte.tif".to_string(), bytes)
    }
}
