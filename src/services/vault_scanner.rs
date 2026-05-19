use anyhow::Result;
use chrono::NaiveDate;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct NoteFile {
    pub path: String,
    pub content: String,
}

/// Scans VAULT_PATH for .md files modified on `date`.
/// Returns all matching files with their full text content.
pub async fn scan_today_notes(vault_path: &str, date: NaiveDate) -> Result<Vec<NoteFile>> {
    let mut results = Vec::new();
    scan_dir(PathBuf::from(vault_path), date, &mut results).await?;
    Ok(results)
}

fn is_same_day(mtime: std::time::SystemTime, date: NaiveDate) -> bool {
    let secs = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let dt = chrono::DateTime::from_timestamp(secs, 0)
        .map(|d| d.naive_utc().date())
        .unwrap_or(NaiveDate::MIN);
    dt == date
}

// Recursive async directory walk
async fn scan_dir(dir: PathBuf, date: NaiveDate, results: &mut Vec<NoteFile>) -> Result<()> {
    let mut entries = fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden directories (e.g. .obsidian, .trash)
        if file_name.starts_with('.') {
            continue;
        }

        let meta = fs::metadata(&path).await?;
        if meta.is_dir() {
            // Box the future to avoid infinite-size recursive type
            Box::pin(scan_dir(path, date, results)).await?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(mtime) = meta.modified() {
                if is_same_day(mtime, date) {
                    let content = fs::read_to_string(&path).await.unwrap_or_default();
                    results.push(NoteFile {
                        path: path.to_string_lossy().to_string(),
                        content,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Count approximate words across all notes
pub fn count_words(notes: &[NoteFile]) -> i32 {
    notes
        .iter()
        .map(|n| n.content.split_whitespace().count() as i32)
        .sum()
}

/// Concatenate all note contents for LLM context (truncated to ~8000 chars)
pub fn build_context(notes: &[NoteFile]) -> String {
    let combined: String = notes
        .iter()
        .map(|n| format!("# {}\n{}\n\n", n.path, n.content))
        .collect();

    if combined.len() > 8000 {
        combined[..8000].to_string()
    } else {
        combined
    }
}
