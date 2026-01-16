#![allow(dead_code)]

use sea_query::{Alias, ColumnRef, Expr, ExprTrait, IntoIden, SimpleExpr, Value};

use crate::orm::select::table_column;

/// Filter represents database predicates without exposing ``SeaQuery`` types to guest code.
///
/// Values are stored internally as ``sea_query::Value`` but guest code never imports ``SeaQuery``.
/// Instead, guests use natural Rust types (i32, String, ``DateTime<Utc>``) which convert via From.
///
/// For filters with optional table parameter: None uses the entity's default table,
/// ``Some("table_name")`` uses the specified table (useful for joins).
#[derive(Debug, Clone)]
pub enum Filter {
    // Using static lifetimes since table and column names are compile time constants
    /// [table.]column = value
    Eq(Option<&'static str>, &'static str, Value),
    /// [table.]column != value
    Ne(Option<&'static str>, &'static str, Value),
    /// [table.]column > value
    Gt(Option<&'static str>, &'static str, Value),
    /// [table.]column >= value
    Gte(Option<&'static str>, &'static str, Value),
    /// [table.]column < value
    Lt(Option<&'static str>, &'static str, Value),
    /// [table.]column <= value
    Lte(Option<&'static str>, &'static str, Value),
    /// [table.]column IN (values)
    In(Option<&'static str>, &'static str, Vec<Value>),
    /// [table.]column NOT IN (values)
    NotIn(Option<&'static str>, &'static str, Vec<Value>),
    /// [table.]column IS NULL
    IsNull(Option<&'static str>, &'static str),
    /// [table.]column IS NOT NULL
    IsNotNull(Option<&'static str>, &'static str),
    /// [table.]column LIKE pattern
    Like(Option<&'static str>, &'static str, String),
    /// [table.]column NOT LIKE pattern
    NotLike(Option<&'static str>, &'static str, String),
    /// [table.]column BETWEEN low AND high
    Between(Option<&'static str>, &'static str, Value, Value),
    /// [table.]column NOT BETWEEN low AND high
    NotBetween(Option<&'static str>, &'static str, Value, Value),
    /// [table.]column = ANY(values)
    Any(Option<&'static str>, &'static str, Vec<Value>),
    /// Column-to-column comparison: table1.col1 = table2.col2
    ColEq(&'static str, &'static str, &'static str, &'static str),
    /// Column-to-column comparison: table1.col1 != table2.col2
    ColNe(&'static str, &'static str, &'static str, &'static str),
    /// Column-to-column comparison: table1.col1 > table2.col2
    ColGt(&'static str, &'static str, &'static str, &'static str),
    /// Column-to-column comparison: table1.col1 >= table2.col2
    ColGte(&'static str, &'static str, &'static str, &'static str),
    /// Column-to-column comparison: table1.col1 < table2.col2
    ColLt(&'static str, &'static str, &'static str, &'static str),
    /// Column-to-column comparison: table1.col1 <= table2.col2
    ColLte(&'static str, &'static str, &'static str, &'static str),
    /// Logical AND of multiple filters
    And(Vec<Self>),
    /// Logical OR of multiple filters
    Or(Vec<Self>),
    /// Logical NOT of a filter
    Not(Box<Self>),
}

impl Filter {
    /// Helper to resolve a column reference with optional table qualifier
    fn resolve_column(
        tbl: Option<&'static str>, col: &'static str, default_table: &'static str,
    ) -> SimpleExpr {
        Expr::col(table_column(tbl.unwrap_or(default_table), col)).into()
    }

