//! 本地管理后台 API：管理员账号与英/语/数课程维护。

use axum::extract::{Extension, Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::auth::{self, Account, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/admin/users", get(users).post(create_user))
        .route("/api/admin/users/{username}", put(update_user))
        .route("/api/admin/courses", get(courses).post(save_course))
        .route("/api/admin/courses/{id}", put(update_course))
        .route("/api/admin/courses/import", post(import_courses))
}

fn require_admin(user: &AuthUser) -> AppResult<()> {
    if user.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限".into()))
    }
}

async fn users(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let accounts = auth::load_accounts(&state.cfg.auth_file)?;
    let mut result = Vec::new();
    for account in accounts {
        let row = sqlx::query("SELECT a.family_id, f.mother_name, c.child_name FROM account_family a LEFT JOIN family f ON f.family_id=a.family_id LEFT JOIN child c ON c.family_id=a.family_id WHERE a.username=?")
            .bind(&account.username).fetch_optional(&state.pool).await?;
        result.push(json!({
            "username": account.username,
            "role": if account.role == "admin" || account.username == "admin" { "admin" } else { "user" },
            "family_id": row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("family_id").ok()).flatten(),
            "mother_name": row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("mother_name").ok()).flatten(),
            "child_name": row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("child_name").ok()).flatten(),
        }));
    }
    Ok(Json(json!({ "users": result })))
}

#[derive(Deserialize)]
struct UserInput {
    username: String,
    password: String,
    role: Option<String>,
}

async fn create_user(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<UserInput>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    validate_user(&input)?;
    let mut accounts = auth::load_accounts(&state.cfg.auth_file)?;
    if accounts.iter().any(|a| a.username == input.username.trim()) {
        return Err(AppError::BadRequest("账号已存在".into()));
    }
    accounts.push(Account {
        username: input.username.trim().into(),
        password: input.password,
        role: normalized_role(input.role.as_deref())?,
    });
    save_accounts(&state.cfg.auth_file, &accounts)?;
    sqlx::query("INSERT OR IGNORE INTO account_family (username) VALUES (?)")
        .bind(input.username.trim())
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn update_user(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path(username): Path<String>,
    Json(input): Json<UserInput>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    if input.username.trim() != username {
        return Err(AppError::BadRequest("不能修改账号名".into()));
    }
    validate_user(&input)?;
    let mut accounts = auth::load_accounts(&state.cfg.auth_file)?;
    let account = accounts
        .iter_mut()
        .find(|a| a.username == username)
        .ok_or_else(|| AppError::NotFound("账号不存在".into()))?;
    account.password = input.password;
    account.role = normalized_role(input.role.as_deref())?;
    save_accounts(&state.cfg.auth_file, &accounts)?;
    Ok(Json(json!({ "ok": true })))
}

fn validate_user(input: &UserInput) -> AppResult<()> {
    let name = input.username.trim();
    if name.len() < 3
        || name.len() > 40
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::BadRequest(
            "账号需为 3~40 位字母、数字、横线或下划线".into(),
        ));
    }
    if input.password.len() < 8 {
        return Err(AppError::BadRequest("密码至少 8 位".into()));
    }
    Ok(())
}

fn normalized_role(role: Option<&str>) -> AppResult<String> {
    match role.unwrap_or("user") {
        "user" => Ok("user".into()),
        "admin" => Ok("admin".into()),
        _ => Err(AppError::BadRequest("role 非法".into())),
    }
}

fn save_accounts(path: &str, accounts: &[Account]) -> AppResult<()> {
    let backup = format!("{}.bak", path);
    if std::path::Path::new(path).exists() {
        let _ = std::fs::copy(path, backup);
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&json!({ "accounts": accounts }))?,
    )?;
    Ok(())
}

#[derive(Deserialize)]
struct CourseQuery {
    subject: String,
}

async fn courses(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<CourseQuery>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let items = match q.subject.as_str() {
        "english" => english_courses(&state).await?,
        "chinese" | "math" => subject_courses(&state, &q.subject).await?,
        _ => return Err(AppError::BadRequest("subject 非法".into())),
    };
    Ok(Json(json!({ "items": items, "total": items.len() })))
}

