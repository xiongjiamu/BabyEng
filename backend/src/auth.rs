//! auth.json 多账号认证与账号数据空间辅助。

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AuthFile {
    AccountsObject { accounts: Vec<Account> },
    Accounts(Vec<Account>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Account {
    pub username: String,
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "user".into()
}

pub fn load_accounts(path: &str) -> AppResult<Vec<Account>> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| AppError::Internal(format!("无法读取账号配置 {}: {}", path, e)))?;
    let file: AuthFile = serde_json::from_str(&raw)
        .map_err(|e| AppError::Internal(format!("账号配置格式错误: {}", e)))?;
    Ok(match file {
        AuthFile::AccountsObject { accounts } | AuthFile::Accounts(accounts) => accounts,
    })
}

pub fn account_role(path: &str, username: &str) -> AppResult<String> {
    let account = load_accounts(path)?
        .into_iter()
        .find(|a| a.username == username)
        .ok_or_else(|| AppError::Unauthorized("账号已不存在".into()))?;
    Ok(if account.role == "admin" || account.username == "admin" {
        "admin".into()
    } else {
        "user".into()
    })
}

pub struct AuthService;

impl AuthService {
    pub fn login(&self, path: &str, username: &str, password: &str) -> AppResult<String> {
        let accounts = load_accounts(path)?;
        let valid = accounts.iter().any(|a| {
            !a.username.is_empty()
                && a.username == username
                && constant_time_eq(&a.password, password)
        });
        if !valid {
            return Err(AppError::Unauthorized("账号或密码错误".into()));
        }
        let token = Uuid::new_v4().to_string();
        Ok(token)
    }
}

fn constant_time_eq(expected: &str, actual: &str) -> bool {
    let a = expected.as_bytes();
    let b = actual.as_bytes();
    let mut diff = a.len() ^ b.len();
    let max = a.len().max(b.len());
    for i in 0..max {
        diff |= usize::from(*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0));
    }
    diff == 0
}

pub async fn require_auth(
    State(state): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(request.headers().get(header::AUTHORIZATION))
        .or_else(|| query_token(request.uri().query()))
        .ok_or_else(|| AppError::Unauthorized("请先登录".into()))?;
    let now = Utc::now();
    let expires_at = now + Duration::days(30);
    let refreshed = sqlx::query(
        "UPDATE auth_session SET last_seen_at=?, expires_at=? WHERE token=? AND expires_at>?",
    )
    .bind(now.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .bind(token)
    .bind(now.to_rfc3339())
    .execute(&state.pool)
    .await?;
    if refreshed.rows_affected() == 0 {
        return Err(AppError::Unauthorized("登录已失效，请重新登录".into()));
    }
    let username: String = sqlx::query_scalar("SELECT username FROM auth_session WHERE token=?")
        .bind(token)
        .fetch_one(&state.pool)
        .await?;
    let role = account_role(&state.cfg.auth_file, &username)?;
    request.extensions_mut().insert(AuthUser { username, role });
    Ok(next.run(request).await)
}

fn query_token(query: Option<&str>) -> Option<&str> {
    query?
        .split('&')
        .find_map(|part| part.strip_prefix("access_token="))
}

pub fn bearer_token(value: Option<&axum::http::HeaderValue>) -> Option<&str> {
    value?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|v| !v.is_empty())
}

pub async fn family_id(pool: &SqlitePool, user: &AuthUser) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT family_id FROM account_family WHERE username=?")
            .bind(&user.username)
            .fetch_optional(pool)
            .await?
            .flatten(),
    )
}

pub async fn require_family_id(pool: &SqlitePool, user: &AuthUser) -> AppResult<String> {
    family_id(pool, user)
        .await?
        .ok_or_else(|| AppError::NotFound("家庭未初始化".into()))
}

pub async fn require_child(pool: &SqlitePool, user: &AuthUser, child_id: &str) -> AppResult<()> {
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM child c JOIN account_family a ON a.family_id=c.family_id WHERE a.username=? AND c.child_id=?)",
    )
    .bind(&user.username)
    .bind(child_id)
    .fetch_one(pool)
    .await?;
    if owned {
        Ok(())
    } else {
        Err(AppError::NotFound("孩子不存在".into()))
    }
}

pub async fn default_child(pool: &SqlitePool, user: &AuthUser) -> AppResult<String> {
    sqlx::query_scalar(
        "SELECT c.child_id FROM child c JOIN account_family a ON a.family_id=c.family_id WHERE a.username=? ORDER BY c.created_at LIMIT 1",
    )
    .bind(&user.username)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("未创建孩子档案".into()))
}

pub async fn resolve_child(
    pool: &SqlitePool,
    user: &AuthUser,
    requested: Option<&str>,
) -> AppResult<String> {
    if let Some(id) = requested.filter(|id| !id.is_empty()) {
        require_child(pool, user, id).await?;
        Ok(id.to_string())
    } else {
        default_child(pool, user).await
    }
}
