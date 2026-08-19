//! 家庭与孩子：首次引导（6.7）、设置（6.5/11.3/11.4）、模型配置（9.8）

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{AgeBand, Child, Family};
use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/family/me", get(family_me))
        .route("/api/family/init", post(family_init))
        .route("/api/family/settings", put(family_settings))
        .route("/api/family/child", post(child_create))
        .route("/api/family/child/{child_id}", put(child_update))
        .route("/api/family/child/{child_id}", get(child_get))
}

#[derive(Deserialize)]
struct FamilyInit {
    mother_name: Option<String>,
    child_name: Option<String>,
    child_birthdate: Option<String>, // YYYY-MM-DD
}

#[derive(Serialize)]
struct FamilyView {
    family: Family,
    child: Option<Child>,
    age_band: Option<String>,
    age_months: Option<i64>,
    settings: serde_json::Value,
}

async fn family_me(State(state): State<SharedState>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    // 单家庭：取第一条
    let fam = sqlx::query_as::<_, (String, String, String)>("SELECT family_id, mother_name, settings FROM family LIMIT 1")
        .fetch_optional(pool)
        .await?;
    let Some((family_id, mother_name, settings_json)) = fam else {
        // 未初始化 → 前端进引导页
        return Ok(Json(json!({ "initialized": false })));
    };
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(json!({}));
    let child = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, i64)>(
        "SELECT child_id, family_id, child_name, child_birthdate, age_band_override, level FROM child WHERE family_id=? LIMIT 1",
    )
    .bind(&family_id)
    .fetch_optional(pool)
    .await?
    .map(|(child_id, family_id, child_name, child_birthdate, age_band_override, level)| Child {
        child_id,
        family_id,
        child_name,
        child_birthdate,
        age_band_override,
        level,
    });

    let (age_band, age_months) = match &child {
        Some(c) => {
            let band = c
                .age_band_override
                .clone()
                .or_else(|| {
                    AgeBand::from_birthdate(c.child_birthdate.as_deref(), &chrono::Utc::now())
                        .map(|b| b.label().to_string())
                });
            let months = c
                .child_birthdate
                .as_deref()
                .and_then(|bd| {
                    chrono::NaiveDate::parse_from_str(bd, "%Y-%m-%d").ok().map(|d| {
                        let today = chrono::Local::now().date_naive();
                        (today.year() - d.year()) * 12 + (today.month() as i32 - d.month() as i32)
                            + if today.day() >= d.day() { 0 } else { -1 }
                    })
                })
                .map(|m| m as i64);
            (band, months)
        }
        None => (None, None),
    };

    Ok(Json(json!({
        "initialized": true,
        "family": { "family_id": family_id, "mother_name": mother_name },
        "child": child,
        "age_band": age_band,
        "age_months": age_months,
        "settings": settings,
    })))
}

