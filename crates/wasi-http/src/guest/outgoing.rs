use std::any::Any;
use std::error::Error;

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;
use wasip3::http::handler;
use wasip3::http_compat::{http_from_wasi_response, http_into_wasi_request};

use crate::DEFAULT_FORBIDDEN_HEADERS;
pub use crate::guest::cache::{Cache, CacheOptions};

/// Send an HTTP request using the WASI HTTP proxy handler.
///
/// # Errors
///
/// Returns an error if the request could not be sent.
pub async fn handle<T>(request: http::Request<T>) -> Result<http::Response<Bytes>>
where
    T: Body + Any,
    T::Data: Into<Vec<u8>>,
    T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
{
    let maybe_cache = Cache::maybe_from(&request)?;

    // check cache when indicated by request
    if let Some(cache) = maybe_cache.as_ref()
        && let Some(hit) = cache.get().await?
    {
        tracing::debug!("cache hit");
        return Ok(hit);
    }

    tracing::debug!("forwarding request to proxy: {:?}", request.headers());

    let wasi_req = http_into_wasi_request(request).context("Issue converting request")?;
    let wasi_resp = handler::handle(wasi_req).await.context("Issue calling proxy")?;
    let http_resp = http_from_wasi_response(wasi_resp).context("Issue converting response")?;

    // convert body
    let (parts, body) = http_resp.into_parts();
    let collected = body.collect().await.context("failed to collect body")?;
    let bytes = collected.to_bytes();
    let mut response = http::Response::from_parts(parts, bytes);
    // filter out bad headers not allowed by wasi-http
    let headers = response.headers_mut();
    for forbidden in &DEFAULT_FORBIDDEN_HEADERS {
        headers.remove(forbidden);
    }

    // add ETag header and cache response when indicated by request
    if let Some(cache) = maybe_cache {
        headers.insert(http::header::ETAG, http::HeaderValue::from_str(&cache.etag())?);
        cache.put(&response).await?;
        tracing::debug!("response cached");
    }

    tracing::debug!("proxy response: {response:?}");

    Ok(response)
}
