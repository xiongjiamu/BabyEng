//! 词条/句子/场景查询（M2 学习模式、M5 场景库数据来源）

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::error::AppResult;
use crate::models::{Sentence, Word};
use crate::state::SharedState;
use crate::store;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/words", get(words))
        .route("/api/words/{id}", get(word_detail))
        .route("/api/sentences", get(sentences))
        .route("/api/sentences/{id}", get(sentence_detail))
        .route("/api/scenes", get(scenes))
}

#[derive(Deserialize)]
struct WordQuery {
    category: Option<String>,
    level: Option<i64>,
    /// 已学的进度信息（word-learn 页显示）
    child_id: Option<String>,
    limit: Option<i64>,
}

async fn words(State(state): State<SharedState>, Query(q): Query<WordQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let mut sql = String::from("SELECT * FROM word WHERE review_status='published'");

    if let Some(cat) = &q.category {
        match cat.as_str() {
            "item" => sql.push_str(" AND category LIKE 'item_%'"),
            "person" => sql.push_str(" AND category LIKE 'person_%'"),
            "number" => sql.push_str(" AND category='number'"),
            "emotion" => sql.push_str(" AND category='emotion'"),
            _ => {}
        }
    }
    if let Some(lv) = q.level {
        sql.push_str(&format!(" AND level={}", lv));
    }
    sql.push_str(" ORDER BY category, level, id");
    if let Some(limit) = q.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }

    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    let mut words: Vec<Word> = Vec::new();
    for r in &rows {
        words.push(crate::db::word_from_row(r)?);
    }

    // 进度信息（已学标记）
    let mut learned = std::collections::HashSet::new();
    if let Some(cid) = &q.child_id {
        let rows = sqlx::query("SELECT target_id FROM progress WHERE child_id=? AND target_type='word'")
            .bind(cid)
            .fetch_all(pool)
            .await?;
        for r in rows {
            learned.insert(r.try_get::<String, _>("target_id")?);
        }
    }

    Ok(Json(json!({
        "words": words.iter().map(|w| {
            let mut v = serde_json::to_value(w).unwrap_or(json!({}));
            v["learned"] = json!(learned.contains(&w.id));
            v
        }).collect::<Vec<_>>(),
        "total": words.len(),
    })))
}

async fn word_detail(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let w = store::get_word(pool, &id).await?;
    let Some(w) = w else {
        return Err(crate::error::AppError::NotFound("词条不存在".into()));
    };
    Ok(Json(serde_json::to_value(w)?))
}

async fn sentences(State(state): State<SharedState>, Query(q): Query<WordQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let mut sql = String::from("SELECT * FROM sentence WHERE review_status='published'");
    if let Some(scene) = &q.category {
        // scene 参数来自前端枚举（morning/meal/play/bedtime/outing），白名单拼接
        if ["morning", "meal", "play", "bedtime", "outing"].contains(&scene.as_str()) {
            sql.push_str(&format!(" AND scene='{}'", scene));
        }
    }
    sql.push_str(" ORDER BY scene, id");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    let mut sents: Vec<Sentence> = Vec::new();
    for r in &rows {
        sents.push(crate::db::sentence_from_row(r)?);
    }
    Ok(Json(json!({ "sentences": sents, "total": sents.len() })))
}

async fn sentence_detail(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let s = store::get_sentence(pool, &id).await?;
    let Some(s) = s else {
        return Err(crate::error::AppError::NotFound("句子不存在".into()));
    };
    Ok(Json(serde_json::to_value(s)?))
}

/// 场景分类汇总（首页场景快捷区 / 学习模式入口统计）
async fn scenes(State(state): State<SharedState>, Query(q): Query<WordQuery>) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = q.child_id.unwrap_or_default();
    let stats = store::scene_stats(pool, &child_id).await?;
    Ok(Json(stats))
}
