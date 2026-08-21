//! 词条/句子/场景查询（M2 学习模式、M5 场景库数据来源）

use axum::extract::{Extension, Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Timelike;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::auth::{self, AuthUser};
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
        .route("/api/subject-items", get(subject_items))
        .route("/api/activities/today", get(today_activities))
}

#[derive(Deserialize)]
struct SubjectQuery {
    subject: String,
    child_id: Option<String>,
}

async fn subject_items(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<SubjectQuery>,
) -> AppResult<Json<serde_json::Value>> {
    if !["chinese", "math"].contains(&q.subject.as_str()) {
        return Err(crate::error::AppError::BadRequest("subject 非法".into()));
    }
    let child_id = auth::resolve_child(&state.pool, &user, q.child_id.as_deref()).await?;
    let learned_rows = sqlx::query(
        "SELECT target_id FROM progress WHERE child_id=? AND target_type='subject_item' AND learn_count>0",
    )
    .bind(&child_id)
    .fetch_all(&state.pool)
    .await?;
    let learned = learned_rows
        .iter()
        .map(|row| row.try_get::<String, _>("target_id"))
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    let rows = sqlx::query("SELECT id, subject, category, title, prompt, answer, image_emoji, level, scene, materials, parent_script, child_action_a, child_action_b, observe_for, safety_note, material_tags, interest_tags, review_status FROM subject_item WHERE subject=? AND review_status='published' ORDER BY level, category, id")
        .bind(&q.subject)
        .fetch_all(&state.pool)
        .await?;
    let items = rows.into_iter().map(|row| {
        let id: String = row.try_get("id")?;
        Ok(json!({
            "id": id, "subject": row.try_get::<String, _>("subject")?,
            "category": row.try_get::<String, _>("category")?, "title": row.try_get::<String, _>("title")?,
            "prompt": row.try_get::<String, _>("prompt")?, "answer": row.try_get::<String, _>("answer")?,
            "image_emoji": row.try_get::<String, _>("image_emoji")?, "level": row.try_get::<i64, _>("level")?,
            "scene": row.try_get::<String, _>("scene")?, "materials": row.try_get::<String, _>("materials")?,
            "parent_script": row.try_get::<String, _>("parent_script")?, "child_action_a": row.try_get::<String, _>("child_action_a")?,
            "child_action_b": row.try_get::<String, _>("child_action_b")?, "observe_for": row.try_get::<String, _>("observe_for")?,
            "safety_note": row.try_get::<String, _>("safety_note")?,
            "material_tags": parse_tags(&row.try_get::<String, _>("material_tags")?),
            "interest_tags": parse_tags(&row.try_get::<String, _>("interest_tags")?),
            "review_status": row.try_get::<String, _>("review_status")?, "learned": learned.contains(&id),
        }))
    }).collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(Json(json!({ "total": items.len(), "items": items })))
}

#[derive(Deserialize)]
struct TodayActivityQuery {
    child_id: Option<String>,
}

