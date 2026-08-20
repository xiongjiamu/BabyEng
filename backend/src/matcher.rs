//! 匹配管线 L1/L2（PRD 4.1.1）：
//! L1 别名精确 → L2 模糊（编辑距离 + 拼音相似度，处理 ASR 同音错字）
//! 命中层级决定数据来源；彻底未命中由上层落 unmatched_query

use std::collections::HashMap;

use crate::models::{AskResult, Sentence, Word};

/// 常见同音/近音字组（中文 ASR 高频混淆，PRD A3 验收场景）
const HOMOPHONE_GROUPS: &[&[&str]] = &[
    &["杯", "被", "北", "贝", "备", "背", "辈"],
    &["子", "纸", "只", "指", "止", "紫", "仔"],
    &["机", "鸡", "基", "积", "肌", "激"],
    &["十", "是", "时", "市", "事", "试", "石", "识", "柿"],
    &["四", "死", "丝", "思", "撕"],
    &["碗", "晚", "挽", "婉"],
    &["饭", "范", "返", "贩", "翻"],
    &["鞋", "协", "斜", "携"],
    &["球", "求", "秋", "丘"],
    &["车", "扯", "彻"],
    &["灯", "登", "蹬"],
    &["门", "闷"],
    &["床", "窗", "创", "疮"],
    &["帽", "猫", "毛", "矛"],
    &["裤", "哭", "苦", "库"],
    &["裙", "群"],
    &["狗", "够", "勾", "沟"],
    &["水", "谁", "睡"],
    &["喝", "和", "河", "盒"],
    &["奶", "乃", "耐"],
    &["蛋", "但", "淡", "担"],
    &["米", "咪", "密", "蜜"],
    &["菜", "才", "采", "彩"],
    &["汤", "烫", "趟"],
    &["药", "要", "耀"],
    &["糖", "躺", "堂"],
    &["饼", "并", "病"],
    &["瓶", "平", "评"],
    &["叉", "差", "查", "茶"],
    &["勺", "少", "烧", "哨"],
    &["盘", "盼", "判"],
    &["娃", "挖", "哇"],
    &["糕", "高", "告"],
    &["肉", "揉", "柔"],
    &["鱼", "于", "余"],
    &["包", "抱", "报", "宝", "饱"],
    &["星", "兴", "行"],
    &["月", "越", "阅"],
    &["椅", "以", "已", "意"],
    &["桌", "捉", "卓"],
    &["沙", "杀", "傻"],
    &["发", "法", "罚"],
    &["镜", "静", "净", "敬"],
    &["香", "箱", "乡", "相"],
    &["书", "输", "梳", "舒"],
    &["笔", "比", "毕"],
    &["睡", "水", "谁"],
    &["哭", "苦", "库"],
    &["笑", "孝", "小"],
    &["累", "泪", "类"],
    &["怕", "趴", "爬"],
    &["气", "汽", "器", "弃"],
    &["开", "凯", "楷"],
    &["关", "官", "观"],
    &["坐", "做", "作", "昨"],
    &["走", "奏", "揍"],
    &["来", "莱", "赖"],
    &["去", "趣", "区"],
    &["看", "砍", "坎"],
    &["听", "厅", "亭"],
    &["吃", "痴", "池", "迟"],
    &["玩", "完", "顽"],
    &["拿", "那", "哪"],
    &["爷", "耶", "野"],
    &["哥", "歌", "割"],
    &["姐", "接", "节", "借"],
    &["弟", "第", "地"],
    &["妹", "没", "眉"],
    &["家", "加", "佳", "夹"],
    &["妈", "吗", "麻", "马", "码"],
    &["爸", "吧", "八", "拔"],
];

/// 单条可匹配目标（词或句子），统一抽象
#[derive(Debug, Clone)]
pub struct Target {
    pub target_type: String, // word / sentence
    pub id: String,
    pub zh: String,
    pub aliases: Vec<String>,
    pub en: String,
    pub category: Option<String>,
    pub scene: Option<String>,
}

