//! wasi-sql ORM layer for SQL databases.
//!
//! Provides a fluent API for building SQL queries with compile-time type safety,
//! automatic type conversions, and ``SeaQuery`` abstraction.
//!
//! # Quick Start
//!
//! ## Define an Entity
//!
//! ```ignore
//! use chrono::{DateTime, Utc};
//!
//! entity! {
//!     table = "posts",
//!     #[derive(Debug, Clone)]
//!     pub struct Post {
//!         pub id: i32,
//!         pub title: String,
//!         pub content: String,
//!         pub published: bool,
//!         pub created_at: DateTime<Utc>,
//!     }
//! }
//! ```
//!
//! ## CRUD Operations
//!
//! ```ignore
//! use crate::orm::{SelectBuilder, InsertBuilder, UpdateBuilder, DeleteBuilder, Filter};
//!
//! // Select with filter
//! let posts = SelectBuilder::<Post>::new()
//!     .where(Filter::eq("published", true))
//!     .where(Filter::gt("created_at", Utc::now() - Duration::days(7)))
//!     .order_by_desc(None, "created_at")
//!     .limit(10)
//!     .fetch(provider, "db").await?;
//!
//! // Insert
//! InsertBuilder::<Post>::new()
//!     .set("title", "Hello World")
//!     .set("content", "My first post")
//!     .set("published", true)
//!     .build()?;
//!
//! // Or insert from an entity
//! let post = Post {
//!     id: 1,
//!     title: "Hello".to_string(),
//!     content: "World".to_string(),
//!     published: true,
//!     created_at: Utc::now(),
//! };
//! InsertBuilder::<Post>::from_entity(&post).build()?;
//!
//! // Update
//! UpdateBuilder::<Post>::new()
//!     .set("published", true)
//!     .where(Filter::eq("id", 42))
//!     .build()?;
//!
//! // Delete
//! DeleteBuilder::<Post>::new()
//!     .where(Filter::eq("id", 42))
//!     .build()?;
//! ```
//!
//! ## Joins
//!
//! ```ignore
//! use crate::orm::Join;
//!
//! // Entity with default joins
//! entity! {
//!     table = "posts",
//!     joins = [
//!         Join::left("users", Filter::col_eq("posts", "author_id", "users", "id")),
//!     ],
//!     #[derive(Debug, Clone)]
//!     pub struct PostWithAuthor {
//!         pub id: i32,
//!         pub title: String,
//!         pub author_name: String,  // From joined users table
//!     }
//! }
//!
//! // Joins happen automatically
//! let posts = SelectBuilder::<PostWithAuthor>::new()
//!     .fetch(provider, "db").await?;
//!
//! // Or add ad-hoc joins
//! let posts = SelectBuilder::<Post>::new()
//!     .join(Join::left("users", Filter::col_eq("posts", "author_id", "users", "id")))
//!     .fetch(provider, "db").await?;
//! ```
//!
//! ## Filtering
//!
//! ```ignore
//! // Basic comparisons
//! Filter::eq("status", "active")
//! Filter::gt("views", 1000)
//! Filter::like("title", "%rust%")
//! Filter::in("id", vec![1, 2, 3])
//!
//! // Logical combinators
//! Filter::and(vec![
//!     Filter::eq("published", true),
//!     Filter::gt("views", 100),
//! ])
//!
//! Filter::or(vec![
//!     Filter::eq("featured", true),
//!     Filter::gt("views", 5000),
//! ])
//!
//! // Table-qualified (for joins)
//! Filter::table_eq("posts", "published", true)
//! Filter::col_eq("posts", "author_id", "users", "id")
//! ```
//!
//! ## Upserts
//!
//! ```ignore
//! // Insert or update on conflict
//! InsertBuilder::<User>::new()
//!     .set("email", "test@example.com")
//!     .set("name", "John Doe")
//!     .on_conflict("email")
//!     .do_update(&["name"])
//!     .build()?;
//! ```
//!
//! ## Custom Types
//!
//! ```ignore
//! impl FetchValue for UserId {
//!     fn fetch(row: &Row, col: &str) -> anyhow::Result<Self> {
//!         let id: String = FetchValue::fetch(row, col)?;
//!         Ok(UserId(id))
//!     }
//! }
//! ```
//!
//! For comprehensive examples and advanced usage, see [`orm_usage.md`](orm/orm_usage.md).

mod delete;
mod entity;
mod filter;
mod insert;
mod join;
mod query;
mod select;
mod update;

use anyhow::{Result, anyhow};
pub use delete::DeleteBuilder;
pub use entity::{Entity, EntityValues, FetchValue};
pub use filter::Filter;
use futures::FutureExt;
use futures::future::BoxFuture;
pub use insert::InsertBuilder;
pub use join::Join;
#[doc(hidden)]
pub use sea_query::Value as SeaQueryValue;
pub use select::SelectBuilder;
pub use update::UpdateBuilder;

use crate::readwrite;
use crate::types::{Connection, DataType, Row, Statement};

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

/// Trait for types that provide ORM database access.
///
/// Implement this trait to enable ORM operations. Default implementations
/// use the WASI SQL bindings to execute queries.
pub trait OrmDataStore: Send + Sync {
    fn query(
        &self, pool_name: String, query: String, params: Vec<DataType>,
    ) -> FutureResult<Vec<Row>> {
        async {
            let cnn = Connection::open(pool_name)
                .await
                .map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

            let stmt = Statement::prepare(query, params)
                .await
                .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

            let res =
                readwrite::query(&cnn, &stmt).await.map_err(|e| anyhow!("query failed: {e:?}"))?;

            Ok(res)
        }
        .boxed()
    }

    fn exec(&self, pool_name: String, query: String, params: Vec<DataType>) -> FutureResult<u32> {
        async {
            let cnn = Connection::open(pool_name)
                .await
                .map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

            let stmt = Statement::prepare(query, params)
                .await
                .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

            let res =
                readwrite::exec(&cnn, &stmt).await.map_err(|e| anyhow!("exec failed: {e:?}"))?;

            Ok(res)
        }
        .boxed()
    }
}
