use crate::error::{AppError, Result};
use crate::models::{GeneratedQuestion, LlmQuestionResponse};
use crate::services::llm_client::{LlmClient, Message};

const SYSTEM_PROMPT: &str = r#"你是一位嚴謹的學習教練。
根據使用者提供的筆記內容，生成 6 道主動召回（Active Recall）題目：
- 2 道四選一選擇題（type: "choice"）
- 2 道簡答題（type: "short"）
- 2 道反思申論題（type: "reflective"）

必須以純 JSON 格式回應，不得包含任何 Markdown 標記或說明文字。
格式如下：
{
  "questions": [
    {
      "type": "choice",
      "text": "題目文字",
      "options": {"A": "選項A", "B": "選項B", "C": "選項C", "D": "選項D"},
      "answer": "A",
      "model_answer": "詳細解釋為何 A 正確"
    },
    {
      "type": "short",
      "text": "題目文字",
      "options": null,
      "answer": null,
      "model_answer": "參考答案"
    },
    {
      "type": "reflective",
      "text": "反思題目文字",
      "options": null,
      "answer": null,
      "model_answer": "評分參考要點"
    }
  ]
}"#;

pub async fn generate_questions(
    llm: &LlmClient,
    notes_context: &str,
) -> Result<Vec<GeneratedQuestion>> {
    let user_msg = format!(
        "以下是今天的學習筆記，請根據內容生成題目：\n\n{}",
        notes_context
    );

    let messages = vec![Message::system(SYSTEM_PROMPT), Message::user(user_msg)];

    // Attempt up to 2 times in case JSON parsing fails
    for attempt in 1..=2 {
        let raw = llm.chat(messages.clone()).await?;

        // Strip possible ```json ``` wrapping
        let json_str = extract_json(&raw);

        match serde_json::from_str::<LlmQuestionResponse>(&json_str) {
            Ok(resp) if !resp.questions.is_empty() => return Ok(resp.questions),
            Ok(_) => {
                tracing::warn!("LLM returned empty questions list (attempt {attempt})");
            }
            Err(e) => {
                tracing::warn!("JSON parse failed (attempt {attempt}): {e}\nRaw: {raw}");
            }
        }
        if attempt == 2 {
            return Err(AppError::LlmJsonParse(
                "LLM did not return valid question JSON after 2 attempts".into(),
            ));
        }
    }
    unreachable!()
}

fn extract_json(raw: &str) -> String {
    // Strip ```json ... ``` fences if present
    let s = raw.trim();
    let s = s.strip_prefix("```json").unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim().to_string()
}

/// Generate a note quality evaluation (rating 1-5 + feedback text)
pub async fn evaluate_notes(
    llm: &LlmClient,
    notes_context: &str,
) -> Result<(i32, String)> {
    let prompt = format!(
        r#"請評估以下筆記的學習質量，並以純 JSON 格式回應（不含任何 Markdown）：
{{"rating": <1到5的整數>, "feedback": "<建議改進的具體文字>"}}

筆記內容：
{notes_context}"#
    );

    let messages = vec![Message::user(prompt)];
    let raw = llm.chat(messages).await?;
    let json_str = extract_json(&raw);

    #[derive(serde::Deserialize)]
    struct NoteEval {
        rating: i32,
        feedback: String,
    }

    let eval: NoteEval = serde_json::from_str(&json_str).map_err(|e| {
        AppError::LlmJsonParse(format!("Note eval parse failed: {e}"))
    })?;

    let rating = eval.rating.clamp(1, 5);
    Ok((rating, eval.feedback))
}
