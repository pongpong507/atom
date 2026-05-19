mod cli;
mod db;
mod error;
mod models;
mod routes;
mod services;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::Key;
use sha2::Digest;
use axum::extract::FromRef;
use services::llm_client::LlmClient;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub llm: Arc<LlmClient>,
    pub cookie_key: Key,
}

// SignedCookieJar needs to extract the Key from AppState
impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

/// Extractor: validates session cookie and returns the current User.
pub struct AuthUser(pub models::User);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum_extra::extract::cookie::SignedCookieJar;
        let state = AppState::from_ref(state);
        let jar = SignedCookieJar::from_headers(&parts.headers, state.cookie_key.clone());

        let session_id = jar
            .get("atom_session")
            .map(|c| c.value().to_string())
            .ok_or_else(|| Redirect::to("/index.html").into_response())?;

        let session = db::crud::get_session(&state.pool, &session_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .ok_or_else(|| Redirect::to("/index.html").into_response())?;

        let user = db::crud::find_user_by_id(&state.pool, session.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .ok_or_else(|| Redirect::to("/index.html").into_response())?;

        Ok(AuthUser(user))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "atom=info,tower_http=info".parse().unwrap()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://host.docker.internal:11434".into());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".into());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let cookie_secret = std::env::var("COOKIE_SECRET")
        .expect("COOKIE_SECRET must be set (at least 64 random characters)");

    // CLI subcommand: atom add-user
    if std::env::args().nth(1).as_deref() == Some("add-user") {
        let pool = db::connect(&database_url).await?;
        cli::add_user(&pool).await?;
        return Ok(());
    }

    let pool = db::connect(&database_url).await?;
    let llm = Arc::new(LlmClient::new(&ollama_url, &ollama_model));
    // SHA-512 the secret to get exactly 64 bytes required by Key::from
    let key_bytes = sha2::Sha512::digest(cookie_secret.as_bytes());
    let cookie_key = Key::from(key_bytes.as_slice());

    // Background task: clean up expired sessions every hour
    {
        let cleanup_pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
                    .execute(&cleanup_pool)
                    .await
                {
                    tracing::warn!("Session cleanup failed: {e}");
                }
            }
        });
    }

    let state = AppState { pool, llm, cookie_key };

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:8080".parse::<axum::http::HeaderValue>().unwrap(),
        ])
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        // Health check (for Docker)
        .route("/health", get(|| async { "ok" }))
        // Auth routes
        .route("/", get(|| async { Redirect::to("/index.html") }))
        .route("/login", post(routes::auth::login))
        .route("/logout", post(routes::auth::logout))
        // API routes (require session)
        .route("/api/quiz/today", get(routes::quiz::get_today_quiz))
        .route("/api/quiz/submit", post(routes::quiz::submit_answer))
        .route("/api/dashboard/stats", get(routes::dashboard::get_stats))
        .route("/api/dashboard/insights", post(routes::dashboard::get_insights))
        // Static files as fallback (HTML + HTMX + Chart.js)
        .fallback_service(ServeDir::new("static"))
        .layer(cors)
        .with_state(state);

    tracing::info!("Atom is running at http://{bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