    /// Convert Filter to ``SeaQuery`` ``SimpleExpr`` using the specified table name.
    #[must_use]
    pub fn into_expr(self, default_table: &'static str) -> SimpleExpr {
        match self {
            Self::Eq(tbl, col, val) => Self::resolve_column(tbl, col, default_table).eq(val),
            Self::Ne(tbl, col, val) => Self::resolve_column(tbl, col, default_table).ne(val),
            Self::Gt(tbl, col, val) => Self::resolve_column(tbl, col, default_table).gt(val),
            Self::Gte(tbl, col, val) => Self::resolve_column(tbl, col, default_table).gte(val),
            Self::Lt(tbl, col, val) => Self::resolve_column(tbl, col, default_table).lt(val),
            Self::Lte(tbl, col, val) => Self::resolve_column(tbl, col, default_table).lte(val),
            Self::In(tbl, col, vals) => Self::resolve_column(tbl, col, default_table).is_in(vals),
            Self::NotIn(tbl, col, vals) => {
                Self::resolve_column(tbl, col, default_table).is_not_in(vals)
            }
            Self::IsNull(tbl, col) => Self::resolve_column(tbl, col, default_table).is_null(),
            Self::IsNotNull(tbl, col) => {
                Self::resolve_column(tbl, col, default_table).is_not_null()
            }
            Self::Like(tbl, col, pattern) => {
                Self::resolve_column(tbl, col, default_table).like(pattern)
            }
            Self::NotLike(tbl, col, pattern) => {
                Self::resolve_column(tbl, col, default_table).not_like(pattern)
            }
            Self::Between(tbl, col, low, high) => {
                Self::resolve_column(tbl, col, default_table).between(low, high)
            }
            Self::NotBetween(tbl, col, low, high) => {
                Self::resolve_column(tbl, col, default_table).not_between(low, high)
            }
            Self::Any(tbl, col, vals) => {
                // Note: SeaQuery's ANY requires subquery; this is simplified for direct value array
                Self::resolve_column(tbl, col, default_table).is_in(vals)
            }
            Self::ColEq(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).eq(Expr::col(right))
            }
            Self::ColNe(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).ne(Expr::col(right))
            }
            Self::ColGt(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).gt(Expr::col(right))
            }
            Self::ColGte(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).gte(Expr::col(right))
            }
            Self::ColLt(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).lt(Expr::col(right))
            }
            Self::ColLte(tbl1, col1, tbl2, col2) => {
                let left = table_column(tbl1, col1);
                let right = table_column(tbl2, col2);
                Expr::col(left).lte(Expr::col(right))
            }
            Self::And(filters) => {
                let mut exprs = filters.into_iter().map(|f| f.into_expr(default_table));
                exprs.next().map_or_else(
                    || Expr::value(true), // no filters, so all conditions satisfied, hence `true`
                    |first| exprs.fold(first, sea_query::SimpleExpr::and),
                )
            }
            Self::Or(filters) => {
                let mut exprs = filters.into_iter().map(|f| f.into_expr(default_table));
                exprs.next().map_or_else(
                    || Expr::value(false), // no filters, so 0 conditions satisfied, hence `false`
                    |first| exprs.fold(first, sea_query::SimpleExpr::or),
                )
            }
            Self::Not(filter) => Expr::expr(filter.into_expr(default_table)).not(),
        }
    }

    // Convenience constructors for common single-table queries

    #[must_use]
    pub fn eq(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Eq(None, col, val.into())
    }

    #[must_use]
    pub fn ne(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Ne(None, col, val.into())
    }

    #[must_use]
    pub fn gt(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Gt(None, col, val.into())
    }

    #[must_use]
    pub fn gte(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Gte(None, col, val.into())
    }

    #[must_use]
    pub fn lt(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Lt(None, col, val.into())
    }

    #[must_use]
    pub fn lte(col: &'static str, val: impl Into<Value>) -> Self {
        Self::Lte(None, col, val.into())
    }

    #[must_use]
    pub fn r#in(col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>) -> Self {
        Self::In(None, col, vals.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn not_in(col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>) -> Self {
        Self::NotIn(None, col, vals.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub const fn is_null(col: &'static str) -> Self {
        Self::IsNull(None, col)
    }

    #[must_use]
    pub const fn is_not_null(col: &'static str) -> Self {
        Self::IsNotNull(None, col)
    }

    #[must_use]
    pub const fn like(col: &'static str, pattern: String) -> Self {
        Self::Like(None, col, pattern)
    }

    #[must_use]
    pub const fn not_like(col: &'static str, pattern: String) -> Self {
        Self::NotLike(None, col, pattern)
    }

    #[must_use]
    pub fn between(col: &'static str, low: impl Into<Value>, high: impl Into<Value>) -> Self {
        Self::Between(None, col, low.into(), high.into())
    }

    #[must_use]
    pub fn not_between(col: &'static str, low: impl Into<Value>, high: impl Into<Value>) -> Self {
        Self::NotBetween(None, col, low.into(), high.into())
    }

    #[must_use]
    pub fn any(col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>) -> Self {
        Self::Any(None, col, vals.into_iter().map(Into::into).collect())
    }

    // Table-qualified variants for joined queries

    #[must_use]
    pub fn table_eq(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Eq(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_ne(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Ne(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_gt(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Gt(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_gte(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Gte(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_lt(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Lt(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_lte(table: &'static str, col: &'static str, val: impl Into<Value>) -> Self {
        Self::Lte(Some(table), col, val.into())
    }

    #[must_use]
    pub fn table_in(
        table: &'static str, col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>,
    ) -> Self {
        Self::In(Some(table), col, vals.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn table_not_in(
        table: &'static str, col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>,
    ) -> Self {
        Self::NotIn(Some(table), col, vals.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub const fn table_is_null(table: &'static str, col: &'static str) -> Self {
        Self::IsNull(Some(table), col)
    }

    #[must_use]
    pub const fn table_is_not_null(table: &'static str, col: &'static str) -> Self {
        Self::IsNotNull(Some(table), col)
    }

    #[must_use]
    pub const fn table_like(table: &'static str, col: &'static str, pattern: String) -> Self {
        Self::Like(Some(table), col, pattern)
    }

    #[must_use]
    pub const fn table_not_like(table: &'static str, col: &'static str, pattern: String) -> Self {
        Self::NotLike(Some(table), col, pattern)
    }

    #[must_use]
    pub fn table_between(
        table: &'static str, col: &'static str, low: impl Into<Value>, high: impl Into<Value>,
    ) -> Self {
        Self::Between(Some(table), col, low.into(), high.into())
    }

    #[must_use]
    pub fn table_not_between(
        table: &'static str, col: &'static str, low: impl Into<Value>, high: impl Into<Value>,
    ) -> Self {
        Self::NotBetween(Some(table), col, low.into(), high.into())
    }

    #[must_use]
    pub fn table_any(
        table: &'static str, col: &'static str, vals: impl IntoIterator<Item = impl Into<Value>>,
    ) -> Self {
        Self::Any(Some(table), col, vals.into_iter().map(Into::into).collect())
    }

    /// Compare two columns for equality.
    /// Table names are required since we're comparing columns from different tables.
    #[must_use]
    pub const fn col_eq(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColEq(table1, col1, table2, col2)
    }

    #[must_use]
    pub const fn col_ne(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColNe(table1, col1, table2, col2)
    }

    #[must_use]
    pub const fn col_gt(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColGt(table1, col1, table2, col2)
    }

    #[must_use]
    pub const fn col_gte(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColGte(table1, col1, table2, col2)
    }

    #[must_use]
    pub const fn col_lt(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColLt(table1, col1, table2, col2)
    }

    #[must_use]
    pub const fn col_lte(
        table1: &'static str, col1: &'static str, table2: &'static str, col2: &'static str,
    ) -> Self {
        Self::ColLte(table1, col1, table2, col2)
    }
}

/// Helper for table.column references with optional table specification
pub fn col_ref(table: Option<&'static str>, column: &'static str) -> ColumnRef {
    table.map_or_else(
        || ColumnRef::Column(Alias::new(column).into_iden()),
        |t| ColumnRef::TableColumn(Alias::new(t).into_iden(), Alias::new(column).into_iden()),
    )
}
