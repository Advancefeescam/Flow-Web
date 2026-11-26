//! Local notes over the existing Flow schema. Linked-device sync is not available on web.
use crate::{AppState, errors::{AppError, AppResult}, rpc::arg};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, SqliteConnection};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct NoteRow {
    id: String,
    target_id: String,
    kind: String,
    text: String,
    updated_at: i64,
    title: Option<String>,
    channel_name: Option<String>,
    channel_id: Option<String>,
    thumbnail_url: Option<String>,
    duration_seconds: Option<i32>,
    channel_avatar_url: Option<String>,
    channel_handle: Option<String>,
    subscriber_count_text: Option<String>,
    position: Option<i64>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Subject {
    title: Option<String>, channel_name: Option<String>, channel_id: Option<String>,
    thumbnail_url: Option<String>, duration_seconds: Option<i32>, channel_avatar_url: Option<String>,
    channel_handle: Option<String>, subscriber_count_text: Option<String>,
}

fn note_id(kind: &str, target: &str) -> AppResult<String> {
    if !matches!(kind, "Video" | "Channel") || target.trim().is_empty() || target.len() > 256 {
        return Err(AppError::Validation("Invalid note target".into()));
    }
    Ok(format!("{}:{target}", kind.to_ascii_lowercase()))
}

fn check_ids(ids: &[String]) -> AppResult<()> {
    if ids.len() > 2000 || ids.iter().any(|id| id.len() > 264 || !(id.starts_with("video:") || id.starts_with("channel:"))) {
        return Err(AppError::Validation("Invalid note IDs".into()));
    }
    Ok(())
}

impl NoteRow {
    fn set_subject(&mut self, s: Subject) {
        self.title = s.title;
        self.channel_name = s.channel_name;
        self.channel_id = s.channel_id;
        self.thumbnail_url = s.thumbnail_url;
        self.duration_seconds = s.duration_seconds;
        self.channel_avatar_url = s.channel_avatar_url;
        self.channel_handle = s.channel_handle;
        self.subscriber_count_text = s.subscriber_count_text;
    }
}

async fn get(conn: &mut SqliteConnection, id: &str) -> AppResult<Option<NoteRow>> {
    Ok(sqlx::query_as("SELECT * FROM notes WHERE id = ?").bind(id).fetch_optional(conn).await?)
}

async fn write(conn: &mut SqliteConnection, n: &NoteRow) -> AppResult<()> {
    sqlx::query("INSERT INTO notes (id,target_id,kind,text,updated_at,title,channel_name,channel_id,thumbnail_url,duration_seconds,channel_avatar_url,channel_handle,subscriber_count_text,position)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET
        text=excluded.text,updated_at=excluded.updated_at,title=excluded.title,channel_name=excluded.channel_name,channel_id=excluded.channel_id,
        thumbnail_url=excluded.thumbnail_url,duration_seconds=excluded.duration_seconds,channel_avatar_url=excluded.channel_avatar_url,
        channel_handle=excluded.channel_handle,subscriber_count_text=excluded.subscriber_count_text,position=excluded.position")
        .bind(&n.id).bind(&n.target_id).bind(&n.kind).bind(&n.text).bind(n.updated_at)
        .bind(&n.title).bind(&n.channel_name).bind(&n.channel_id).bind(&n.thumbnail_url).bind(n.duration_seconds)
        .bind(&n.channel_avatar_url).bind(&n.channel_handle).bind(&n.subscriber_count_text).bind(n.position)
        .execute(conn).await?;
    Ok(())
}

pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    if command == "list_notes" {
        let rows: Vec<NoteRow> = sqlx::query_as("SELECT * FROM notes WHERE kind IN ('Video','Channel') ORDER BY updated_at DESC,id")
            .fetch_all(&state.pool).await?;
        return Ok(Some(json!(rows)));
    }
    if !matches!(command, "save_note" | "save_note_subject" | "delete_notes" | "save_notes_order" | "restore_notes") {
        return Ok(None);
    }
    let _write = state.writes.lock().await;
    let mut tx = state.pool.begin().await?;
    let value = match command {
        "save_note" | "save_note_subject" => {
            let kind: String = arg(args, "kind")?;
            let target: String = arg(args, "targetId")?;
            let id = note_id(&kind, &target)?;
            let stored = get(&mut tx, &id).await?;
            let subject: Option<Subject> = arg(args, "subject")?;
            if command == "save_note_subject" {
                if let (Some(mut row), Some(subject)) = (stored, subject) {
                    row.set_subject(subject);
                    write(&mut tx, &row).await?;
                }
                Value::Null
            } else {
                let text: String = arg(args, "text")?;
                if text.len() > 65536 { return Err(AppError::Validation("Note too large".into())); }
                if text.trim().is_empty() {
                    sqlx::query("DELETE FROM notes WHERE id=?").bind(&id).execute(&mut *tx).await?;
                    Value::Null
                } else {
                    let mut row = stored.unwrap_or_else(|| NoteRow {
                        id, target_id: target, kind, text: String::new(), updated_at: 0,
                        title: None, channel_name: None, channel_id: None, thumbnail_url: None, duration_seconds: None,
                        channel_avatar_url: None, channel_handle: None, subscriber_count_text: None, position: None,
                    });
                    row.text = text.trim().to_owned();
                    row.updated_at = chrono::Utc::now().timestamp_millis().max(row.updated_at.saturating_add(1));
                    if let Some(subject) = subject { row.set_subject(subject); }
                    write(&mut tx, &row).await?;
                    json!(row)
                }
            }
        },
        "delete_notes" | "save_notes_order" => {
            let ids: Vec<String> = arg(args, "ids")?;
            check_ids(&ids)?;
            for (position, id) in ids.iter().enumerate() {
                if command == "delete_notes" {
                    sqlx::query("DELETE FROM notes WHERE id=?").bind(id).execute(&mut *tx).await?;
                } else {
                    sqlx::query("UPDATE notes SET position=? WHERE id=?").bind(position as i64).bind(id).execute(&mut *tx).await?;
                }
            }
            Value::Null
        },
        "restore_notes" => {
            let mut rows: Vec<NoteRow> = arg(args, "notes")?;
            if rows.len() > 2000 { return Err(AppError::Validation("Too many notes".into())); }
            for row in &mut rows {
                if row.id != note_id(&row.kind, &row.target_id)? || row.text.trim().is_empty() || row.text.len() > 65536 {
                    return Err(AppError::Validation("Invalid restored note".into()));
                }
                row.updated_at = chrono::Utc::now().timestamp_millis().max(row.updated_at.saturating_add(1));
                write(&mut tx, row).await?;
            }
            json!(rows)
        },
        _ => unreachable!(),
    };
    tx.commit().await?;
    Ok(Some(value))
}
