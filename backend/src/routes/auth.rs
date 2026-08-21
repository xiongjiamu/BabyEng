//! 登录、退出和当前账号。

use axum::extract::{Extension, State};
use axum::http::{header, HeaderMap};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{bearer_token, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::SharedState;

pub fn public_router() -> Router<SharedState> {
    Router::new().route("/api/auth/login", post(login))
}

pub fn protected_router() -> Router<SharedState> {
    Router::new()
        .route("/api/auth/me", get(me))
        .route("/api/auth/logout", post(logout))
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(
    State(state): State<SharedState>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<serde_json::Value>> {
    let username = body.username.trim();
    if username.is_empty() || body.password.is_empty() {
        return Err(AppError::BadRequest("账号和密码不能为空".into()));
    }
    let token = state
        .auth
        .login(&state.cfg.auth_file, username, &body.password)?;
    let role = crate::auth::account_role(&state.cfg.auth_file, username)?;
    let now = Utc::now();
    let expires_at = now + Duration::days(30);
    sqlx::query(
        "INSERT INTO auth_session (token, username, created_at, last_seen_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(username)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .execute(&state.pool)
    .await?;
    sqlx::query("INSERT OR IGNORE INTO account_family (username) VALUES (?)")
        .bind(username)
        .execute(&state.pool)
        .await?;
    // 从单家庭旧版本升级时，首个登录账号自动接管唯一的遗留家庭。
    sqlx::query(
        "UPDATE account_family SET family_id=(SELECT f.family_id FROM family f LEFT JOIN account_family a ON a.family_id=f.family_id WHERE a.family_id IS NULL LIMIT 1) \
         WHERE username=? AND family_id IS NULL \
         AND NOT EXISTS(SELECT 1 FROM account_family WHERE family_id IS NOT NULL) \
         AND 1=(SELECT COUNT(*) FROM family f LEFT JOIN account_family a ON a.family_id=f.family_id WHERE a.family_id IS NULL)",
    )
    .bind(username)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({
        "ok": true,
        "token": token,
        "username": username,
        "role": role,
        "expires_at": expires_at.to_rfc3339(),
    })))
}

async fn me(Extension(user): Extension<AuthUser>) -> Json<serde_json::Value> {
    Json(json!({ "authenticated": true, "username": user.username, "role": user.role }))
}

async fn logout(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let token = bearer_token(headers.get(header::AUTHORIZATION))
        .ok_or_else(|| AppError::Unauthorized("请先登录".into()))?;
    sqlx::query("DELETE FROM auth_session WHERE token=?")
        .bind(token)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}