async fn today_activities(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<TodayActivityQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let child_id = auth::resolve_child(&state.pool, &user, q.child_id.as_deref()).await?;
    let hour = chrono::Local::now().hour();
    let scene = match hour {
        6..=9 => "morning",
        10..=13 | 18..=19 => "meal",
        20..=23 => "bedtime",
        _ => "play",
    };
    let settings_json: String = sqlx::query_scalar(
        "SELECT f.settings FROM child c JOIN family f ON f.family_id=c.family_id WHERE c.child_id=?",
    )
    .bind(&child_id)
    .fetch_one(&state.pool)
    .await?;
    let settings: serde_json::Value = serde_json::from_str(&settings_json).unwrap_or(json!({}));
    let available_materials = setting_tags(&settings, "available_materials");
    let child_interests = setting_tags(&settings, "child_interests");
    let feedback_since = (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339();
    let feedback_rows = sqlx::query(
        "SELECT target_id, mother_mark FROM learning_record WHERE child_id=? \
         AND target_type='subject_item' AND action='observe' AND recorded_at>=? \
         ORDER BY recorded_at DESC",
    )
    .bind(&child_id)
    .bind(&feedback_since)
    .fetch_all(&state.pool)
    .await?;
    let feedback = feedback_rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("target_id")?,
                row.try_get::<Option<String>, _>("mother_mark")?,
            ))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let recently_not_interested = latest_not_interested(feedback);
    let rows = sqlx::query(
        "SELECT s.id, s.subject, s.category, s.title, s.prompt, s.answer, s.image_emoji, s.level, s.scene, s.materials, s.parent_script, s.child_action_a, s.child_action_b, s.observe_for, s.safety_note, s.material_tags, s.interest_tags, \
         COALESCE(p.learn_count,0) AS learn_count FROM subject_item s LEFT JOIN progress p ON p.child_id=? AND p.target_type='subject_item' AND p.target_id=s.id \
         WHERE s.review_status='published' ORDER BY CASE WHEN COALESCE(p.learn_count,0)=0 THEN 0 ELSE 1 END, s.subject, s.category, s.id",
    )
    .bind(&child_id)
    .fetch_all(&state.pool)
    .await?;

    let mut scene_interests = Vec::new();
    let mut scene_others = Vec::new();
    let mut other_interests = Vec::new();
    let mut other_items = Vec::new();
    let mut deferred = Vec::new();
    for row in rows {
        let id: String = row.try_get("id")?;
        let item_scene: String = row.try_get("scene")?;
        let material_tags = parse_tags(&row.try_get::<String, _>("material_tags")?);
        if !matches_available_materials(&material_tags, &available_materials) {
            continue;
        }
        let interest_tags = parse_tags(&row.try_get::<String, _>("interest_tags")?);
        let matches_interest = has_tag_overlap(&interest_tags, &child_interests);
        let is_deferred = recently_not_interested.contains(&id);
        let value = json!({
            "id": id.clone(), "subject": row.try_get::<String,_>("subject")?,
            "category": row.try_get::<String,_>("category")?, "title": row.try_get::<String,_>("title")?,
            "prompt": row.try_get::<String,_>("prompt")?, "answer": row.try_get::<String,_>("answer")?,
            "image_emoji": row.try_get::<String,_>("image_emoji")?, "level": row.try_get::<i64,_>("level")?,
            "scene": item_scene.clone(), "materials": row.try_get::<String,_>("materials")?,
            "parent_script": row.try_get::<String,_>("parent_script")?, "child_action_a": row.try_get::<String,_>("child_action_a")?,
            "child_action_b": row.try_get::<String,_>("child_action_b")?, "observe_for": row.try_get::<String,_>("observe_for")?,
            "safety_note": row.try_get::<String,_>("safety_note")?, "learned": row.try_get::<i64,_>("learn_count")? > 0,
            "material_tags": material_tags, "interest_tags": interest_tags, "interest_match": matches_interest,
            "recently_not_interested": is_deferred,
        });
        if is_deferred {
            deferred.push(value);
        } else if item_scene == scene && matches_interest {
            scene_interests.push(value);
        } else if item_scene == scene {
            scene_others.push(value);
        } else if matches_interest {
            other_interests.push(value);
        } else {
            other_items.push(value);
        }
    }
    scene_interests.extend(scene_others);
    scene_interests.truncate(2);
    other_interests.extend(other_items);
    scene_interests.extend(other_interests.into_iter().take(3 - scene_interests.len()));
    for value in deferred {
        if scene_interests.len() >= 3 {
            break;
        }
        let current_scene_count = scene_interests
            .iter()
            .filter(|item| item["scene"].as_str() == Some(scene))
            .count();
        if value["scene"].as_str() != Some(scene) || current_scene_count < 2 {
            scene_interests.push(value);
        }
    }
    Ok(Json(json!({
        "date": chrono::Local::now().date_naive().format("%Y-%m-%d").to_string(),
        "scene": scene,
        "items": scene_interests,
        "preferences_applied": !available_materials.is_empty() || !child_interests.is_empty(),
    })))
}

