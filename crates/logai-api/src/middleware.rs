use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn require_api_key(
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let expected_key = std::env::var("LOGAI_API_KEY").ok();

    let Some(expected) = expected_key else {
        return Ok(next.run(request).await);
    };

    if expected.is_empty() {
        return Ok(next.run(request).await);
    }

    let provided = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match provided {
        Some(key) if key == expected => Ok(next.run(request).await),
        Some(_) => Err((StatusCode::UNAUTHORIZED, "Invalid API key")),
        None => Err((StatusCode::UNAUTHORIZED, "Missing X-API-Key header")),
    }
}
