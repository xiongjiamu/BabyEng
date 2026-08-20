//! auth.json 多账号认证与账号数据空间辅助。

use std::collections::HashMap;
use std::sync::RwLock;

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AuthFile {
    AccountsObject { accounts: Vec<Account> },
    Accounts(Vec<Account>),
}

#[derive(Debug, Deserialize)]
struct Account {
    username: String,
    password: String,
}

#[derive(Default)]
pub struct AuthService {
    sessions: RwLock<HashMap<String, String>>,
}

impl AuthService {
    pub fn login(&self, path: &str, username: &str, password: &str) -> AppResult<String> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            AppError::Internal(format!("无法读取账号配置 {}: {}", path, e))
        })?;
        let file: AuthFile = serde_json::from_str(&raw)
            .map_err(|e| AppError::Internal(format!("账号配置格式错误: {}", e)))?;
        let accounts = match file {
            AuthFile::AccountsObject { accounts } | AuthFile::Accounts(accounts) => accounts,
        };
        let valid = accounts.iter().any(|a| {
            !a.username.is_empty() && a.username == username && constant_time_eq(&a.password, password)
        });
        if !valid {
            return Err(AppError::Unauthorized("账号或密码错误".into()));
        }
        let token = Uuid::new_v4().to_string();
        self.sessions
            .write()
            .map_err(|_| AppError::Internal("会话锁异常".into()))?
            .insert(token.clone(), username.to_string());
        Ok(token)
    }

    pub fn username_for_token(&self, token: &str) -> Option<String> {
        self.sessions.read().ok()?.get(token).cloned()
    }

    pub fn logout(&self, token: &str) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(token);
        }
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
    let username = state
        .auth
        .username_for_token(token)
        .ok_or_else(|| AppError::Unauthorized("登录已失效，请重新登录".into()))?;
    request.extensions_mut().insert(AuthUser { username });
    Ok(next.run(request).await)
}

fn query_token(query: Option<&str>) -> Option<&str> {
    query?.split('&').find_map(|part| part.strip_prefix("access_token="))
}

pub fn bearer_token(value: Option<&axum::http::HeaderValue>) -> Option<&str> {
    value?.to_str().ok()?.strip_prefix("Bearer ").filter(|v| !v.is_empty())
}

pub async fn family_id(pool: &SqlitePool, user: &AuthUser) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT family_id FROM account_family WHERE username=?")
        .bind(&user.username)
        .fetch_optional(pool)
        .await?
        .flatten())
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
    if owned { Ok(()) } else { Err(AppError::NotFound("孩子不存在".into())) }
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

pub async fn resolve_child(pool: &SqlitePool, user: &AuthUser, requested: Option<&str>) -> AppResult<String> {
    if let Some(id) = requested.filter(|id| !id.is_empty()) {
        require_child(pool, user, id).await?;
        Ok(id.to_string())
    } else {
        default_child(pool, user).await
    }
}
