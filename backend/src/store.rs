//! 词库与业务查询（store 层）

use sqlx::Row;
use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::models::{AskResult, Sentence, Word};

pub async fn load_words(pool: &SqlitePool) -> AppResult<Vec<Word>> {
    let rows = sqlx::query(
        "SELECT * FROM word WHERE review_status='published' ORDER BY category, level, id",
    )
    .fetch_all(pool)
    .await?;
    rows.iter().map(crate::db::word_from_row).collect()
}

pub async fn load_sentences(pool: &SqlitePool) -> AppResult<Vec<Sentence>> {
    let rows =
        sqlx::query("SELECT * FROM sentence WHERE review_status='published' ORDER BY scene, id")
            .fetch_all(pool)
            .await?;
    rows.iter().map(crate::db::sentence_from_row).collect()
}

pub async fn get_word(pool: &SqlitePool, id: &str) -> AppResult<Option<Word>> {
    let row = sqlx::query("SELECT * FROM word WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(|r| crate::db::word_from_row(&r)).transpose()
}

pub async fn get_sentence(pool: &SqlitePool, id: &str) -> AppResult<Option<Sentence>> {
    let row = sqlx::query("SELECT * FROM sentence WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(|r| crate::db::sentence_from_row(&r)).transpose()
}

/// 按 target 补齐 AskResult 详情（音标/例句/母亲卡等）
pub async fn enrich_ask_result(pool: &SqlitePool, base: AskResult) -> AppResult<AskResult> {
    let mut r = base;
    if r.target_type == "word" {
        if let Some(w) = get_word(pool, &r.target_id).await? {
            r.zh = w.zh;
            r.en = w.en;
            r.phonetic = w.phonetic;
            r.phonetic_source = Some(w.phonetic_source);
            r.category = Some(w.category);
            r.example_en = w.example_en;
            r.example_zh = w.example_zh;
            r.mother_tip = w.mother_tip;
            r.image_emoji = Some(w.image_emoji);
        }
    } else if let Some(s) = get_sentence(pool, &r.target_id).await? {
        r.zh = s.zh;
        r.en = s.en;
        r.phonetic = s.phonetic;
        r.phonetic_source = Some(s.phonetic_source);
        r.scene = Some(s.scene);
        r.example_context = s.example_context;
    }
    Ok(r)
}

/// 场景分类汇总（首页场景快捷区 + 学习模式统计）
pub async fn scene_stats(pool: &SqlitePool, child_id: &str) -> AppResult<serde_json::Value> {
    let rows = sqlx::query(
        "SELECT category, COUNT(*) as total, \
         SUM(CASE WHEN EXISTS(SELECT 1 FROM progress p WHERE p.child_id=? AND p.target_type='word' AND p.target_id=w.id) THEN 1 ELSE 0 END) as learned \
         FROM word w GROUP BY category ORDER BY category",
    )
    .bind(child_id)
    .fetch_all(pool)
    .await?;

    let mut scenes = Vec::new();
    for r in &rows {
        let category: String = r.try_get("category")?;
        let total: i64 = r.try_get("total")?;
        let learned: Option<i64> = r.try_get("learned")?;
        scenes.push(serde_json::json!({
            "category": category,
            "total": total,
            "learned": learned.unwrap_or(0),
        }));
    }
    Ok(serde_json::json!({ "scenes": scenes }))
}
