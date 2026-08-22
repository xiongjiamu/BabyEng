//! 数据库：连接池、迁移、seed 导入、查询辅助

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use tracing::info;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{Sentence, Word};

pub async fn connect(cfg: &Config) -> AppResult<SqlitePool> {
    // 确保数据目录存在
    if let Some(dir) = Path::new(&cfg.database_url)
        .to_str()
        .and_then(|u| u.strip_prefix("sqlite://"))
        .map(|p| {
            let mut d = p.to_string();
            if let Some(idx) = d.rfind('/') {
                d.truncate(idx);
            }
            d
        })
    {
        if !dir.is_empty() {
            std::fs::create_dir_all(&dir).ok();
        }
    }

    let opts = SqliteConnectOptions::new()
        .filename(
            cfg.database_url
                .strip_prefix("sqlite://")
                .unwrap_or(&cfg.database_url),
        )
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .map_err(AppError::Db)?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Internal(format!("数据库迁移失败: {}", e)))?;

    info!("database ready: {}", cfg.database_url);
    Ok(pool)
}

/// 幂等导入 seed 数据（PRD 3.7 / 10.1 的 58 条词句）
pub async fn seed_if_empty(pool: &SqlitePool, seed_dir: &str) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM word")
        .fetch_one(pool)
        .await?;
    seed_subject_items(pool, seed_dir).await?;
    if count > 0 {
        info!("word table already seeded ({} rows), skip", count);
        return Ok(());
    }

    let words_path = format!("{}/words.json", seed_dir);
    let sentences_path = format!("{}/sentences.json", seed_dir);
    if !Path::new(&words_path).exists() {
        info!("seed dir not found: {} (skip seeding)", seed_dir);
        return Ok(());
    }

    let words_json = std::fs::read_to_string(&words_path)?;
    let words: Vec<Word> = serde_json::from_str(&words_json)?;
    let sentences_json = std::fs::read_to_string(&sentences_path)?;
    let sentences: Vec<Sentence> = serde_json::from_str(&sentences_json)?;

    let mut tx = pool.begin().await?;
    for w in &words {
        sqlx::query(
            "INSERT OR REPLACE INTO word (id, zh, aliases, en, pos, phonetic, phonetic_source, category, level, image_emoji, image_source, example_en, example_zh, mother_tip, review_status) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&w.id)
        .bind(&w.zh)
        .bind(serde_json::to_string(&w.aliases)?)
        .bind(&w.en)
        .bind(&w.pos)
        .bind(&w.phonetic)
        .bind(&w.phonetic_source)
        .bind(&w.category)
        .bind(w.level)
        .bind(&w.image_emoji)
        .bind(&w.image_source)
        .bind(&w.example_en)
        .bind(&w.example_zh)
        .bind(&w.mother_tip)
        .bind(&w.review_status)
        .execute(&mut *tx)
        .await?;
        // 展开别名表（含主词 zh，PRD 8.9）
        for a in std::iter::once(&w.zh).chain(w.aliases.iter()) {
            sqlx::query("INSERT OR REPLACE INTO word_alias (alias, word_id) VALUES (?,?)")
                .bind(a)
                .bind(&w.id)
                .execute(&mut *tx)
                .await?;
        }
    }
    for s in &sentences {
        sqlx::query(
            "INSERT OR REPLACE INTO sentence (id, zh, aliases, en, phonetic, phonetic_source, scene, example_context, review_status) \
             VALUES (?,?,?,?,?,?,?,?,?)",
        )
        .bind(&s.id)
        .bind(&s.zh)
        .bind(serde_json::to_string(&s.aliases)?)
        .bind(&s.en)
        .bind(&s.phonetic)
        .bind(&s.phonetic_source)
        .bind(&s.scene)
        .bind(&s.example_context)
        .bind(&s.review_status)
        .execute(&mut *tx)
        .await?;
        for a in std::iter::once(&s.zh).chain(s.aliases.iter()) {
            sqlx::query("INSERT OR REPLACE INTO sentence_alias (alias, sentence_id) VALUES (?,?)")
                .bind(a)
                .bind(&s.id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    info!(
        "seeded {} words, {} sentences from {}",
        words.len(),
        sentences.len(),
        seed_dir
    );
    Ok(())
}

async fn seed_subject_items(pool: &SqlitePool, seed_dir: &str) -> AppResult<()> {
    let path = format!("{}/subject_items.json", seed_dir);
    if !Path::new(&path).exists() {
        return Ok(());
    }
    let items: Vec<crate::models::SubjectItem> =
        serde_json::from_str(&std::fs::read_to_string(path)?)?;
    for item in &items {
        sqlx::query(
            "INSERT OR IGNORE INTO subject_item (id, subject, category, title, prompt, answer, image_emoji, level, scene, materials, parent_script, child_action_a, child_action_b, observe_for, safety_note, material_tags, interest_tags, review_status) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&item.id)
        .bind(&item.subject)
        .bind(&item.category)
        .bind(&item.title)
        .bind(&item.prompt)
        .bind(&item.answer)
        .bind(&item.image_emoji)
        .bind(item.level)
        .bind(&item.scene)
        .bind(&item.materials)
        .bind(&item.parent_script)
        .bind(&item.child_action_a)
        .bind(&item.child_action_b)
        .bind(&item.observe_for)
        .bind(&item.safety_note)
        .bind(serde_json::to_string(&item.material_tags)?)
        .bind(serde_json::to_string(&item.interest_tags)?)
        .bind(&item.review_status)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// 行 → Word（从查询行解析）
pub fn word_from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Word> {
    let aliases: String = row.try_get("aliases")?;
    Ok(Word {
        id: row.try_get("id")?,
        zh: row.try_get("zh")?,
        aliases: serde_json::from_str(&aliases).unwrap_or_default(),
        en: row.try_get("en")?,
        pos: row.try_get("pos")?,
        phonetic: row.try_get("phonetic")?,
        phonetic_source: row.try_get("phonetic_source")?,
        category: row.try_get("category")?,
        level: row.try_get("level")?,
        image_emoji: row.try_get("image_emoji")?,
        image_source: row.try_get("image_source")?,
        tts_audio_path: row.try_get("tts_audio_path")?,
        tts_voice: row.try_get("tts_voice")?,
        example_en: row.try_get("example_en")?,
        example_zh: row.try_get("example_zh")?,
        mother_tip: row.try_get("mother_tip")?,
        review_status: row.try_get("review_status")?,
    })
}

/// 行 → Sentence
pub fn sentence_from_row(row: &sqlx::sqlite::SqliteRow) -> AppResult<Sentence> {
    let aliases: String = row.try_get("aliases")?;
    Ok(Sentence {
        id: row.try_get("id")?,
        zh: row.try_get("zh")?,
        aliases: serde_json::from_str(&aliases).unwrap_or_default(),
        en: row.try_get("en")?,
        phonetic: row.try_get("phonetic")?,
        phonetic_source: row.try_get("phonetic_source")?,
        scene: row.try_get("scene")?,
        tts_audio_path: row.try_get("tts_audio_path")?,
        tts_voice: row.try_get("tts_voice")?,
        example_context: row.try_get("example_context")?,
        review_status: row.try_get("review_status")?,
    })
}

#[cfg(test)]
mod activity_schema_tests {
    use super::*;

    #[tokio::test]
    async fn migrations_add_reviewable_activity_fields() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('subject_item')")
                .fetch_all(&pool)
                .await
                .unwrap();
        for required in [
            "scene",
            "materials",
            "parent_script",
            "child_action_a",
            "child_action_b",
            "observe_for",
            "safety_note",
            "material_tags",
            "interest_tags",
        ] {
            assert!(columns.iter().any(|column| column == required));
        }
    }

    #[test]
    fn published_seed_activities_have_complete_guidance() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/seed/subject_items.json");
        let items: Vec<crate::models::SubjectItem> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for item in items
            .iter()
            .filter(|item| item.review_status == "published")
        {
            assert!(!item.scene.is_empty(), "{} scene", item.id);
            assert!(!item.materials.is_empty(), "{} materials", item.id);
            assert!(!item.parent_script.is_empty(), "{} parent_script", item.id);
            assert!(
                !item.child_action_a.is_empty(),
                "{} child_action_a",
                item.id
            );
            assert!(
                !item.child_action_b.is_empty(),
                "{} child_action_b",
                item.id
            );
            assert!(!item.observe_for.is_empty(), "{} observe_for", item.id);
            assert!(!item.safety_note.is_empty(), "{} safety_note", item.id);
            assert!(!item.material_tags.is_empty(), "{} material_tags", item.id);
        }
    }
}
