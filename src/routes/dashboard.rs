use crate::{
    db::crud,
    error::Result,
    services::quiz_evaluator,
    AppState, AuthUser,
};
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ReportQuery {
    #[serde(default = "default_days")]
    pub days: i32,
}
fn default_days() -> i32 { 7 }

/// GET /api/dashboard/stats?days=7
pub async fn get_stats(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ReportQuery>,
) -> Result<impl IntoResponse> {
    let days = q.days.clamp(1, 90);
    let stats = crud::get_daily_stats(&state.pool, user.user_id, days).await?;
    Ok(Json(json!({ "stats": stats })))
}

/// POST /api/dashboard/insights?days=7
/// Calls Ollama to generate a text-based learning summary
pub async fn get_insights(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ReportQuery>,
) -> Result<impl IntoResponse> {
    let days = q.days.clamp(1, 90);
    let stats = crud::get_daily_stats(&state.pool, user.user_id, days).await?;

    if stats.is_empty() {
        return Ok(Json(json!({ "insights": "暫無足夠的數據來生成分析，請先完成至少一天的問答！" })));
    }

    // Build a plain-text summary for the LLM
    let mut summary = format!("最近 {days} 天學習數據摘要：\n");
    for s in &stats {
        summary.push_str(&format!(
            "- {}: 答題 {} 題，正確 {} 題，平均分 {:.0}，筆記質量 {}/5，字數 {}\n",
            s.record_date,
            s.total_answered.unwrap_or(0),
            s.correct_count.unwrap_or(0),
            s.avg_score.unwrap_or(0.0),
            s.note_quality_rating.unwrap_or(0),
            s.total_words.unwrap_or(0)
        ));
    }

    let insights = quiz_evaluator::generate_insights(&state.llm, &summary).await?;
    Ok(Json(json!({ "insights": insights })))
}
