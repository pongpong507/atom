// All queries use runtime sqlx (no macros) so the project compiles
// without a live DATABASE_URL at build time — safe for Docker builds.

use crate::models::{
    DailyNotesSummary, DailyStats, GeneratedQuestion, Question, Session, User,
};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// ─── Users ───────────────────────────────────────────────

pub async fn create_user(pool: &PgPool, username: &str, password_hash: &str) -> sqlx::Result<i32> {
    let row = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING user_id")
        .bind(username)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;
    Ok(row.get("user_id"))
}

pub async fn find_user_by_username(pool: &PgPool, username: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT user_id, username, password_hash, created_at FROM users WHERE username = $1"
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_id(pool: &PgPool, user_id: i32) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT user_id, username, password_hash, created_at FROM users WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

// ─── Sessions ────────────────────────────────────────────

pub async fn create_session(pool: &PgPool, user_id: i32) -> sqlx::Result<String> {
    let session_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);
    sqlx::query("INSERT INTO sessions (session_id, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&session_id)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(session_id)
}

pub async fn get_session(pool: &PgPool, session_id: &str) -> sqlx::Result<Option<Session>> {
    sqlx::query_as::<_, Session>(
        "SELECT session_id, user_id, expires_at FROM sessions
         WHERE session_id = $1 AND expires_at > NOW()"
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_session(pool: &PgPool, session_id: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE session_id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ─── Daily Notes Summary ─────────────────────────────────

pub async fn upsert_daily_summary(
    pool: &PgPool,
    user_id: i32,
    date: NaiveDate,
    file_paths: &[String],
    total_words: i32,
    quality_rating: i32,
    feedback: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"INSERT INTO daily_notes_summary
             (user_id, record_date, file_paths, total_words, note_quality_rating, note_feedback)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (user_id, record_date) DO UPDATE SET
             file_paths          = EXCLUDED.file_paths,
             total_words         = EXCLUDED.total_words,
             note_quality_rating = EXCLUDED.note_quality_rating,
             note_feedback       = EXCLUDED.note_feedback"#
    )
    .bind(user_id)
    .bind(date)
    .bind(file_paths)
    .bind(total_words)
    .bind(quality_rating)
    .bind(feedback)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_daily_summary(
    pool: &PgPool,
    user_id: i32,
    date: NaiveDate,
) -> sqlx::Result<Option<DailyNotesSummary>> {
    sqlx::query_as::<_, DailyNotesSummary>(
        r#"SELECT note_id, user_id, record_date, file_paths, total_words,
                  note_quality_rating, note_feedback, created_at
           FROM daily_notes_summary
           WHERE user_id = $1 AND record_date = $2"#
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(pool)
    .await
}

// ─── Questions ───────────────────────────────────────────

pub async fn save_questions(
    pool: &PgPool,
    user_id: i32,
    date: NaiveDate,
    questions: &[GeneratedQuestion],
) -> sqlx::Result<Vec<i32>> {
    let mut ids = Vec::new();
    for q in questions {
        let options_json = q.options.as_ref().map(|o| json!(o));
        // For choice questions, model_answer stores the correct letter (A/B/C/D).
        // For open-ended questions, model_answer stores the reference answer text.
        let stored_answer = if q.question_type == "choice" {
            q.answer.as_deref().unwrap_or("A").to_string()
        } else {
            q.model_answer.clone()
        };
        let row = sqlx::query(
            r#"INSERT INTO questions
                 (user_id, record_date, question_type, question_text, options, model_answer)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING question_id"#
        )
        .bind(user_id)
        .bind(date)
        .bind(&q.question_type)
        .bind(&q.text)
        .bind(options_json)
        .bind(&stored_answer)
        .fetch_one(pool)
        .await?;
        ids.push(row.get::<i32, _>("question_id"));
    }
    Ok(ids)
}

pub async fn get_questions_for_date(
    pool: &PgPool,
    user_id: i32,
    date: NaiveDate,
) -> sqlx::Result<Vec<Question>> {
    sqlx::query_as::<_, Question>(
        r#"SELECT question_id, user_id, record_date, question_type,
                  question_text, options, model_answer, created_at
           FROM questions
           WHERE user_id = $1 AND record_date = $2
           ORDER BY question_id"#
    )
    .bind(user_id)
    .bind(date)
    .fetch_all(pool)
    .await
}

pub async fn get_question(pool: &PgPool, question_id: i32) -> sqlx::Result<Option<Question>> {
    sqlx::query_as::<_, Question>(
        r#"SELECT question_id, user_id, record_date, question_type,
                  question_text, options, model_answer, created_at
           FROM questions WHERE question_id = $1"#
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
}

// ─── Answers ─────────────────────────────────────────────

pub async fn save_answer(
    pool: &PgPool,
    question_id: i32,
    user_answer: &str,
    is_correct: Option<bool>,
    ai_evaluation: Option<&str>,
    response_time_seconds: i32,
    score: i32,
) -> sqlx::Result<i32> {
    let row = sqlx::query(
        r#"INSERT INTO user_answers
             (question_id, user_answer, is_correct, ai_evaluation, response_time_seconds, score)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING answer_id"#
    )
    .bind(question_id)
    .bind(user_answer)
    .bind(is_correct)
    .bind(ai_evaluation)
    .bind(response_time_seconds)
    .bind(score)
    .fetch_one(pool)
    .await?;
    Ok(row.get("answer_id"))
}

// ─── Dashboard / Reports ─────────────────────────────────

pub async fn get_daily_stats(
    pool: &PgPool,
    user_id: i32,
    days: i32,
) -> sqlx::Result<Vec<DailyStats>> {
    sqlx::query_as::<_, DailyStats>(
        r#"SELECT
               q.record_date,
               AVG(ua.score)::float8                              AS avg_score,
               COUNT(ua.answer_id) FILTER (WHERE ua.is_correct)  AS correct_count,
               COUNT(ua.answer_id)                               AS total_answered,
               AVG(ua.response_time_seconds)::float8             AS avg_response_time,
               dns.note_quality_rating,
               dns.total_words
           FROM questions q
           LEFT JOIN user_answers ua ON ua.question_id = q.question_id
           LEFT JOIN daily_notes_summary dns
                  ON dns.user_id = q.user_id AND dns.record_date = q.record_date
           WHERE q.user_id = $1
             AND q.record_date >= CURRENT_DATE - ($2 * INTERVAL '1 day')
           GROUP BY q.record_date, dns.note_quality_rating, dns.total_words
           ORDER BY q.record_date"#
    )
    .bind(user_id)
    .bind(days)
    .fetch_all(pool)
    .await
}