async fn english_courses(state: &SharedState) -> AppResult<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT id, zh, en, aliases, category, level, image_emoji, phonetic, example_en, example_zh, mother_tip, review_status FROM word ORDER BY category, level, id").fetch_all(&state.pool).await?;
    let mut items: Vec<serde_json::Value> = rows.iter().map(|r| Ok(json!({
        "id": r.try_get::<String,_>("id")?, "subject":"english", "kind":"word", "zh":r.try_get::<String,_>("zh")?, "en":r.try_get::<String,_>("en")?,
        "aliases":serde_json::from_str::<Vec<String>>(&r.try_get::<String,_>("aliases")?).unwrap_or_default(), "category":r.try_get::<String,_>("category")?,
        "level":r.try_get::<i64,_>("level")?, "image_emoji":r.try_get::<String,_>("image_emoji")?, "phonetic":r.try_get::<Option<String>,_>("phonetic")?,
        "example_en":r.try_get::<Option<String>,_>("example_en")?, "example_zh":r.try_get::<Option<String>,_>("example_zh")?, "mother_tip":r.try_get::<Option<String>,_>("mother_tip")?, "review_status":r.try_get::<String,_>("review_status")?
    }))).collect::<AppResult<_>>()?;
    let rows = sqlx::query("SELECT id, zh, en, aliases, scene, phonetic, example_context, review_status FROM sentence ORDER BY scene, id").fetch_all(&state.pool).await?;
    for r in rows {
        items.push(json!({"id":r.try_get::<String,_>("id")?, "subject":"english", "kind":"sentence", "zh":r.try_get::<String,_>("zh")?, "en":r.try_get::<String,_>("en")?, "aliases":serde_json::from_str::<Vec<String>>(&r.try_get::<String,_>("aliases")?).unwrap_or_default(), "category":r.try_get::<String,_>("scene")?, "level":1, "image_emoji":"💬", "phonetic":r.try_get::<Option<String>,_>("phonetic")?, "example_zh":r.try_get::<Option<String>,_>("example_context")?, "review_status":r.try_get::<String,_>("review_status")?}));
    }
    Ok(items)
}

async fn subject_courses(state: &SharedState, subject: &str) -> AppResult<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT id, subject, category, title, prompt, answer, image_emoji, level, review_status FROM subject_item WHERE subject=? ORDER BY category, level, id").bind(subject).fetch_all(&state.pool).await?;
    rows.iter().map(|r| Ok(json!({"id":r.try_get::<String,_>("id")?, "subject":r.try_get::<String,_>("subject")?, "kind":"activity", "category":r.try_get::<String,_>("category")?, "title":r.try_get::<String,_>("title")?, "prompt":r.try_get::<String,_>("prompt")?, "answer":r.try_get::<String,_>("answer")?, "image_emoji":r.try_get::<String,_>("image_emoji")?, "level":r.try_get::<i64,_>("level")?, "review_status":r.try_get::<String,_>("review_status")?}))).collect()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CourseInput {
    id: String,
    subject: String,
    #[serde(default)]
    kind: String,
    category: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    answer: String,
    #[serde(default)]
    zh: String,
    #[serde(default)]
    en: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    phonetic: Option<String>,
    #[serde(default)]
    image_emoji: String,
    #[serde(default = "one")]
    level: i64,
    #[serde(default)]
    example_en: Option<String>,
    #[serde(default)]
    example_zh: Option<String>,
    #[serde(default)]
    mother_tip: Option<String>,
    #[serde(default = "draft")]
    review_status: String,
}
fn one() -> i64 {
    1
}
fn draft() -> String {
    "draft".into()
}

async fn save_course(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<CourseInput>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    upsert_course(&state, &input, false).await?;
    Ok(Json(json!({ "ok": true, "id": input.id })))
}

async fn update_course(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(input): Json<CourseInput>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    if input.id != id {
        return Err(AppError::BadRequest("不能修改课程 ID".into()));
    }
    upsert_course(&state, &input, true).await?;
    Ok(Json(json!({ "ok": true, "id": input.id })))
}

#[derive(Deserialize)]
struct ImportBody {
    items: Vec<CourseInput>,
}
async fn import_courses(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ImportBody>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    if body.items.is_empty() || body.items.len() > 500 {
        return Err(AppError::BadRequest("每次导入 1~500 条课程".into()));
    }
    for input in &body.items {
        upsert_course(&state, input, true).await?;
    }
    Ok(Json(json!({ "ok": true, "imported": body.items.len() })))
}

