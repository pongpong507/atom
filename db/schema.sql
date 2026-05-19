-- Atom DB Schema
-- Run via setup.sh on Ubuntu PostgreSQL machine

CREATE TABLE IF NOT EXISTS users (
    user_id   SERIAL PRIMARY KEY,
    username  VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id VARCHAR(64) PRIMARY KEY,
    user_id    INT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS daily_notes_summary (
    note_id             SERIAL PRIMARY KEY,
    user_id             INT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    record_date         DATE NOT NULL,
    file_paths          TEXT[],
    total_words         INT,
    note_quality_rating INT CHECK (note_quality_rating BETWEEN 1 AND 5),
    note_feedback       TEXT,
    created_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, record_date)
);

CREATE TABLE IF NOT EXISTS questions (
    question_id   SERIAL PRIMARY KEY,
    user_id       INT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    record_date   DATE NOT NULL,
    question_type VARCHAR(20) NOT NULL CHECK (question_type IN ('choice', 'short', 'reflective')),
    question_text TEXT NOT NULL,
    options       JSONB,
    model_answer  TEXT,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_answers (
    answer_id             SERIAL PRIMARY KEY,
    question_id           INT NOT NULL REFERENCES questions(question_id) ON DELETE CASCADE,
    user_answer           TEXT,
    is_correct            BOOLEAN,
    ai_evaluation         TEXT,
    response_time_seconds INT,
    score                 INT CHECK (score BETWEEN 0 AND 100),
    answered_at           TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_questions_user_date    ON questions(user_id, record_date);
CREATE INDEX IF NOT EXISTS idx_user_answers_question  ON user_answers(question_id);
CREATE INDEX IF NOT EXISTS idx_daily_notes_user_date  ON daily_notes_summary(user_id, record_date);
CREATE INDEX IF NOT EXISTS idx_sessions_user          ON sessions(user_id);