fn parse_tags(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn setting_tags(settings: &serde_json::Value, key: &str) -> Vec<String> {
    settings
        .get(key)
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn matches_available_materials(required: &[String], available: &[String]) -> bool {
    available.is_empty() || required.is_empty() || has_tag_overlap(required, available)
}

fn has_tag_overlap(left: &[String], right: &[String]) -> bool {
    !right.is_empty() && left.iter().any(|tag| right.contains(tag))
}

/// 输入按时间倒序；同一活动只采用最新反馈。
fn latest_not_interested(
    feedback: impl IntoIterator<Item = (String, Option<String>)>,
) -> std::collections::HashSet<String> {
    let mut latest = std::collections::HashMap::new();
    for (id, mark) in feedback {
        latest.entry(id).or_insert(mark);
    }
    latest
        .into_iter()
        .filter_map(|(id, mark)| (mark.as_deref() == Some("not_interested")).then_some(id))
        .collect()
}

#[derive(Deserialize)]
struct WordQuery {
    category: Option<String>,
    level: Option<i64>,
    /// 已学的进度信息（word-learn 页显示）
    child_id: Option<String>,
    limit: Option<i64>,
}

async fn words(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<WordQuery>,
) -> AppResult<Json<serde_json::Value>> {
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
        auth::require_child(pool, &user, cid).await?;
        let rows =
            sqlx::query("SELECT target_id FROM progress WHERE child_id=? AND target_type='word'")
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

async fn word_detail(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let w = store::get_word(pool, &id).await?;
    let Some(w) = w else {
        return Err(crate::error::AppError::NotFound("词条不存在".into()));
    };
    Ok(Json(serde_json::to_value(w)?))
}

async fn sentences(
    State(state): State<SharedState>,
    Query(q): Query<WordQuery>,
) -> AppResult<Json<serde_json::Value>> {
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

async fn sentence_detail(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let s = store::get_sentence(pool, &id).await?;
    let Some(s) = s else {
        return Err(crate::error::AppError::NotFound("句子不存在".into()));
    };
    Ok(Json(serde_json::to_value(s)?))
}

/// 场景分类汇总（首页场景快捷区 / 学习模式入口统计）
async fn scenes(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<WordQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.pool;
    let child_id = auth::resolve_child(pool, &user, q.child_id.as_deref()).await?;
    let stats = store::scene_stats(pool, &child_id).await?;
    Ok(Json(stats))
}

#[cfg(test)]
mod activity_preference_tests {
    use super::*;

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn empty_material_preference_does_not_filter() {
        assert!(matches_available_materials(&tags(&["toys_blocks"]), &[]));
    }

    #[test]
    fn configured_materials_require_overlap() {
        assert!(matches_available_materials(
            &tags(&["toys_blocks", "household_objects"]),
            &tags(&["toys_blocks"])
        ));
        assert!(!matches_available_materials(
            &tags(&["food_tableware"]),
            &tags(&["clothing"])
        ));
    }

    #[test]
    fn interest_match_only_applies_when_configured() {
        assert!(has_tag_overlap(
            &tags(&["animals", "movement"]),
            &tags(&["animals"])
        ));
        assert!(!has_tag_overlap(&tags(&["animals"]), &[]));
    }

    #[test]
    fn latest_feedback_can_clear_not_interested_downgrade() {
        let deferred = latest_not_interested([
            ("cleared".into(), Some("observed_with_help".into())),
            ("cleared".into(), Some("not_interested".into())),
            ("still_deferred".into(), Some("not_interested".into())),
        ]);
        assert!(!deferred.contains("cleared"));
        assert!(deferred.contains("still_deferred"));
    }
}
