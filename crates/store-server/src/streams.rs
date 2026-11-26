use crate::Pool;

#[derive(Debug, Clone)]
pub struct StreamRow {
    pub id: String,
    pub kind: String,
    pub owner: String,
    pub bot_id: Option<String>,
    pub next_seq: i64,
}

pub async fn load_stream(pool: &Pool, id: &str) -> Result<Option<StreamRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT id, kind, owner, bot_id, next_seq FROM record_streams WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| StreamRow {
        id: r.id,
        kind: r.kind,
        owner: r.owner,
        bot_id: r.bot_id,
        next_seq: r.next_seq,
    }))
}

pub async fn ensure_stream(
    pool: &Pool,
    id: &str,
    kind: &str,
    owner: &str,
    bot_id: Option<&str>,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO record_streams (id, kind, owner, bot_id, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING",
        id,
        kind,
        owner,
        bot_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn max_seq(pool: &Pool, stream_id: &str) -> Result<i64, sqlx::Error> {
    let stream = load_stream(pool, stream_id)
        .await?
        .ok_or(sqlx::Error::Protocol("stream row missing".into()))?;
    stream
        .next_seq
        .checked_sub(1)
        .ok_or(sqlx::Error::Protocol("next_seq below one".into()))
}

pub async fn find_record_by_device(
    pool: &Pool,
    stream_id: &str,
    device_id: &str,
) -> Result<Option<RecordRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records WHERE stream_id = $1 AND device_id = $2",
        stream_id,
        device_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(record_from_row))
}

#[derive(Debug, Clone)]
pub struct RecordRow {
    pub stream_id: String,
    pub seq: i64,
    pub device_id: String,
    pub kind: String,
    pub content: String,
    pub created_at_ms: i64,
    pub offline_sent: bool,
    pub task_id: Option<String>,
    pub task_seq: Option<i64>,
    pub purged: bool,
}

#[derive(Debug, Clone)]
struct RecordFields {
    stream_id: String,
    seq: i64,
    device_id: String,
    kind: String,
    content: String,
    created_at_ms: i64,
    offline_sent: bool,
    task_id: Option<String>,
    task_seq: Option<i64>,
    purged: bool,
}

fn record_from_row(r: RecordFields) -> RecordRow {
    RecordRow {
        stream_id: r.stream_id,
        seq: r.seq,
        device_id: r.device_id,
        kind: r.kind,
        content: r.content,
        created_at_ms: r.created_at_ms,
        offline_sent: r.offline_sent,
        task_id: r.task_id,
        task_seq: r.task_seq,
        purged: r.purged,
    }
}

pub async fn list_after(
    pool: &Pool,
    stream_id: &str,
    after: i64,
    limit: i64,
) -> Result<Vec<RecordRow>, sqlx::Error> {
    let rows = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records \
         WHERE stream_id = $1 AND seq > $2 ORDER BY seq LIMIT $3",
        stream_id,
        after,
        limit,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(record_from_row).collect())
}

pub async fn list_streams_of(pool: &Pool, owner: &str) -> Result<Vec<StreamRow>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT id, kind, owner, bot_id, next_seq FROM record_streams WHERE owner = $1 \
         ORDER BY id",
        owner,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| StreamRow {
            id: r.id,
            kind: r.kind,
            owner: r.owner,
            bot_id: r.bot_id,
            next_seq: r.next_seq,
        })
        .collect())
}

pub struct InsertSpec<'a> {
    pub device_id: &'a str,
    pub kind: &'a str,
    pub content: &'a str,
    pub created_at_ms: i64,
    pub offline_sent: bool,
    pub task_id: Option<&'a str>,
}

pub async fn insert_record(
    pool: &Pool,
    stream_id: &str,
    spec: &InsertSpec<'_>,
) -> Result<i64, sqlx::Error> {
    let task_id = spec.task_id;
    let task_seq = bump_task_seq(pool, task_id).await?;
    let row = sqlx::query!(
        "WITH bumped AS (UPDATE record_streams SET next_seq = next_seq + 1 \
         WHERE id = $1 RETURNING next_seq - 1 AS seq) \
         INSERT INTO records (stream_id, seq, device_id, kind, content, created_at_ms, \
         offline_sent, task_id, task_seq) \
         SELECT $1, b.seq, $2, $3, $4, $5, $6, $7, $8 FROM bumped b RETURNING seq",
        stream_id,
        spec.device_id,
        spec.kind,
        spec.content,
        spec.created_at_ms,
        spec.offline_sent,
        task_id,
        task_seq,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.seq)
}

async fn bump_task_seq(pool: &Pool, task_id: Option<&str>) -> Result<Option<i64>, sqlx::Error> {
    let Some(task_id) = task_id else {
        return Ok(None);
    };
    sqlx::query!(
        "INSERT INTO task_counters (task_id) VALUES ($1) ON CONFLICT (task_id) DO NOTHING",
        task_id,
    )
    .execute(pool)
    .await?;
    let row = sqlx::query!(
        "UPDATE task_counters SET next_seq = next_seq + 1 \
         WHERE task_id = $1 RETURNING next_seq - 1 AS task_seq",
        task_id,
    )
    .fetch_one(pool)
    .await?;
    let seq = row
        .task_seq
        .ok_or(sqlx::Error::Protocol("task counter row missing".into()))?;
    Ok(Some(seq))
}

pub async fn record_at(
    pool: &Pool,
    stream_id: &str,
    seq: i64,
) -> Result<Option<RecordRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        RecordFields,
        "SELECT stream_id, seq, device_id, kind, content, created_at_ms, offline_sent, \
         task_id, task_seq, purged FROM records WHERE stream_id = $1 AND seq = $2",
        stream_id,
        seq,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(record_from_row))
}
