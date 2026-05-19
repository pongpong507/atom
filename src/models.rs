use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub user_id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Session {
    pub session_id: String,
    pub user_id: i32,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DailyNotesSummary {
    pub note_id: i32,
    pub user_id: i32,
    pub record_date: NaiveDate,
    pub file_paths: Option<Vec<String>>,
    pub total_words: Option<i32>,
    pub note_quality_rating: Option<i32>,
    pub note_feedback: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Question {
    pub question_id: i32,
    pub user_id: i32,
    pub record_date: NaiveDate,
    pub question_type: String,
    pub question_text: String,
    pub options: Option<JsonValue>,
    pub model_answer: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserAnswer {
    pub answer_id: i32,
    pub question_id: i32,
    pub user_answer: Option<String>,
    pub is_correct: Option<bool>,
    pub ai_evaluation: Option<String>,
    pub response_time_seconds: Option<i32>,
    pub score: Option<i32>,
    pub answered_at: DateTime<Utc>,
}

// --- API request/response types ---

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub username: String,
}

/// A single question as returned by the LLM (before DB insert)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedQuestion {
    #[serde(rename = "type")]
    pub question_type: String,
    pub text: String,
    pub options: Option<std::collections::HashMap<String, String>>,
    pub answer: Option<String>,
    pub model_answer: String,
}

/// The full LLM response envelope
#[derive(Debug, Deserialize)]
pub struct LlmQuestionResponse {
    pub questions: Vec<GeneratedQuestion>,
}

/// Payload sent from frontend when user submits an answer
#[derive(Debug, Deserialize)]
pub struct SubmitAnswerRequest {
    pub question_id: i32,
    pub user_answer: String,
    pub response_time_seconds: i32,
}

/// Dashboard stats row (per day)
#[derive(Debug, Serialize, FromRow)]
pub struct DailyStats {
    pub record_date: NaiveDate,
    pub avg_score: Option<f64>,
    pub correct_count: Option<i64>,
    pub total_answered: Option<i64>,
    pub avg_response_time: Option<f64>,
    pub note_quality_rating: Option<i32>,
    pub total_words: Option<i32>,
}