/// 首次启动引导提交（PRD 6.7：孩子生日 → 自动分段）
async fn family_init(State(state): State<SharedState>, Json(body): Json<FamilyInit>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let family_id = Uuid::new_v4().to_string();
    let child_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO family (family_id, mother_name, settings, created_at) VALUES (?,?,?,?)")
        .bind(&family_id)
        .bind(body.mother_name.clone().unwrap_or_default())
        .bind(json!({
            "tts_rate": 0.8,
            "audio_only": true,          // A 段默认开启，B 段默认关闭（6.6）
            "screen_limit_min": 5,       // A 段手动关闭纯音频后上限 5 分钟/天（11.3）
            "session_limit_min": 3,
            "bedtime_hour": 21,
        }).to_string())
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    // 推导分段：默认 A 段纯音频；跳过生日默认 B 段（6.7）
    let birthdate = body.child_birthdate.clone();
    let band = AgeBand::from_birthdate(birthdate.as_deref(), &chrono::Utc::now());
    let default_audio_only = band.map(|b| b == AgeBand::A).unwrap_or(false);
    sqlx::query(
        "INSERT INTO child (child_id, family_id, child_name, child_birthdate, age_band_override, level, created_at) VALUES (?,?,?,?,?,?,?)",
    )
    .bind(&child_id)
    .bind(&family_id)
    .bind(body.child_name.clone().unwrap_or_else(|| "宝宝".into()))
    .bind(&birthdate)
    .bind(Option::<String>::None)
    .bind(1)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // 按分段修正音频默认值
    if !default_audio_only {
        sqlx::query("UPDATE family SET settings=? WHERE family_id=?")
            .bind(json!({
                "tts_rate": 0.8,
                "audio_only": false,
                "screen_limit_min": 15,  // B 段默认 15 分钟/天（11.3）
                "session_limit_min": 5,
                "bedtime_hour": 21,
            }).to_string())
            .bind(&family_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(Json(json!({
        "ok": true,
        "family_id": family_id,
        "child_id": child_id,
        "age_band": band.map(|b| b.label().to_string()),
        "audio_only": default_audio_only,
    })))
}

#[derive(Deserialize)]
struct SettingsBody {
    settings: serde_json::Value,
}

async fn family_settings(State(state): State<SharedState>, Json(body): Json<SettingsBody>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let fam: Option<(String,)> = sqlx::query_as("SELECT family_id FROM family LIMIT 1")
        .fetch_optional(pool)
        .await?;
    let Some((family_id,)) = fam else {
        return Err(AppError::NotFound("家庭未初始化".into()));
    };
    sqlx::query("UPDATE family SET settings=? WHERE family_id=?")
        .bind(body.settings.to_string())
        .bind(&family_id)
        .execute(pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ChildCreate {
    child_name: Option<String>,
    child_birthdate: Option<String>,
}

async fn child_create(State(state): State<SharedState>, Json(body): Json<ChildCreate>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let fam: Option<(String,)> = sqlx::query_as("SELECT family_id FROM family LIMIT 1")
        .fetch_optional(pool)
        .await?;
    let Some((family_id,)) = fam else {
        return Err(AppError::NotFound("家庭未初始化".into()));
    };
    let child_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO child (child_id, family_id, child_name, child_birthdate, level) VALUES (?,?,?,?,1)",
    )
    .bind(&child_id)
    .bind(&family_id)
    .bind(body.child_name.unwrap_or_else(|| "宝宝".into()))
    .bind(&body.child_birthdate)
    .execute(pool)
    .await?;
    Ok(Json(json!({ "ok": true, "child_id": child_id })))
}

#[derive(Deserialize)]
struct ChildUpdate {
    child_name: Option<String>,
    child_birthdate: Option<String>,
    age_band_override: Option<String>,
    level: Option<i64>,
}

async fn child_update(
    State(state): State<SharedState>,
    Path(child_id): Path<String>,
    Json(body): Json<ChildUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    if let Some(name) = &body.child_name {
        sqlx::query("UPDATE child SET child_name=? WHERE child_id=?")
            .bind(name)
            .bind(&child_id)
            .execute(pool)
            .await?;
    }
    if let Some(bd) = &body.child_birthdate {
        sqlx::query("UPDATE child SET child_birthdate=? WHERE child_id=?")
            .bind(bd)
            .bind(&child_id)
            .execute(pool)
            .await?;
    }
    if let Some(band) = &body.age_band_override {
        let band = if band.is_empty() { None } else { Some(band.clone()) };
        sqlx::query("UPDATE child SET age_band_override=? WHERE child_id=?")
            .bind(band)
            .bind(&child_id)
            .execute(pool)
            .await?;
    }
    if let Some(lv) = body.level {
        sqlx::query("UPDATE child SET level=? WHERE child_id=?")
            .bind(lv)
            .bind(&child_id)
            .execute(pool)
            .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

async fn child_get(State(state): State<SharedState>, Path(child_id): Path<String>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, i64)>(
        "SELECT child_id, family_id, child_name, child_birthdate, age_band_override, level FROM child WHERE child_id=?",
    )
    .bind(&child_id)
    .fetch_optional(pool)
    .await?
    .map(|(child_id, family_id, child_name, child_birthdate, age_band_override, level)| Child {
        child_id,
        family_id,
        child_name,
        child_birthdate,
        age_band_override,
        level,
    });
    let Some(child) = child else {
        return Err(AppError::NotFound("孩子不存在".into()));
    };
    let band = child
        .age_band_override
        .clone()
        .or_else(|| {
            AgeBand::from_birthdate(child.child_birthdate.as_deref(), &chrono::Utc::now())
                .map(|b| b.label().to_string())
        });
    Ok(Json(json!({
        "child": child,
        "age_band": band,
    })))
}
