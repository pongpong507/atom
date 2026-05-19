use crate::{
    db::crud,
    error::{AppError, Result},
    models::LoginRequest,
    AppState,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar};
use time::Duration;

pub async fn login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(req): Form<LoginRequest>,
) -> Result<Response> {
    let user = crud::find_user_by_username(&state.pool, &req.username)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| AppError::Unauthorized)?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let session_id = crud::create_session(&state.pool, user.user_id).await?;

    let cookie = Cookie::build(("atom_session", session_id))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(Duration::days(7))
        .build();

    Ok((StatusCode::OK, jar.add(cookie), Redirect::to("/quiz.html")).into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    if let Some(session_cookie) = jar.get("atom_session") {
        let _ = crud::delete_session(&state.pool, session_cookie.value()).await;
    }
    let removal = Cookie::build(("atom_session", ""))
        .path("/")
        .max_age(Duration::ZERO)
        .build();
    (jar.remove(removal), Redirect::to("/index.html"))
}
