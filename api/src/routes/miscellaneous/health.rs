#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check successful")
    )
)]
pub async fn health() -> &'static str {
    "
    {
        \"status\": \"ok\"
    }
    "
}
