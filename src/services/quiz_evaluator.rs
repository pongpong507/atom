use crate::error::Result;
use crate::services::llm_client::{LlmClient, Message};

/// Evaluate a multiple-choice answer. Returns (is_correct, score).
pub fn evaluate_choice(user_answer: &str, correct_answer: &str) -> (bool, i32) {
    let correct = user_answer.trim().to_uppercase() == correct_answer.trim().to_uppercase();
    let score = if correct { 100 } else { 0 };
    (correct, score)
}

/// Evaluate an open-ended or reflective answer via LLM.
/// Returns (ai_evaluation_text, score_0_to_100).
pub async fn evaluate_open(
    llm: &LlmClient,
    question: &str,
    user_answer: &str,
    model_answer: &str,
) -> Result<(String, i32)> {
    let prompt = format!(
        r#"你是一位嚴格但友善的學習評審。請評估學習者的回答，並以純 JSON 格式回應（不含 Markdown）：
{{"score": <0到100的整數>, "evaluation": "<繁體中文評語，指出優點與不足，並給出改進建議>"}}

題目：{question}

參考答案要點：{model_answer}

學習者的回答：{user_answer}"#
    );

    let messages = vec![Message::user(prompt)];
    let raw = llm.chat(messages).await?;

    // Strip ```json fences
    let s = raw.trim();
    let s = s.strip_prefix("```json").unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    let json_str = s.trim();

    #[derive(serde::Deserialize)]
    struct EvalResult {
        score: i32,
        evaluation: String,
    }

    match serde_json::from_str::<EvalResult>(json_str) {
        Ok(result) => {
            let score = result.score.clamp(0, 100);
            Ok((result.evaluation, score))
        }
        Err(e) => {
            tracing::warn!("Open eval JSON parse failed: {e}. Raw: {raw}");
            // Graceful fallback: return raw text and a neutral score
            Ok((raw.trim().to_string(), 50))
        }
    }
}

/// Generate a weekly/monthly AI mentor insight from aggregated stats
pub async fn generate_insights(
    llm: &LlmClient,
    stats_summary: &str,
) -> Result<String> {
    let prompt = format!(
        r#"你是一位關心學生成長的學習導師。根據以下學習數據摘要，給出一段繁體中文的個人化學習評語，
指出最近表現好的地方、需要加強的知識點，以及具體可執行的改進建議：

{stats_summary}"#
    );

    let messages = vec![Message::user(prompt)];
    llm.chat(messages).await
}