async fn upsert_course(
    state: &SharedState,
    input: &CourseInput,
    allow_replace: bool,
) -> AppResult<()> {
    validate_course(input)?;
    if !allow_replace {
        let exists: bool = if input.subject == "english" && input.kind == "sentence" {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sentence WHERE id=?)")
                .bind(&input.id)
                .fetch_one(&state.pool)
                .await?
        } else if input.subject == "english" {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM word WHERE id=?)")
                .bind(&input.id)
                .fetch_one(&state.pool)
                .await?
        } else {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM subject_item WHERE id=?)")
                .bind(&input.id)
                .fetch_one(&state.pool)
                .await?
        };
        if exists {
            return Err(AppError::BadRequest("课程 ID 已存在".into()));
        }
    }
    if input.subject == "english" && input.kind == "sentence" {
        sqlx::query("INSERT INTO sentence (id, zh, aliases, en, phonetic, phonetic_source, scene, example_context, review_status) VALUES (?,?,?,?,?,'manual',?,?,?) ON CONFLICT(id) DO UPDATE SET zh=excluded.zh, aliases=excluded.aliases, en=excluded.en, phonetic=excluded.phonetic, scene=excluded.scene, example_context=excluded.example_context, review_status=excluded.review_status")
            .bind(&input.id).bind(&input.zh).bind(serde_json::to_string(&input.aliases)?).bind(&input.en).bind(&input.phonetic).bind(&input.category).bind(&input.example_zh).bind(&input.review_status).execute(&state.pool).await?;
        sqlx::query("DELETE FROM sentence_alias WHERE sentence_id=?")
            .bind(&input.id)
            .execute(&state.pool)
            .await?;
        for alias in std::iter::once(&input.zh).chain(input.aliases.iter()) {
            sqlx::query("INSERT OR IGNORE INTO sentence_alias (alias, sentence_id) VALUES (?,?)")
                .bind(alias)
                .bind(&input.id)
                .execute(&state.pool)
                .await?;
        }
        refresh_matcher(state).await?;
    } else if input.subject == "english" {
        sqlx::query("INSERT INTO word (id, zh, aliases, en, pos, phonetic, phonetic_source, category, level, image_emoji, image_source, example_en, example_zh, mother_tip, review_status) VALUES (?,?,?,?, 'noun', ?, 'manual', ?, ?, ?, 'generated', ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET zh=excluded.zh, aliases=excluded.aliases, en=excluded.en, phonetic=excluded.phonetic, category=excluded.category, level=excluded.level, image_emoji=excluded.image_emoji, example_en=excluded.example_en, example_zh=excluded.example_zh, mother_tip=excluded.mother_tip, review_status=excluded.review_status")
            .bind(&input.id).bind(&input.zh).bind(serde_json::to_string(&input.aliases)?).bind(&input.en).bind(&input.phonetic).bind(&input.category).bind(input.level).bind(&input.image_emoji).bind(&input.example_en).bind(&input.example_zh).bind(&input.mother_tip).bind(&input.review_status).execute(&state.pool).await?;
        sqlx::query("DELETE FROM word_alias WHERE word_id=?")
            .bind(&input.id)
            .execute(&state.pool)
            .await?;
        for alias in std::iter::once(&input.zh).chain(input.aliases.iter()) {
            sqlx::query("INSERT OR IGNORE INTO word_alias (alias, word_id) VALUES (?,?)")
                .bind(alias)
                .bind(&input.id)
                .execute(&state.pool)
                .await?;
        }
        refresh_matcher(state).await?;
    } else {
        sqlx::query("INSERT INTO subject_item (id, subject, category, title, prompt, answer, image_emoji, level, review_status, updated_at) VALUES (?,?,?,?,?,?,?,?,?,strftime('%Y-%m-%dT%H:%M:%SZ','now')) ON CONFLICT(id) DO UPDATE SET subject=excluded.subject, category=excluded.category, title=excluded.title, prompt=excluded.prompt, answer=excluded.answer, image_emoji=excluded.image_emoji, level=excluded.level, review_status=excluded.review_status, updated_at=excluded.updated_at")
            .bind(&input.id).bind(&input.subject).bind(&input.category).bind(&input.title).bind(&input.prompt).bind(&input.answer).bind(&input.image_emoji).bind(input.level).bind(&input.review_status).execute(&state.pool).await?;
    }
    Ok(())
}

async fn refresh_matcher(state: &SharedState) -> AppResult<()> {
    let words = crate::store::load_words(&state.pool).await?;
    let sentences = crate::store::load_sentences(&state.pool).await?;
    *state.matcher.write().await = crate::matcher::Matcher::new(&words, &sentences);
    Ok(())
}

fn validate_course(input: &CourseInput) -> AppResult<()> {
    if input.id.is_empty()
        || input.id.len() > 80
        || !input
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest("课程 ID 非法".into()));
    }
    if !["english", "chinese", "math"].contains(&input.subject.as_str()) {
        return Err(AppError::BadRequest("subject 非法".into()));
    }
    if !["draft", "published"].contains(&input.review_status.as_str()) {
        return Err(AppError::BadRequest("review_status 非法".into()));
    }
    if input.category.trim().is_empty() || input.level < 1 || input.level > 3 {
        return Err(AppError::BadRequest("分类或难度非法".into()));
    }
    if input.subject == "english" && (input.zh.trim().is_empty() || input.en.trim().is_empty()) {
        return Err(AppError::BadRequest("英语课程必须填写中英文".into()));
    }
    if input.subject == "english"
        && !input.kind.is_empty()
        && !["word", "sentence"].contains(&input.kind.as_str())
    {
        return Err(AppError::BadRequest("英语课程 kind 非法".into()));
    }
    if input.subject != "english"
        && (input.title.trim().is_empty()
            || input.prompt.trim().is_empty()
            || input.answer.trim().is_empty())
    {
        return Err(AppError::BadRequest(
            "语文/数学课程必须填写标题、引导语和答案".into(),
        ));
    }
    Ok(())
}
