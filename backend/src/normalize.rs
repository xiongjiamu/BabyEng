//! L0 输入归一（PRD 4.1.1）：
//! 去标点与语气词 → 剥离「英语怎么说」等提问框架 → 去指示代词与「XX 的」所有格 → 去量词

/// 剥离提问框架。返回 (剩余文本, 是否剥离成功)
fn strip_question_frame(s: &str) -> (String, bool) {
    let frames = [
        "用英语怎么说",
        "用英文怎么说",
        "英语怎么说",
        "英文怎么说",
        "英语怎么读",
        "英文怎么读",
        "怎么说英语",
        "怎么说英文",
        "英语是什么",
        "英文是什么",
        "英语叫什么",
        "英文叫什么",
        "是什么东西",
        "是什么意思",
        "是什么",
        "叫什么",
        "怎么说",
        "怎么读",
        "英语里怎么说",
        "英文里怎么说",
        "用英语讲",
        "用英语",
        "英文",
        "英语",
    ];
    for f in frames {
        if let Some(pos) = s.find(f) {
            // 框架出现在末尾（"杯子用英语怎么说"）→ 取框架之前
            // 框架出现在开头（"英语怎么说的杯子"）→ 取框架之后
            let rest = if pos + f.len() >= s.len() {
                s[..pos].trim().to_string()
            } else {
                s[pos + f.len()..].trim().to_string()
            };
            return (rest, true);
        }
    }
    (s.to_string(), false)
}

/// 去指示代词、所有格前缀（"这个"/"那个"/"我的"/"妈妈的"等）
/// 注意：数字+量词组合（"一个苹果"）刻意保留——那是 L4 组合查询信号，不剥离避免误命中单词
fn strip_noise(s: &str) -> String {
    let mut out = s.to_string();
    let prefixes = [
        "这个", "那个", "这些", "那些", "这边", "那边", "这里", "那里", "我的", "你的", "咱们的",
        "妈妈的", "爸爸的", "宝宝的", "小孩子的",
    ];
    loop {
        let mut changed = false;
        for p in &prefixes {
            if let Some(rest) = out.strip_prefix(p) {
                out = rest.trim_start().to_string();
                changed = true;
            }
        }
        // 剥离任意「称谓+的」结构（如 妈妈的手机 → 手机；杯子的 → 杯子）
        if let Some(pos) = out.find('的') {
            let before = out[..pos].trim().to_string();
            let after = out[pos + 1..].trim().to_string();
            let rel = ["妈妈", "爸爸", "宝宝", "爷爷", "奶奶", "哥哥", "姐姐", "弟弟", "妹妹", "我的", "你的"];
            if rel.iter().any(|r| before == *r) && !after.is_empty() {
                out = after;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out.trim().to_string()
}

/// 去标点与纯语气词。
/// 注意：`了`/`啦`/`吧`/`呀` 是短句的常见组成部分（"该睡觉了""起床啦"），不能删
fn strip_punct(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(
                    *c,
                    '啊' | '呢' | '哦' | '嗯' | '哈' | '嘛' | '么' | '诶' | '唉' | '哎' | '嘞'
                        | '，' | '。' | '？' | '！' | '、' | '；' | '：' | ',' | '.' | '?' | '!'
                )
        })
        .collect()
}

/// 数字统一：汉字数字 → 阿拉伯数字（便于与别名匹配）
fn unify_digits(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let d = match c {
            '一' => '1',
            '二' => '2',
            '两' => '2',
            '三' => '3',
            '四' => '4',
            '五' => '5',
            '六' => '6',
            '七' => '7',
            '八' => '8',
            '九' => '9',
            '零' => '0',
            other => other,
        };
        out.push(d);
    }
    out
}

/// L0 归一主入口：返回归一后的 query（也可能为空——纯语气词输入）
pub fn normalize(raw: &str) -> String {
    let (s1, _) = strip_question_frame(raw);
    let s2 = strip_punct(&s1);
    let s3 = strip_noise(&s2);
    let s4 = unify_digits(&s3);
    s4.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frames() {
        assert_eq!(normalize("杯子英语怎么说"), "杯子");
        assert_eq!(normalize("用英语怎么说杯子"), "杯子");
        assert_eq!(normalize("爷爷"), "爷爷");
        assert_eq!(normalize("该睡觉了"), "该睡觉了");
        assert_eq!(normalize("起床啦"), "起床啦");
        assert_eq!(normalize("杯子是什么"), "杯子");
    }

    #[test]
    fn test_noise() {
        assert_eq!(normalize("这个杯子是什么"), "杯子");
        assert_eq!(normalize("妈妈的手机"), "手机");
        // 数字+量词组合保留（L4 组合查询信号，避免误命中单词）
        assert_eq!(normalize("三个苹果"), "3个苹果");
        assert_eq!(normalize("嗯嗯，杯子吧"), "杯子吧");
    }
}