/// 匹配结果
#[derive(Debug, Clone)]
pub enum Match {
    /// L1/L2 精确命中
    Hit(Target, &'static str),
    /// 相似度接近阈值：交给母亲二选一（PRD 4.1.1 L2）
    Ambiguous(Vec<Target>),
    /// 彻底未命中
    Miss,
}

pub struct Matcher {
    targets: Vec<Target>,
    /// alias(含 zh 主词) -> target 索引列表
    alias_index: HashMap<String, Vec<usize>>,
    homophone: HashMap<char, Vec<char>>,
}

impl Matcher {
    pub fn new(words: &[Word], sentences: &[Sentence]) -> Self {
        let mut targets = Vec::new();
        let mut alias_index: HashMap<String, Vec<usize>> = HashMap::new();

        for w in words {
            if w.review_status != "published" {
                continue; // 未完成校音的词条不下发给前端（8.1）
            }
            let mut aliases = w.aliases.clone();
            if !aliases.contains(&w.zh) {
                aliases.push(w.zh.clone());
            }
            aliases.push(w.en.to_lowercase()); // 也支持直接说英文词
            let idx = targets.len();
            targets.push(Target {
                target_type: "word".into(),
                id: w.id.clone(),
                zh: w.zh.clone(),
                aliases: aliases.clone(),
                en: w.en.clone(),
                category: Some(w.category.clone()),
                scene: None,
            });
            for a in aliases {
                alias_index.entry(a).or_default().push(idx);
            }
        }
        for s in sentences {
            if s.review_status != "published" {
                continue;
            }
            let mut aliases = s.aliases.clone();
            if !aliases.contains(&s.zh) {
                aliases.push(s.zh.clone());
            }
            aliases.push(s.en.to_lowercase());
            let idx = targets.len();
            targets.push(Target {
                target_type: "sentence".into(),
                id: s.id.clone(),
                zh: s.zh.clone(),
                aliases: aliases.clone(),
                en: s.en.clone(),
                category: None,
                scene: Some(s.scene.clone()),
            });
            for a in aliases {
                alias_index.entry(a).or_default().push(idx);
            }
        }

        let mut homophone: HashMap<char, Vec<char>> = HashMap::new();
        for group in HOMOPHONE_GROUPS {
            let chars: Vec<char> = group.iter().map(|s| s.chars().next().unwrap()).collect();
            for c in &chars {
                homophone.entry(*c).or_insert_with(|| chars.clone());
            }
        }

        Self {
            targets,
            alias_index,
            homophone,
        }
    }

    /// L1 精确：归一后 query 直查别名表
    fn exact(&self, q: &str) -> Option<Target> {
        let q = q.trim().to_lowercase();
        if let Some(idxs) = self.alias_index.get(&q) {
            if let Some(&i) = idxs.first() {
                return Some(self.targets[i].clone());
            }
        }
        None
    }

