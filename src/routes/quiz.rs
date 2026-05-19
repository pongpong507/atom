use crate::{
    db::crud,
    error::{AppError, Result},
    models::SubmitAnswerRequest,
    services::{question_gen, quiz_evaluator, vault_scanner},
    AppState, AuthUser,
};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Local;
use serde_json::json;

/// GET /api/quiz/today
/// Returns today's questions (generates them if not yet created).
pub async fn get_today_quiz(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<impl IntoResponse> {
    let today = Local::now().date_naive();

    let existing = crud::get_questions_for_date(&state.pool, user.user_id, today).await?;
    if !existing.is_empty() {
        return Ok(Json(json!({ "questions": existing, "already_generated": true })));
    }

    let vault_path = std::env::var("VAULT_PATH").unwrap_or_else(|_| "/app/vault".into());
    let notes = vault_scanner::scan_today_notes(&vault_path, today)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if notes.is_empty() {
        return Ok(Json(json!({
            "questions": [],
            "message": "今天還沒有新的筆記，請先在 Obsidian 記錄學習內容後再來作答！"
        })));
    }

    let context = vault_scanner::build_context(&notes);
    let total_words = vault_scanner::count_words(&notes);
    let file_paths: Vec<String> = notes.iter().map(|n| n.path.clone()).collect();

    let (quality, feedback) = question_gen::evaluate_notes(&state.llm, &context).await?;

    crud::upsert_daily_summary(
        &state.pool,
        user.user_id,
        today,
        &file_paths,
        total_words,
        quality,
        &feedback,
    )
    .await?;

    let generated = question_gen::generate_questions(&state.llm, &context).await?;
    crud::save_questions(&state.pool, user.user_id, today, &generated).await?;

    let questions = crud::get_questions_for_date(&state.pool, user.user_id, today).await?;

    Ok(Json(json!({
        "questions": questions,
        "note_quality": quality,
        "note_feedback": feedback,
        "already_generated": false
    })))
}

/// POST /api/quiz/submit
pub async fn submit_answer(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(req): Json<SubmitAnswerRequest>,
) -> Result<impl IntoResponse> {
    let question = crud::get_question(&state.pool, req.question_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Question not found".into()))?;

    let model_answer = question.model_answer.unwrap_or_default();

    let (is_correct, ai_evaluation, score) = match question.question_type.as_str() {
        "choice" => {
            let (ok, s) = quiz_evaluator::evaluate_choice(&req.user_answer, &model_answer);
            (Some(ok), None, s)
        }
        _ => {
            let (eval, s) = quiz_evaluator::evaluate_open(
                &state.llm,
                &question.question_text,
                &req.user_answer,
                &model_answer,
            )
            .await?;
            (None, Some(eval), s)
        }
    };

    crud::save_answer(
        &state.pool,
        req.question_id,
        &req.user_answer,
        is_correct,
        ai_evaluation.as_deref(),
        req.response_time_seconds,
        score,
    )
    .await?;

    Ok(Json(json!({
        "is_correct": is_correct,
        "ai_evaluation": ai_evaluation,
        "score": score
    })))
}
