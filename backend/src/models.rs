//! 数据模型：与 PRD 8.x 表结构一一对应

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Word {
    pub id: String,
    pub zh: String,
    pub aliases: Vec<String>,
    pub en: String,
    pub pos: String,
    pub phonetic: Option<String>,
    pub phonetic_source: String,
    pub category: String,
    pub level: i64,
    pub image_emoji: String,
    #[serde(default = "default_image_source")]
    pub image_source: String,
    pub tts_audio_path: Option<String>,
    pub tts_voice: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub mother_tip: Option<String>,
    #[serde(default = "default_review_status")]
    pub review_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentence {
    pub id: String,
    pub zh: String,
    pub aliases: Vec<String>,
    pub en: String,
    pub phonetic: Option<String>,
    pub phonetic_source: String,
    pub scene: String,
    pub tts_audio_path: Option<String>,
    pub tts_voice: Option<String>,
    pub example_context: Option<String>,
    #[serde(default = "default_review_status")]
    pub review_status: String,
}

fn default_image_source() -> String {
    "family".into()
}

fn default_review_status() -> String {
    "published".into()
}

/// 问答结果卡（M1 输出契约，PRD 4.1）
#[derive(Debug, Clone, Serialize)]
pub struct AskResult {
    pub target_type: String, // word / sentence
    pub target_id: String,
    pub zh: String,
    pub en: String,
    pub phonetic: Option<String>,
    pub phonetic_source: Option<String>,
    pub category: Option<String>,
    pub scene: Option<String>,
    pub example_en: Option<String>,
    pub example_zh: Option<String>,
    pub example_context: Option<String>,
    pub mother_tip: Option<String>,
    pub image_emoji: Option<String>,
    pub match_level: String, // L0 / L1 / L2 / L3 / L4 / none
    pub tts_available: bool,
    pub tts_url: Option<String>,
}

/// 未命中时的相近词推荐（PRD 4.1.1：让母亲有台阶下）
#[derive(Debug, Clone, Serialize)]
pub struct AskResponse {
    pub status: String, // hit / ambiguous / nomatch / asr_fail / tts_only_down
    pub result: Option<AskResult>,
    pub candidates: Vec<AskResult>, // 二选一候选 or 相近词推荐
    pub recognized_text: Option<String>,
    pub normalized_text: Option<String>,
    pub unmatched_id: Option<String>, // nomatch 时写入的未命中表 id
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Child {
    pub child_id: String,
    pub family_id: String,
    pub child_name: String,
    pub child_birthdate: Option<String>,
    pub age_band_override: Option<String>,
    pub level: i64,
}

/// 年龄分段推导（PRD 1.2）：12~24 月 = A，24~36 月 = B
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum AgeBand {
    A,
    B,
}

impl AgeBand {
    pub fn from_birthdate(birthdate: Option<&str>, now: &DateTime<Utc>) -> Option<AgeBand> {
        let birthdate = birthdate?;
        let bd = chrono::NaiveDate::parse_from_str(birthdate, "%Y-%m-%d").ok()?;
        let today = now.date_naive();
        let months = (today.year() - bd.year()) * 12
            + (today.month() as i32 - bd.month() as i32)
            + if today.day() >= bd.day() { 0 } else { -1 };
        match months {
            m if (12..24).contains(&m) => Some(AgeBand::A),
            m if (24..36).contains(&m) => Some(AgeBand::B),
            _ => None, // 超出 12~36 月范围：不强制分段
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgeBand::A => "A",
            AgeBand::B => "B",
        }
    }
}

/// 复习排期分档（PRD 8.6）：掌握度决定下次推送间隔
pub fn next_review_days(mastery: f64) -> Option<i64> {
    if mastery < 0.3 {
        Some(1)
    } else if mastery < 0.6 {
        Some(3)
    } else if mastery < 0.85 {
        Some(7)
    } else {
        None // ≥0.85 不再主动推送，只在母亲手动翻阅时出现
    }
}