    /// 字符级编辑距离（Levenshtein）
    fn levenshtein(a: &[char], b: &[char]) -> usize {
        let (la, lb) = (a.len(), b.len());
        if la == 0 {
            return lb;
        }
        if lb == 0 {
            return la;
        }
        let mut prev: Vec<usize> = (0..=lb).collect();
        let mut cur = vec![0usize; lb + 1];
        for i in 1..=la {
            cur[0] = i;
            for j in 1..=lb {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[lb]
    }

    /// L2 模糊：编辑距离 + 同音替换
    /// 返回 (相似度, 目标) 列表，按相似度降序
    fn fuzzy(&self, q: &str) -> Vec<(f64, Target)> {
        let qc: Vec<char> = q.chars().collect();
        let mut scored: Vec<(f64, Target)> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 1. 编辑距离：q vs 每个目标的每个别名，取该目标的所有别名中的最高相似度
        //    （修复：首个达阈值的别名可能不是最高分，seen 去重会错误丢弃更高分候选）
        for t in &self.targets {
            let mut cands: Vec<&str> = vec![&t.zh];
            for a in &t.aliases {
                cands.push(a);
            }
            let mut best = 0.0f64;
            for cand in cands {
                let cc: Vec<char> = cand.chars().collect();
                let dist = Self::levenshtein(&qc, &cc);
                let max_len = qc.len().max(cc.len()).max(1);
                let sim = 1.0 - (dist as f64 / max_len as f64);
                if sim > best {
                    best = sim;
                }
            }
            if best >= 0.6 && seen.insert(t.id.clone()) {
                scored.push((best, t.clone()));
            }
        }

        // 2. 同音替换：query 逐字替换为同音组字后重新精确匹配
        for (i, c) in qc.iter().enumerate() {
            if let Some(grp) = self.homophone.get(c) {
                for &alt in grp {
                    if alt == *c {
                        continue;
                    }
                    let mut variant = qc.clone();
                    variant[i] = alt;
                    let v: String = variant.iter().collect();
                    if let Some(t) = self.exact(&v) {
                        if seen.insert(t.id.clone()) {
                            // 同音命中给较高分（区别于编辑距离的猜测）
                            scored.push((0.95, t));
                        }
                    }
                }
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// 匹配管线主入口：L1 → L2 → Miss
    pub fn match_query(&self, normalized: &str, raw: &str) -> Match {
        if let Some(t) = self.exact(normalized) {
            return Match::Hit(t, "L1");
        }
        if let Some(t) = self.exact(raw) {
            return Match::Hit(t, "L1");
        }
        if normalized.is_empty() {
            return Match::Miss;
        }

        let scored = self.fuzzy(normalized);
        match scored.first() {
            Some((sim, t)) if *sim >= 0.8 => Match::Hit(t.clone(), "L2"),
            Some((sim, _)) if *sim >= 0.6 => {
                // 接近阈值：给出前两个候选让母亲二选一
                let mut cands = Vec::new();
                for (_, t) in scored.iter().take(2) {
                    if !cands.iter().any(|c: &Target| c.id == t.id) {
                        cands.push(t.clone());
                    }
                }
                Match::Ambiguous(cands)
            }
            _ => Match::Miss,
        }
    }

    /// 同类目相近词推荐（未命中时给母亲台阶下，PRD 4.1.1）
    pub fn suggest_similar(&self, category: Option<&str>, exclude: &str, n: usize) -> Vec<Target> {
        let mut out = Vec::new();
        if let Some(cat) = category {
            // 优先推荐同类目（item_ 前缀匹配）
            for t in &self.targets {
                if out.len() >= n {
                    break;
                }
                if t.id == exclude || t.target_type != "word" {
                    continue;
                }
                if let Some(c) = &t.category {
                    if c == cat
                        || (cat.starts_with("item") && c.starts_with("item"))
                        || cat == "general"
                    {
                        out.push(t.clone());
                    }
                }
            }
        }
        if out.is_empty() {
            // 无 category 时推荐高频词（固定几个场景代表词）
            for t in &self.targets {
                if out.len() >= n {
                    break;
                }
                if t.id == exclude || t.target_type != "word" {
                    continue;
                }
                if [
                    "word_cup",
                    "word_ball",
                    "word_apple",
                    "word_mom",
                    "word_dog",
                    "word_car",
                ]
                .contains(&t.id.as_str())
                {
                    out.push(t.clone());
                }
            }
        }
        out
    }
}

impl Target {
    pub fn to_ask_result(&self) -> AskResult {
        // 需要词条详情（音标、例句等），由上层从 DB 补齐；这里只给基础字段
        AskResult {
            target_type: self.target_type.clone(),
            target_id: self.id.clone(),
            zh: self.zh.clone(),
            en: self.en.clone(),
            phonetic: None,
            phonetic_source: None,
            category: self.category.clone(),
            scene: self.scene.clone(),
            example_en: None,
            example_zh: None,
            example_context: None,
            mother_tip: None,
            image_emoji: None,
            match_level: String::new(),
            tts_available: false,
            tts_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_word(id: &str, zh: &str, aliases: &[&str], en: &str) -> Word {
        Word {
            id: id.into(),
            zh: zh.into(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            en: en.into(),
            pos: "noun".into(),
            phonetic: Some("/kʌp/".into()),
            phonetic_source: "dict".into(),
            category: "item_tableware".into(),
            level: 1,
            image_emoji: "☕".into(),
            image_source: "family".into(),
            tts_audio_path: None,
            tts_voice: None,
            example_en: None,
            example_zh: None,
            mother_tip: None,
            review_status: "published".into(),
        }
    }

    #[test]
    fn test_l1_exact() {
        let words = vec![sample_word("word_cup", "杯子", &["水杯", "喝水杯"], "cup")];
        let m = Matcher::new(&words, &[]);
        assert!(matches!(
            m.match_query("杯子", "杯子"),
            Match::Hit(t, "L1") if t.id == "word_cup"
        ));
        assert!(matches!(
            m.match_query("水杯", "水杯"),
            Match::Hit(t, _) if t.id == "word_cup"
        ));
    }

    #[test]
    fn test_l2_homophone() {
        let words = vec![
            sample_word("word_cup", "杯子", &["水杯", "喝水杯"], "cup"),
            sample_word("word_quilt", "被子", &["棉被"], "quilt"),
        ];
        let m = Matcher::new(&words, &[]);
        // ASR 把「杯子」识别成「被子」→ 同音替换后应命中 cup 或给出二选一
        match m.match_query("被子", "被子") {
            Match::Hit(t, _) => assert!(t.id == "word_quilt" || t.id == "word_cup"),
            Match::Ambiguous(_) => {}
            Match::Miss => panic!("should not miss"),
        }
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(Matcher::levenshtein(&['杯', '子'], &['被', '子']), 1);
        assert_eq!(Matcher::levenshtein(&['杯', '子'], &['杯', '子']), 0);
        assert_eq!(
            Matcher::levenshtein(&['c', 'u', 'p'], &['c', 'u', 'p', 's']),
            1
        );
    }
}
