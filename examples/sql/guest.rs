//! # SQL Wasm Guest (Default Backend)
//!
//! This module demonstrates the WASI SQL interface with the default backend.
//! It shows how to perform database operations that work with any SQL-compatible
//! database configured by the host.
//!
//! ## Operations Demonstrated
//!
//! - Opening database connections by name
//! - Preparing parameterized SQL statements
//! - Executing SELECT queries
//! - Executing INSERT/UPDATE/DELETE commands
//! - Converting results to JSON
//!
//! ## Security
//!
//! Always use parameterized queries (`$1`, `$2`, etc.) to prevent SQL injection.
//! Never concatenate user input into SQL strings.
//!
//! ## Backend Agnostic
//!
//! This guest code works with any WASI SQL backend:
//! - PostgreSQL (sql-postgres example)
//! - Azure SQL
//! - Any SQL-compatible database

#![cfg(target_arch = "wasm32")]

use anyhow::{Result, anyhow};
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use chrono::Utc;
use qwasr_sdk::{HttpResult, OrmDataStore};
use qwasr_wasi_sql::orm::{InsertBuilder, SelectBuilder};
use qwasr_wasi_sql::types::{Connection, Statement};
use qwasr_wasi_sql::{entity, readwrite};
use serde::Serialize;
use serde_json::{Value, json};
use tracing::Level;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Routes HTTP requests to database operations.
    #[qwasr_wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::debug!("received request: {:?}", request);
        let router = Router::new().route("/", get(query)).route("/", post(insert));
        qwasr_wasi_http::serve(router, request).await
    }
}

/// Queries all rows from the sample table.
#[axum::debug_handler]
#[qwasr_wasi_otel::instrument]
async fn query() -> HttpResult<Json<Value>> {
    tracing::info!("query database");
    ensure_schema().await?;

    let feeds = SelectBuilder::<Feed>::new()
        .order_by_desc(None, "feed_id")
        .limit(100)
        .fetch(&Provider, "db")
        .await
        .map_err(|e| anyhow!("failed to fetch feeds: {e:?}"))?;

    Ok(Json(json!(feeds)))
}

/// Inserts a new row into the sample table.
#[axum::debug_handler]
#[qwasr_wasi_otel::instrument]
async fn insert(_body: Bytes) -> HttpResult<Json<Value>> {
    tracing::info!("insert data");
    ensure_schema().await?;

    // Get current max feed_id
    let feeds = SelectBuilder::<Feed>::new()
        .order_by_desc(None, "feed_id")
        .limit(1)
        .fetch(&Provider, "db")
        .await
        .map_err(|e| anyhow!("failed to fetch max feed_id: {e:?}"))?;

    let next_id = feeds.first().map(|f| f.feed_id + 1).unwrap_or(1);

    let feed = Feed {
        feed_id: next_id,
        agency_id: "test1".to_string(),
        agency_name: "name1".to_string(),
        agency_url: Some("url1".to_string()),
        agency_timezone: Some("NZL".to_string()),
        created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let query = InsertBuilder::<Feed>::from_entity(&feed)
        .build()
        .map_err(|e| anyhow!("failed to build insert query: {e:?}"))?;

    let rows_affected = Provider
        .exec("db".to_string(), query.sql, query.params)
        .await
        .map_err(|e| anyhow!("failed to insert: {e:?}"))?;

    Ok(Json(json!({
        "message": "inserted",
        "feed_id": next_id,
        "rows_affected": rows_affected
    })))
}

async fn ensure_schema() -> Result<()> {
    let pool = Connection::open("db".to_string())
        .await
        .map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

    let create = r"
            CREATE TABLE IF NOT EXISTS feed (
                feed_id INTEGER PRIMARY KEY,
                agency_id TEXT,
                agency_name TEXT,
                agency_url TEXT,
                agency_timezone TEXT,
                created_at TEXT
            );";

    let stmt = Statement::prepare(create.to_string(), vec![])
        .await
        .map_err(|e| anyhow!("failed to create schema: {e:?}"))?;

    readwrite::exec(&pool, &stmt).await.map_err(|e| anyhow!("table creation failed: {e:?}"))?;
    tracing::debug!("Schema initialized!");
    Ok(())
}

entity!(
    table = "feed",
    #[derive(Debug, Clone, Serialize)]
    pub struct Feed {
        pub feed_id: i64,
        pub agency_id: String,
        pub agency_name: String,
        pub agency_url: Option<String>,
        pub agency_timezone: Option<String>,
        pub created_at: String,
    }
);

struct Provider;

impl OrmDataStore for Provider {}
