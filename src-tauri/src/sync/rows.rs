use super::{SyncError, SyncResult};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};
#[path = "references.rs"]
mod references;

#[derive(Clone, Copy)]
struct TableSpec {
    name: &'static str,
    key: &'static str,
    refs: &'static [(&'static str, &'static str)],
    excluded: &'static [&'static str],
}

const WORK_REF: &[(&str, &str)] = &[("work_id", "works")];
const QA_TURN_REFS: &[(&str, &str)] = &[("session_id", "qa_sessions")];
const KOL_NOTE_REFS: &[(&str, &str)] = &[
    ("expert_id", "kol_experts"),
    ("work_id", "works"),
    ("inbox_id", "inbox_items"),
];
const KOL_DRAFT_REFS: &[(&str, &str)] = &[("expert_id", "kol_experts")];
const KOL_INSIGHT_REFS: &[(&str, &str)] =
    &[("draft_id", "kol_drafts"), ("expert_id", "kol_experts")];
const KOL_MATERIAL_REFS: &[(&str, &str)] = &[
    ("expert_id", "kol_experts"),
    ("blob_hash", "material_blobs"),
];
const MATERIAL_SEGMENT_REFS: &[(&str, &str)] = &[("material_id", "kol_materials")];
const ACTIVITY_REFS: &[(&str, &str)] = &[("work_id", "works")];
const ANALYSIS_REFS: &[(&str, &str)] = &[];
const BRIEF_REFS: &[(&str, &str)] = &[("analysis_run_id", "analysis_runs")];
const PROPOSAL_REFS: &[(&str, &str)] = &[
    ("analysis_run_id", "analysis_runs"),
    ("work_id", "works"),
    ("suggested_work_id", "works"),
];
const REVIEW_REFS: &[(&str, &str)] = &[("proposal_id", "ai_proposals")];
const MEMORY_REFS: &[(&str, &str)] = &[
    ("suggested_work_id", "works"),
    ("preferred_work_id", "works"),
];
const NO_EXCLUSIONS: &[&str] = &[];
const REPORT_EXCLUSIONS: &[&str] = &["provider_model_id"];
const ACTIVITY_EXCLUSIONS: &[&str] = &["workspace_id", "path", "metadata_json"];
const ANALYSIS_EXCLUSIONS: &[&str] = &["provider_model_id", "brief_id"];
const PROPOSAL_EXCLUSIONS: &[&str] = &["workspace_id"];
const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "works",
        key: "id",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "tasks",
        key: "id",
        refs: WORK_REF,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "waiting_items",
        key: "id",
        refs: WORK_REF,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "calendar_events",
        key: "id",
        refs: WORK_REF,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "inbox_items",
        key: "id",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "resume_points",
        key: "id",
        refs: WORK_REF,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "activity_events",
        key: "id",
        refs: ACTIVITY_REFS,
        excluded: ACTIVITY_EXCLUSIONS,
    },
    TableSpec {
        name: "analysis_runs",
        key: "id",
        refs: ANALYSIS_REFS,
        excluded: ANALYSIS_EXCLUSIONS,
    },
    TableSpec {
        name: "daily_briefs",
        key: "id",
        refs: BRIEF_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "ai_proposals",
        key: "id",
        refs: PROPOSAL_REFS,
        excluded: PROPOSAL_EXCLUSIONS,
    },
    TableSpec {
        name: "review_decisions",
        key: "id",
        refs: REVIEW_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "classification_memories",
        key: "id",
        refs: MEMORY_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "reports",
        key: "id",
        refs: &[],
        excluded: REPORT_EXCLUSIONS,
    },
    TableSpec {
        name: "qa_sessions",
        key: "id",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "qa_turns",
        key: "id",
        refs: QA_TURN_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_experts",
        key: "id",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_notes",
        key: "id",
        refs: KOL_NOTE_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_drafts",
        key: "id",
        refs: KOL_DRAFT_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_insights",
        key: "id",
        refs: KOL_INSIGHT_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "material_blobs",
        key: "hash",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_materials",
        key: "id",
        refs: KOL_MATERIAL_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "material_segments",
        key: "id",
        refs: MATERIAL_SEGMENT_REFS,
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "ai_readable_documents",
        key: "id",
        refs: &[],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "capture_context",
        key: "inbox_id",
        refs: &[("inbox_id", "inbox_items"), ("work_id", "works")],
        excluded: NO_EXCLUSIONS,
    },
    TableSpec {
        name: "kol_actions",
        key: "id",
        refs: &[("draft_id", "kol_drafts"), ("expert_id", "kol_experts")],
        excluded: NO_EXCLUSIONS,
    },
];
const MAX_STATE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct FieldStamp {
    edited_at_ms: i64,
    device_id: String,
    seq: u64,
    #[serde(default)]
    context: BTreeMap<String, u64>,
}

fn causal_context(conn: &Connection) -> SyncResult<BTreeMap<String, u64>> {
    let mut stmt = conn
        .prepare("SELECT device_id,MAX(seq) FROM sync_applied GROUP BY device_id")
        .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
    rows.collect::<Result<_, _>>()
        .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))
}
fn follows(new: &FieldStamp, old: &FieldStamp) -> bool {
    if new.device_id == old.device_id && new.seq == old.seq {
        return false;
    }
    (new.device_id == old.device_id && new.seq > old.seq)
        || new
            .context
            .get(&old.device_id)
            .is_some_and(|seq| *seq >= old.seq)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct RecordState {
    table: String,
    key: String,
    uid: String,
    row: Value,
    deleted: bool,
    fields: BTreeMap<String, FieldStamp>,
    refs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateFile {
    application: String,
    format_version: u32,
    dataset_id: String,
    generation: String,
    device_id: String,
    sequence: u64,
    created_at_ms: i64,
    records: Vec<RecordState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chunks: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSummary {
    pub uploaded: u64,
    pub applied: u64,
    pub pending: u64,
    pub conflicts: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub table_name: String,
    pub row_key: String,
    pub field: String,
    pub local_value: Value,
    pub remote_value: Value,
    pub created_at: i64,
}

pub fn list_conflicts(conn: &Connection) -> SyncResult<Vec<SyncConflict>> {
    let mut statement = conn
        .prepare("SELECT id,table_name,row_key,field,local_value,remote_value,created_at FROM sync_conflicts WHERE resolved_at IS NULL ORDER BY created_at DESC,id")
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            let local: String = row.get(4)?;
            let remote: String = row.get(5)?;
            Ok(SyncConflict {
                id: row.get(0)?,
                table_name: row.get(1)?,
                row_key: row.get(2)?,
                field: row.get(3)?,
                local_value: serde_json::from_str(&local).unwrap_or(Value::Null),
                remote_value: serde_json::from_str(&remote).unwrap_or(Value::Null),
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))
}

pub fn resolve_conflict(conn: &Connection, id: &str, choice: &str) -> SyncResult<()> {
    if !matches!(choice, "local" | "remote") {
        return Err(SyncError::new(
            "SYNC_RESOLUTION_INVALID",
            "请选择保留本机版本或同步文件夹版本",
        ));
    }
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    let conflict = tx.query_row(
        "SELECT table_name,row_key,field,local_value,remote_value FROM sync_conflicts WHERE id=?1 AND resolved_at IS NULL",
        [id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
    ).map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => SyncError::new("SYNC_CONFLICT_STALE", "这条冲突已经处理，请刷新"),
        other => SyncError::new("SYNC_DB_READ", other.to_string()),
    })?;
    let spec = table_spec(&conflict.0)
        .ok_or_else(|| SyncError::new("SYNC_TABLE_UNSUPPORTED", "冲突对应的数据类型无法处理"))?;
    let (mut row, mut fields, deleted) = load_state(&tx, &conflict.0, &conflict.1)?
        .ok_or_else(|| SyncError::new("SYNC_CONFLICT_STALE", "冲突对应的数据已经变化，请刷新"))?;
    if deleted && conflict.2 != "__deleted" {
        return Err(SyncError::new(
            "SYNC_CONFLICT_DELETED",
            "数据已经删除，请先完成删除冲突处理",
        ));
    }
    let selected: Value = serde_json::from_str(if choice == "local" {
        &conflict.3
    } else {
        &conflict.4
    })
    .map_err(|_| SyncError::new("SYNC_CONFLICT_INVALID", "冲突内容无法读取"))?;
    if conflict.2 == "__deleted" {
        let keep_deleted = selected
            .as_bool()
            .ok_or_else(|| SyncError::new("SYNC_CONFLICT_INVALID", "删除选择无效"))?;
        if keep_deleted {
            delete_row(&tx, spec, &conflict.1)?;
        } else {
            apply_row(&tx, spec, &row)?;
        }
        let device: String = tx
            .query_row(
                "SELECT device_id FROM sync_local_state WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let seq = sequence(&tx)?;
        fields.insert(
            "__deleted".into(),
            FieldStamp {
                edited_at_ms: now_ms(),
                device_id: device,
                seq,
                context: causal_context(&tx)?,
            },
        );
        tx.execute("UPDATE sync_row_versions SET deleted=?3,field_versions_json=?4 WHERE table_name=?1 AND row_key=?2",params![conflict.0,conflict.1,keep_deleted,serde_json::to_string(&fields).unwrap()]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
        tx.execute(
            "UPDATE sync_conflicts SET resolved_at=strftime('%s','now'),resolution=?2 WHERE id=?1",
            params![id, choice],
        )
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
        return tx
            .commit()
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()));
    }
    let object = row
        .as_object_mut()
        .ok_or_else(|| SyncError::new("SYNC_ROW_INVALID", "冲突数据格式无效"))?;
    if !object.contains_key(&conflict.2) {
        return Err(SyncError::new(
            "SYNC_CONFLICT_STALE",
            "冲突字段已经不存在，请刷新",
        ));
    }
    object.insert(conflict.2.clone(), selected);
    let seq = sequence(&tx)?;
    let device: String = tx
        .query_row(
            "SELECT device_id FROM sync_local_state WHERE id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    fields.insert(
        conflict.2.clone(),
        FieldStamp {
            edited_at_ms: now_ms(),
            device_id: device.clone(),
            seq,
            context: causal_context(&tx)?,
        },
    );
    apply_row(&tx, spec, &row)?;
    tx.execute(
        "UPDATE sync_row_versions SET row_json=?1,field_versions_json=?2,edited_at_ms=?3,device_id=?4,seq=?5 WHERE table_name=?6 AND row_key=?7",
        params![row.to_string(), serde_json::to_string(&fields).unwrap(), now_ms(), device, seq, conflict.0, conflict.1],
    ).map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    tx.execute(
        "UPDATE sync_conflicts SET resolved_at=strftime('%s','now'),resolution=?2 WHERE id=?1 AND resolved_at IS NULL",
        params![id, choice],
    ).map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    tx.commit()
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn safe_identifier(value: &str) -> SyncResult<String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(SyncError::new("SYNC_SCHEMA_UNSAFE", "同步数据表标识无效"));
    }
    Ok(format!("\"{value}\""))
}

fn sql_to_json(value: rusqlite::types::ValueRef<'_>) -> SyncResult<Value> {
    Ok(match value {
        rusqlite::types::ValueRef::Null => Value::Null,
        rusqlite::types::ValueRef::Integer(value) => value.into(),
        rusqlite::types::ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| SyncError::new("SYNC_VALUE_INVALID", "数值无法同步"))?,
        rusqlite::types::ValueRef::Text(value) => {
            Value::String(String::from_utf8_lossy(value).into_owned())
        }
        rusqlite::types::ValueRef::Blob(_) => {
            return Err(SyncError::new(
                "SYNC_BLOB_IN_DATABASE",
                "数据库二进制内容需要通过附件通道同步",
            ))
        }
    })
}

fn json_to_sql(value: &Value) -> SyncResult<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(value) => SqlValue::Integer(i64::from(*value)),
        Value::Number(value) if value.is_i64() => SqlValue::Integer(value.as_i64().unwrap()),
        Value::Number(value) if value.is_u64() => {
            let value = value
                .as_u64()
                .and_then(|value| i64::try_from(value).ok())
                .ok_or_else(|| SyncError::new("SYNC_VALUE_RANGE", "整数超出同步范围"))?;
            SqlValue::Integer(value)
        }
        Value::Number(value) => SqlValue::Real(
            value
                .as_f64()
                .ok_or_else(|| SyncError::new("SYNC_VALUE_INVALID", "数值无法同步"))?,
        ),
        Value::String(value) => SqlValue::Text(value.clone()),
        Value::Array(_) | Value::Object(_) => {
            return Err(SyncError::new(
                "SYNC_VALUE_INVALID",
                "数据表字段包含不支持的嵌套值",
            ))
        }
    })
}

fn table_rows(conn: &Connection, spec: TableSpec) -> SyncResult<Vec<(String, Value)>> {
    let table = safe_identifier(spec.name)?;
    let key_column = safe_identifier(spec.key)?;
    let mut statement = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY {key_column}"))
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|v| (*v).into())
        .collect();
    let mut rows = statement
        .query([])
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let mut result = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?
    {
        let mut value = Map::new();
        for (index, name) in names.iter().enumerate() {
            value.insert(
                name.clone(),
                sql_to_json(
                    row.get_ref(index)
                        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?,
                )?,
            );
        }
        for name in spec.excluded {
            value.remove(*name);
        }
        if spec.name == "kol_experts" {
            let ids:String=conn.query_row("SELECT json_group_array(work_id) FROM (SELECT work_id FROM kol_projects WHERE expert_id=?1 ORDER BY work_id)",[value.get("id").and_then(Value::as_i64)],|r|r.get(0)).map_err(|e|SyncError::new("SYNC_DB_READ",e.to_string()))?;
            value.insert("linked_projects_json".into(), Value::String(ids));
        }
        let key = match value.get(spec.key) {
            Some(Value::Number(number)) => number.to_string(),
            Some(Value::String(text)) if !text.is_empty() => text.clone(),
            _ => return Err(SyncError::new("SYNC_ROW_KEY", "同步数据缺少可用主键")),
        };
        result.push((key, Value::Object(value)));
    }
    Ok(result)
}

fn ensure_uid(conn: &Connection, table: &str, key: &str) -> SyncResult<String> {
    match conn.query_row(
        "SELECT entity_uid FROM sync_entities WHERE table_name=?1 AND local_key=?2",
        params![table, key],
        |row| row.get::<_, String>(0),
    ) {
        Ok(uid) => Ok(uid),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let uid = if table == "material_blobs" {
                format!("blob:{key}")
            } else {
                uuid::Uuid::new_v4().to_string()
            };
            conn.execute(
                "INSERT INTO sync_entities(table_name,local_key,entity_uid,deleted) VALUES(?1,?2,?3,0)",
                params![table,key,uid],
            )
            .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
            Ok(uid)
        }
        Err(error) => Err(SyncError::new("SYNC_DB_READ", error.to_string())),
    }
}

fn local_key_for_uid(conn: &Connection, table: &str, uid: &str) -> SyncResult<Option<String>> {
    match conn.query_row(
        "SELECT local_key FROM sync_entities WHERE table_name=?1 AND entity_uid=?2",
        params![table, uid],
        |row| row.get(0),
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(SyncError::new("SYNC_DB_READ", error.to_string())),
    }
}

fn references_for(
    conn: &Connection,
    spec: TableSpec,
    row: &Value,
) -> SyncResult<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for (column, target_table) in spec.refs {
        let Some(value) = row.get(*column) else {
            continue;
        };
        let key = match value {
            Value::Number(number) => number.to_string(),
            Value::String(text) if !text.is_empty() => text.clone(),
            Value::Null => continue,
            _ => {
                return Err(SyncError::new(
                    "SYNC_REFERENCE_INVALID",
                    "同步数据关系格式无效",
                ))
            }
        };
        result.insert((*column).into(), ensure_uid(conn, target_table, &key)?);
    }
    if matches!(spec.name, "qa_sessions" | "qa_turns") {
        let scope: Vec<i64> = serde_json::from_str(
            row.get("scope_json")
                .and_then(Value::as_str)
                .unwrap_or("[]"),
        )
        .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "问答项目范围格式无效"))?;
        for (index, id) in scope.into_iter().enumerate() {
            result.insert(
                format!("json|scope_json|/{index}|works"),
                ensure_uid(conn, "works", &id.to_string())?,
            );
        }
    }
    for slot in references::slots(spec.name, row)? {
        result.insert(
            references::tag(&slot),
            ensure_uid(conn, slot.table, &slot.key)?,
        );
    }
    Ok(result)
}

fn table_spec(name: &str) -> Option<TableSpec> {
    TABLES.iter().copied().find(|spec| spec.name == name)
}

fn load_state(
    conn: &Connection,
    table: &str,
    key: &str,
) -> SyncResult<Option<(Value, BTreeMap<String, FieldStamp>, bool)>> {
    let found = conn.query_row(
        "SELECT row_json,field_versions_json,deleted FROM sync_row_versions WHERE table_name=?1 AND row_key=?2",
        params![table, key],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?)),
    );
    match found {
        Ok((row, fields, deleted)) => Ok(Some((
            serde_json::from_str(&row)
                .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "本机同步状态损坏"))?,
            serde_json::from_str(&fields)
                .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "本机字段版本损坏"))?,
            deleted,
        ))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(SyncError::new("SYNC_DB_READ", error.to_string())),
    }
}

fn scan_local(conn: &Connection, device: &str, seq: u64) -> SyncResult<Vec<RecordState>> {
    let stamp = now_ms();
    let context = causal_context(conn)?;
    let edits = pending_field_stamps(conn, device)?;
    for spec in TABLES {
        let table = spec.name;
        let mut seen = std::collections::BTreeSet::new();
        for (key, row) in table_rows(conn, *spec)? {
            seen.insert(key.clone());
            let previous = load_state(conn, table, &key)?;
            let mut fields = previous
                .as_ref()
                .map(|(_, fields, _)| fields.clone())
                .unwrap_or_default();
            let previous_row = previous.as_ref().map(|(row, _, _)| row);
            let object = row
                .as_object()
                .ok_or_else(|| SyncError::new("SYNC_ROW_INVALID", "同步行格式无效"))?;
            for (name, value) in object {
                let recorded = edits.get(&(table.to_owned(), key.clone(), name.clone()));
                if previous_row.and_then(|row| row.get(name)) != Some(value) || recorded.is_some() {
                    fields.insert(
                        name.clone(),
                        recorded.cloned().unwrap_or(FieldStamp {
                            edited_at_ms: stamp,
                            device_id: device.into(),
                            seq,
                            context: context.clone(),
                        }),
                    );
                }
            }
            conn.execute(
                "INSERT INTO sync_row_versions(table_name,row_key,row_json,field_versions_json,deleted,edited_at_ms,device_id,seq) VALUES(?1,?2,?3,?4,0,?5,?6,?7) ON CONFLICT(table_name,row_key) DO UPDATE SET row_json=excluded.row_json,field_versions_json=excluded.field_versions_json,deleted=0,edited_at_ms=excluded.edited_at_ms,device_id=excluded.device_id,seq=excluded.seq",
                params![table,key,row.to_string(),serde_json::to_string(&fields).unwrap(),stamp,device,seq],
            ).map_err(|error| SyncError::new("SYNC_DB_WRITE",error.to_string()))?;
            ensure_uid(conn, table, &key)?;
        }
        let mut known = conn
            .prepare("SELECT row_key,row_json,field_versions_json,deleted FROM sync_row_versions WHERE table_name=?1")
            .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
        let rows: Vec<(String, String, String, bool)> = known
            .query_map([table], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
        drop(known);
        for (key, row, mut fields, deleted) in rows {
            if !deleted && !seen.contains(&key) {
                let mut versions: BTreeMap<String, FieldStamp> = serde_json::from_str(&fields)
                    .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "本机字段版本损坏"))?;
                versions.insert(
                    "__deleted".into(),
                    edits
                        .get(&(table.to_owned(), key.clone(), "__deleted".into()))
                        .cloned()
                        .unwrap_or(FieldStamp {
                            edited_at_ms: stamp,
                            device_id: device.into(),
                            seq,
                            context: context.clone(),
                        }),
                );
                fields = serde_json::to_string(&versions).unwrap();
                conn.execute(
                    "UPDATE sync_row_versions SET field_versions_json=?3,deleted=1,edited_at_ms=?4,device_id=?5,seq=?6 WHERE table_name=?1 AND row_key=?2",
                    params![table,key,fields,stamp,device,seq],
                ).map_err(|error| SyncError::new("SYNC_DB_WRITE",error.to_string()))?;
                let _ = row;
            }
        }
    }
    let mut statement = conn.prepare("SELECT table_name,row_key,row_json,field_versions_json,deleted FROM sync_row_versions ORDER BY table_name,row_key")
        .map_err(|error| SyncError::new("SYNC_DB_READ",error.to_string()))?;
    let mapped = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let mut records: Vec<RecordState> = mapped
        .map(|row| {
            let (table, key, json, fields, deleted) =
                row.map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
            let value: Value = serde_json::from_str(&json)
                .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "本机同步状态损坏"))?;
            let spec = table_spec(&table).ok_or_else(|| {
                SyncError::new("SYNC_TABLE_UNSUPPORTED", "同步状态包含未知数据类型")
            })?;
            Ok(RecordState {
                uid: ensure_uid(conn, &table, &key)?,
                refs: references_for(conn, spec, &value)?,
                table,
                key,
                row: value,
                deleted,
                fields: serde_json::from_str(&fields)
                    .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "本机字段版本损坏"))?,
            })
        })
        .collect::<SyncResult<Vec<_>>>()?;
    records.sort_by_key(|record| {
        TABLES
            .iter()
            .position(|spec| spec.name == record.table)
            .unwrap_or(usize::MAX)
    });
    conn.execute("DELETE FROM sync_pending_edits", [])
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    Ok(records)
}

pub(crate) fn install_capture(conn: &Connection) -> crate::db::DbResult<()> {
    for spec in TABLES {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", spec.name))?;
        let columns: Vec<String> = stmt
            .query_map([], |r| r.get(1))?
            .collect::<rusqlite::Result<_>>()?;
        let object = |prefix: &str| {
            format!(
                "json_object({})",
                columns
                    .iter()
                    .filter(|c| !spec.excluded.contains(&c.as_str()))
                    .map(|c| format!("'{c}',{prefix}.\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        for (op, old, new, key) in [
            ("INSERT", "NULL".to_owned(), object("NEW"), "NEW"),
            ("UPDATE", object("OLD"), object("NEW"), "NEW"),
            ("DELETE", object("OLD"), "NULL".to_owned(), "OLD"),
        ] {
            conn.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS sync_capture_{}_{op} AFTER {op} ON \"{}\" WHEN (SELECT dataset_id<>'' AND suppress_capture=0 FROM sync_local_state WHERE id=1) BEGIN UPDATE sync_local_state SET next_seq=next_seq+1 WHERE id=1; INSERT INTO sync_pending_edits(seq,table_name,row_key,before_json,after_json,occurred_at_ms) SELECT next_seq,'{}',CAST({key}.\"{}\" AS TEXT),{old},{new},CAST(unixepoch('subsec')*1000 AS INTEGER) FROM sync_local_state WHERE id=1; END;",spec.name,spec.name,spec.name,spec.key))?;
        }
    }
    for (event, prefix) in [
        ("INSERT", "NEW"),
        ("DELETE", "OLD"),
        ("UPDATE", "NEW"),
        ("UPDATE", "OLD"),
    ] {
        conn.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS sync_capture_kol_projects_{event}_{prefix} AFTER {event} ON kol_projects WHEN (SELECT dataset_id<>'' AND suppress_capture=0 FROM sync_local_state WHERE id=1) BEGIN UPDATE sync_local_state SET next_seq=next_seq+1 WHERE id=1; INSERT INTO sync_pending_edits(seq,table_name,row_key,before_json,after_json,occurred_at_ms) SELECT next_seq,'kol_experts',CAST({prefix}.expert_id AS TEXT),NULL,json_object('linked_projects_json',(SELECT json_group_array(work_id) FROM (SELECT work_id FROM kol_projects WHERE expert_id={prefix}.expert_id ORDER BY work_id))),CAST(unixepoch('subsec')*1000 AS INTEGER) FROM sync_local_state WHERE id=1; END;"))?;
    }
    Ok(())
}

fn pending_field_stamps(
    conn: &Connection,
    device: &str,
) -> SyncResult<BTreeMap<(String, String, String), FieldStamp>> {
    let context = causal_context(conn)?;
    let mut stmt=conn.prepare("SELECT seq,table_name,row_key,before_json,after_json,occurred_at_ms FROM sync_pending_edits ORDER BY seq").map_err(|e|SyncError::new("SYNC_DB_READ",e.to_string()))?;
    let entries = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })
        .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
    let mut result = BTreeMap::new();
    for entry in entries {
        let (seq, table, key, before, after, at) =
            entry.map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let parse = |s: Option<String>| {
            serde_json::from_str::<Value>(s.as_deref().unwrap_or("null"))
                .map_err(|_| SyncError::new("SYNC_JOURNAL_INVALID", "待同步修改记录损坏"))
        };
        let before = parse(before)?;
        let after = parse(after)?;
        let stamp = FieldStamp {
            edited_at_ms: at,
            device_id: device.into(),
            seq,
            context: context.clone(),
        };
        if after.is_null() {
            result.insert((table, key, "__deleted".into()), stamp);
            continue;
        }
        for (name, value) in after
            .as_object()
            .ok_or_else(|| SyncError::new("SYNC_JOURNAL_INVALID", "修改记录格式无效"))?
        {
            if before.get(name) != Some(value) {
                result.insert((table.clone(), key.clone(), name.clone()), stamp.clone());
            }
        }
    }
    Ok(result)
}

fn sequence(conn: &Connection) -> SyncResult<u64> {
    let next: i64 = conn
        .query_row(
            "SELECT next_seq FROM sync_local_state WHERE id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    conn.execute("UPDATE sync_local_state SET next_seq=next_seq+1,updated_at=strftime('%s','now') WHERE id=1", [])
        .map_err(|error| SyncError::new("SYNC_DB_WRITE",error.to_string()))?;
    Ok(next.max(1) as u64)
}

pub fn publish_state(
    conn: &Connection,
    root: &Path,
    device: &str,
    dataset: &str,
    generation: &str,
) -> SyncResult<std::path::PathBuf> {
    let tx = crate::db::write_transaction(conn)
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    let seq = sequence(&tx)?;
    let records = scan_local(&tx, device, seq)?;
    tx.commit()
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    let directory = root
        .join("generations")
        .join(generation)
        .join("devices")
        .join(device)
        .join("states");
    if directory.is_dir() {
        let mut previous: Vec<_> = fs::read_dir(&directory)
            .map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("json"))
            .collect();
        previous.sort();
        if let Some(path) = previous.last() {
            let prior = read_verified_state(path)?;
            if prior.dataset_id == dataset
                && prior.generation == generation
                && prior.device_id == device
                && prior.records == records
            {
                prune_owned_states(&directory, device, dataset, generation)?;
                return Ok(path.clone());
            }
        }
    }
    let mut state = StateFile {
        application: "MSLDesktop".into(),
        format_version: 2,
        dataset_id: dataset.into(),
        generation: generation.into(),
        device_id: device.into(),
        sequence: seq,
        created_at_ms: now_ms(),
        records,
        chunks: Vec::new(),
    };
    let mut bytes = serde_json::to_vec(&state)
        .map_err(|error| SyncError::new("SYNC_SERIALIZE", error.to_string()))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err(SyncError::new(
                "SYNC_STATE_TOO_LARGE",
                "本次数据超过 128 MiB 安全处理范围；本机记录已保留，未截断上传",
            ));
        }
        let chunk_dir = directory.join("chunks");
        fs::create_dir_all(&chunk_dir)
            .map_err(|e| SyncError::new("SYNC_DIRECTORY_WRITE", e.to_string()))?;
        // Content-addressed chunks arrive before the immutable index is published.
        // Each record remains intact; the receiver applies the whole index atomically.
        let records = std::mem::take(&mut state.records);
        let mut batch = Vec::new();
        let mut size = 2;
        for record in records {
            let n = serde_json::to_vec(&record)
                .map_err(|e| SyncError::new("SYNC_SERIALIZE", e.to_string()))?
                .len()
                + 1;
            if n as u64 > MAX_STATE_BYTES - 2 {
                return Err(SyncError::new(
                    "SYNC_RECORD_TOO_LARGE",
                    "单条记录超过 8 MiB，未截断内容，请拆分该记录",
                ));
            }
            if size + n > 1024 * 1024 && !batch.is_empty() {
                state.chunks.push(write_chunk(&chunk_dir, &batch)?);
                batch.clear();
                size = 2;
            }
            batch.push(record);
            size += n;
        }
        if !batch.is_empty() {
            state.chunks.push(write_chunk(&chunk_dir, &batch)?);
        }
        state.format_version = 2;
        bytes = serde_json::to_vec(&state)
            .map_err(|e| SyncError::new("SYNC_SERIALIZE", e.to_string()))?;
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    fs::create_dir_all(&directory)
        .map_err(|error| SyncError::new("SYNC_DIRECTORY_WRITE", error.to_string()))?;
    let target = directory.join(format!("state-{seq:020}-{digest}.json"));
    let temporary = directory.join(format!(".state-{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temporary, &bytes)
        .map_err(|error| SyncError::new("SYNC_STATE_WRITE", error.to_string()))?;
    fs::rename(&temporary, &target)
        .map_err(|error| SyncError::new("SYNC_STATE_PUBLISH", error.to_string()))?;
    conn.execute(
        "INSERT OR IGNORE INTO sync_checkpoints(id,dataset_id,generation,created_by,sequence,state_hash,created_at) VALUES(?1,?2,?3,?4,?5,?6,strftime('%s','now'))",
        params![format!("{device}:{seq}:{digest}"), dataset, generation, device, seq, digest],
    )
    .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    prune_owned_states(&directory, device, dataset, generation)?;
    Ok(target)
}

fn prune_owned_states(
    directory: &Path,
    device: &str,
    dataset: &str,
    generation: &str,
) -> SyncResult<()> {
    crate::storage::paths::reject_link_components(directory)
        .map_err(|e| SyncError::new("SYNC_STATE_UNSAFE", e))?;
    let mut files: Vec<_> = fs::read_dir(directory)
        .map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("state-") && n.ends_with(".json"))
        })
        .collect();
    if files.len() <= 8 {
        return Ok(());
    }
    files.sort();
    let mut verified = Vec::new();
    for path in files {
        let state = read_verified_state(&path)?;
        // Unknown copies and another device's files are never cleanup candidates.
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        if state.device_id != device
            || state.dataset_id != dataset
            || state.generation != generation
            || name.len() != 6 + 20 + 1 + 64 + 5
        {
            return Ok(());
        }
        verified.push((path, state.chunks));
    }
    let cutoff = verified.len().saturating_sub(8);
    let keep: std::collections::BTreeSet<_> = verified[cutoff..]
        .iter()
        .flat_map(|(_, chunks)| chunks.iter().cloned())
        .collect();
    let removed: std::collections::BTreeSet<_> = verified[..cutoff]
        .iter()
        .flat_map(|(_, chunks)| chunks.iter().cloned())
        .filter(|hash| !keep.contains(hash))
        .collect();
    // Every retained checkpoint contains the complete state, including tombstones.
    // Publish and validate the latest checkpoint before removing an older one.
    for (path, _) in &verified[..cutoff] {
        fs::remove_file(path).map_err(|e| {
            SyncError::new(
                "SYNC_RETENTION_PENDING",
                format!("新检查点已保存，旧文件待清理：{e}"),
            )
        })?;
    }
    for hash in removed {
        let path = directory.join("chunks").join(format!("{hash}.json"));
        crate::storage::paths::reject_link_components(&path)
            .map_err(|e| SyncError::new("SYNC_STATE_UNSAFE", e))?;
        if path.is_file() {
            fs::remove_file(path).map_err(|e| {
                SyncError::new(
                    "SYNC_RETENTION_PENDING",
                    format!("未引用的同步分片待清理：{e}"),
                )
            })?;
        }
    }
    Ok(())
}

fn apply_row(conn: &Connection, spec: TableSpec, row: &Value) -> SyncResult<()> {
    if spec.name == "kol_experts" && row.get("linked_projects_json").is_some() {
        let ids: Vec<i64> = serde_json::from_str(
            row["linked_projects_json"]
                .as_str()
                .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "专家项目关联格式无效"))?,
        )
        .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "专家项目关联格式无效"))?;
        let mut profile = row.clone();
        profile
            .as_object_mut()
            .unwrap()
            .remove("linked_projects_json");
        apply_row(conn, spec, &profile)?;
        let id = profile["id"]
            .as_i64()
            .ok_or_else(|| SyncError::new("SYNC_ROW_KEY", "专家编号无效"))?;
        conn.execute("DELETE FROM kol_projects WHERE expert_id=?1", [id])
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
        for work_id in ids {
            conn.execute("INSERT OR IGNORE INTO kol_projects(expert_id,work_id) SELECT ?1,id FROM works WHERE id=?2",params![id,work_id]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
        }
        return Ok(());
    }
    let table_name = safe_identifier(spec.name)?;
    let key_name = safe_identifier(spec.key)?;
    let object = row
        .as_object()
        .ok_or_else(|| SyncError::new("SYNC_ROW_INVALID", "同步行格式无效"))?;
    let key = json_to_sql(
        object
            .get(spec.key)
            .ok_or_else(|| SyncError::new("SYNC_ROW_KEY", "同步数据缺少主键"))?,
    )?;
    let exists: bool = conn
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table_name} WHERE {key_name}=?1)"),
            [&key],
            |row| row.get(0),
        )
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let columns: Vec<String> = object.keys().cloned().collect();
    let values: Vec<SqlValue> = columns
        .iter()
        .map(|name| json_to_sql(&object[name]))
        .collect::<SyncResult<_>>()?;
    if exists {
        let update_columns: Vec<&String> = columns
            .iter()
            .filter(|name| name.as_str() != spec.key)
            .collect();
        let assignments = update_columns
            .iter()
            .map(|name| Ok(format!("{}=?", safe_identifier(name)?)))
            .collect::<SyncResult<Vec<_>>>()?
            .join(",");
        let mut update_values: Vec<SqlValue> = update_columns
            .iter()
            .map(|name| json_to_sql(&object[*name]))
            .collect::<SyncResult<_>>()?;
        update_values.push(key);
        conn.execute(
            &format!("UPDATE {table_name} SET {assignments} WHERE {key_name}=?"),
            params_from_iter(update_values),
        )
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    } else {
        let names = columns
            .iter()
            .map(|name| safe_identifier(name))
            .collect::<SyncResult<Vec<_>>>()?
            .join(",");
        let placeholders = (0..columns.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        conn.execute(
            &format!("INSERT INTO {table_name}({names}) VALUES({placeholders})"),
            params_from_iter(values),
        )
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    }
    Ok(())
}

fn delete_row(conn: &Connection, spec: TableSpec, key: &str) -> SyncResult<()> {
    if spec.name == "works" {
        // Preserve even this device's unpublished tasks when a confirmed project
        // deletion arrives. Only database-owned context is removed.
        for table in [
            "tasks",
            "waiting_items",
            "calendar_events",
            "activity_events",
        ] {
            conn.execute(
                &format!("UPDATE {table} SET work_id=NULL WHERE work_id=?1"),
                [key],
            )
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
        }
        for table in ["resume_points", "work_file_refs"] {
            conn.execute(&format!("DELETE FROM {table} WHERE work_id=?1"), [key])
                .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
        }
        crate::db::source_lifecycle::retire(conn, &[format!("work:{key}")])
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    }
    let table = safe_identifier(spec.name)?;
    let column = safe_identifier(spec.key)?;
    conn.execute(
        &format!("DELETE FROM {table} WHERE CAST({column} AS TEXT)=?1"),
        [key],
    )
    .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    conn.execute(
        "UPDATE sync_entities SET deleted=1 WHERE table_name=?1 AND local_key=?2",
        params![spec.name, key],
    )
    .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    Ok(())
}

fn row_exists(conn: &Connection, spec: TableSpec, key: &str) -> SyncResult<bool> {
    let table = safe_identifier(spec.name)?;
    let column = safe_identifier(spec.key)?;
    conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE CAST({column} AS TEXT)=?1)"),
        [key],
        |row| row.get(0),
    )
    .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))
}

fn allocate_integer_key(conn: &Connection, spec: TableSpec) -> SyncResult<String> {
    let table = safe_identifier(spec.name)?;
    let column = safe_identifier(spec.key)?;
    let value: i64 = conn
        .query_row(
            &format!("SELECT MAX(COALESCE((SELECT MAX({column}) FROM {table}),0),COALESCE((SELECT MAX(CAST(local_key AS INTEGER)) FROM sync_entities WHERE table_name=?1),0))+1"),
            [spec.name],
            |row| row.get(0),
        )
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    Ok(value.to_string())
}

fn map_remote_key(conn: &Connection, remote: &RecordState) -> SyncResult<String> {
    let spec = table_spec(&remote.table).ok_or_else(|| {
        SyncError::new(
            "SYNC_TABLE_UNSUPPORTED",
            "同步包包含当前版本不支持的数据类型",
        )
    })?;
    let key = if spec.name == "capture_context" {
        let uid = remote
            .refs
            .get("inbox_id")
            .ok_or_else(|| SyncError::new("SYNC_REFERENCE_PENDING", "收件箱关联尚未到达"))?;
        let key = local_key_for_uid(conn, "inbox_items", uid)?
            .ok_or_else(|| SyncError::new("SYNC_REFERENCE_PENDING", "收件箱关联尚未到达"))?;
        conn.execute("INSERT OR IGNORE INTO sync_entities(table_name,local_key,entity_uid,deleted) VALUES(?1,?2,?3,?4)",params![spec.name,key,remote.uid,remote.deleted]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
        key
    } else if let Some(existing) = local_key_for_uid(conn, spec.name, &remote.uid)? {
        existing
    } else {
        let raw_mapping: Option<String> = conn
            .query_row(
                "SELECT entity_uid FROM sync_entities WHERE table_name=?1 AND local_key=?2",
                params![spec.name, remote.key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let occupied = row_exists(conn, spec, &remote.key)?;
        let chosen = if occupied || raw_mapping.is_some() {
            allocate_integer_key(conn, spec)?
        } else {
            remote.key.clone()
        };
        conn.execute(
            "INSERT INTO sync_entities(table_name,local_key,entity_uid,deleted) VALUES(?1,?2,?3,?4)",
            params![spec.name,chosen,remote.uid,remote.deleted],
        )
        .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
        chosen
    };
    Ok(key)
}
fn localize_remote(conn: &Connection, remote: &RecordState) -> SyncResult<RecordState> {
    let spec = table_spec(&remote.table)
        .ok_or_else(|| SyncError::new("SYNC_TABLE_UNSUPPORTED", "不支持的数据表"))?;
    let key = map_remote_key(conn, remote)?;
    let mut localized = remote.clone();
    localized.key = key.clone();
    let slots = references::slots(spec.name, &remote.row)?;
    for (tag, uid) in remote
        .refs
        .iter()
        .filter(|(tag, _)| tag.starts_with("typed|"))
    {
        let slot = slots
            .iter()
            .find(|slot| references::tag(slot) == *tag)
            .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "未登记的结构化关系位置"))?;
        let key = local_key_for_uid(conn, slot.table, uid)?
            .ok_or_else(|| SyncError::new("SYNC_REFERENCE_PENDING", "结构化关系的目标尚未到达"))?;
        references::replace(&mut localized.row, slot, &key)?;
    }
    let object = localized
        .row
        .as_object_mut()
        .ok_or_else(|| SyncError::new("SYNC_ROW_INVALID", "同步行格式无效"))?;
    let original_key = object
        .get(spec.key)
        .ok_or_else(|| SyncError::new("SYNC_ROW_KEY", "同步数据缺少主键"))?;
    object.insert(
        spec.key.into(),
        if original_key.is_number() {
            Value::Number(
                key.parse::<i64>()
                    .map_err(|_| SyncError::new("SYNC_ROW_KEY", "整数主键无法映射"))?
                    .into(),
            )
        } else {
            Value::String(key)
        },
    );
    for (column, target_uid) in &remote.refs {
        if column.starts_with("typed|") {
            continue;
        }
        if let Some(pointer) = column.strip_prefix("json|scope_json|") {
            if !matches!(spec.name, "qa_sessions" | "qa_turns") {
                return Err(SyncError::new("SYNC_REFERENCE_INVALID", "未登记的项目范围"));
            }
            let index = pointer
                .strip_suffix("|works")
                .and_then(|p| p.strip_prefix('/'))
                .and_then(|p| p.parse::<usize>().ok())
                .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "项目范围位置无效"))?;
            let key = local_key_for_uid(conn, "works", target_uid)?
                .ok_or_else(|| SyncError::new("SYNC_REFERENCE_PENDING", "关联项目尚未到达"))?;
            let mut scope: Vec<i64> = serde_json::from_str(
                object
                    .get("scope_json")
                    .and_then(Value::as_str)
                    .unwrap_or("[]"),
            )
            .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "问答范围格式无效"))?;
            let value = scope
                .get_mut(index)
                .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "问答范围位置不存在"))?;
            *value = key
                .parse()
                .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "项目编号无效"))?;
            object.insert(
                "scope_json".into(),
                Value::String(serde_json::to_string(&scope).unwrap()),
            );
            continue;
        }
        let (_, target_table) = spec
            .refs
            .iter()
            .find(|(candidate, _)| candidate == column)
            .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "同步关系未登记"))?;
        let target_key = local_key_for_uid(conn, target_table, target_uid)?.ok_or_else(|| {
            SyncError::new(
                "SYNC_REFERENCE_PENDING",
                "同步关系的目标尚未到达，请稍后重试",
            )
        })?;
        let localized_ref = match object.get(column) {
            Some(Value::Number(_)) => Value::Number(
                target_key
                    .parse::<i64>()
                    .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "关系主键无法映射"))?
                    .into(),
            ),
            Some(Value::String(_)) => Value::String(target_key),
            _ => return Err(SyncError::new("SYNC_REFERENCE_INVALID", "关系字段格式无效")),
        };
        object.insert(column.clone(), localized_ref);
    }
    Ok(localized)
}

fn merge_remote(conn: &Connection, remote: &RecordState, peer: &str) -> SyncResult<(u64, u64)> {
    let remote = localize_remote(conn, remote)?;
    let spec = table_spec(&remote.table).unwrap();
    let local = load_state(conn, &remote.table, &remote.key)?;
    if local.is_none() {
        if !remote.deleted {
            apply_row(conn, spec, &remote.row)?;
        }
        conn.execute(
            "INSERT INTO sync_row_versions(table_name,row_key,row_json,field_versions_json,deleted,edited_at_ms,device_id,seq) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![remote.table,remote.key,remote.row.to_string(),serde_json::to_string(&remote.fields).unwrap(),remote.deleted,remote.fields.values().map(|s|s.edited_at_ms).max().unwrap_or(0),peer,remote.fields.values().map(|s|s.seq).max().unwrap_or(0)],
        ).map_err(|error| SyncError::new("SYNC_DB_WRITE",error.to_string()))?;
    } else {
        let (local_row, local_fields, local_deleted) = local.unwrap();
        let peer_base: Option<String> = conn.query_row(
            "SELECT row_json FROM sync_peer_rows WHERE table_name=?1 AND row_key=?2 AND device_id=?3",
            params![remote.table,remote.key,peer],|row|row.get(0)
        ).optional().map_err(|e|SyncError::new("SYNC_DB_READ",e.to_string()))?;
        let base: Value = peer_base
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(|_| {
                SyncError::new("SYNC_BASE_INVALID", "本机同步比较基线无法解析，已停止合并")
            })?
            .unwrap_or(Value::Null);
        let mut merged = local_row.clone();
        let local_object = local_row.as_object().cloned().unwrap_or_default();
        let remote_object = remote.row.as_object().cloned().unwrap_or_default();
        let base_object = base.as_object().cloned().unwrap_or_default();
        let mut merged_fields = local_fields.clone();
        let mut conflicts = 0;
        for name in local_object
            .keys()
            .chain(remote_object.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            let l = local_object.get(name).unwrap_or(&Value::Null);
            let r = remote_object.get(name).unwrap_or(&Value::Null);
            let b = base_object.get(name).unwrap_or(&Value::Null);
            if l == r {
                if let (Some(ls), Some(rs)) = (local_fields.get(name), remote.fields.get(name)) {
                    if follows(rs, ls) {
                        merged_fields.insert(name.clone(), rs.clone());
                    }
                }
                continue;
            }
            if let (Some(ls), Some(rs)) = (local_fields.get(name), remote.fields.get(name)) {
                if follows(rs, ls) {
                    merged[name] = r.clone();
                    merged_fields.insert(name.clone(), rs.clone());
                    continue;
                }
                if follows(ls, rs) {
                    continue;
                }
            }
            let remote_changed = r != b;
            let local_changed = l != b;
            if remote_changed && !local_changed {
                merged[name] = r.clone();
                if let Some(stamp) = remote.fields.get(name) {
                    merged_fields.insert(name.clone(), stamp.clone());
                }
            } else if remote_changed && local_changed {
                let ls = local_fields.get(name);
                let rs = remote.fields.get(name);
                if rs > ls {
                    merged[name] = r.clone();
                    if let Some(stamp) = rs {
                        merged_fields.insert(name.clone(), stamp.clone());
                    }
                }
                let conflict_id = format!(
                    "{}:{}:{}:{}:{}",
                    remote.table,
                    remote.key,
                    name,
                    peer,
                    rs.map(|s| s.seq).unwrap_or(0)
                );
                conn.execute("INSERT OR IGNORE INTO sync_conflicts(table_name,row_key,field,local_value,remote_value,base_value,created_at,resolved_at,resolution,id) VALUES(?1,?2,?3,?4,?5,?6,strftime('%s','now'),NULL,NULL,?7)",params![remote.table,remote.key,name,l.to_string(),r.to_string(),b.to_string(),conflict_id]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
                conflicts += 1;
            }
        }
        let remote_delete = remote.deleted;
        let ls = local_fields.get("__deleted");
        let rs = remote.fields.get("__deleted");
        let remote_decision = matches!((ls,rs),(Some(l),Some(r)) if follows(r,l));
        let local_decision = matches!((ls,rs),(Some(l),Some(r)) if follows(l,r));
        let merged_deleted = if remote_decision {
            remote_delete
        } else if local_decision {
            local_deleted
        } else {
            local_deleted || remote_delete
        };
        if local_deleted != remote_delete && !remote_decision && !local_decision {
            if base.is_null() || local_row != base || local_deleted {
                let conflict_id = format!(
                    "delete:{}:{}:{}:{}",
                    remote.table,
                    remote.key,
                    peer,
                    remote.fields.values().map(|s| s.seq).max().unwrap_or(0)
                );
                conn.execute("INSERT OR IGNORE INTO sync_conflicts(id,table_name,row_key,field,local_value,remote_value,base_value,created_at) VALUES(?1,?2,?3,'__deleted',?4,?5,'false',strftime('%s','now'))",params![conflict_id,remote.table,remote.key,local_deleted.to_string(),remote_delete.to_string()]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
                conflicts += 1;
            }
        }
        if merged_deleted && !local_deleted {
            delete_row(conn, spec, &remote.key)?;
        } else if !merged_deleted {
            apply_row(conn, spec, &merged)?;
        }
        if remote_decision || (remote_delete && !local_deleted) {
            if let Some(stamp) = rs {
                merged_fields.insert("__deleted".into(), stamp.clone());
            }
        }
        conn.execute("UPDATE sync_row_versions SET row_json=?3,field_versions_json=?4,deleted=?5,edited_at_ms=?6,device_id=?7,seq=?8 WHERE table_name=?1 AND row_key=?2",params![remote.table,remote.key,merged.to_string(),serde_json::to_string(&merged_fields).unwrap(),merged_deleted,merged_fields.values().map(|s|s.edited_at_ms).max().unwrap_or(0),peer,merged_fields.values().map(|s|s.seq).max().unwrap_or(0)]).map_err(|error|SyncError::new("SYNC_DB_WRITE",error.to_string()))?;
        conn.execute("INSERT INTO sync_peer_rows(table_name,row_key,device_id,row_json,field_versions_json,deleted,seen_at) VALUES(?1,?2,?3,?4,?5,?6,strftime('%s','now')) ON CONFLICT(table_name,row_key,device_id) DO UPDATE SET row_json=excluded.row_json,field_versions_json=excluded.field_versions_json,deleted=excluded.deleted,seen_at=excluded.seen_at",params![remote.table,remote.key,peer,remote.row.to_string(),serde_json::to_string(&remote.fields).unwrap(),remote.deleted]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
        return Ok((1, conflicts));
    }
    conn.execute("INSERT INTO sync_peer_rows(table_name,row_key,device_id,row_json,field_versions_json,deleted,seen_at) VALUES(?1,?2,?3,?4,?5,?6,strftime('%s','now')) ON CONFLICT(table_name,row_key,device_id) DO UPDATE SET row_json=excluded.row_json,field_versions_json=excluded.field_versions_json,deleted=excluded.deleted,seen_at=excluded.seen_at",params![remote.table,remote.key,peer,remote.row.to_string(),serde_json::to_string(&remote.fields).unwrap(),remote.deleted]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
    Ok((1, 0))
}

pub fn receive_states(
    conn: &Connection,
    root: &Path,
    own_device: &str,
    dataset: &str,
    generation: &str,
) -> SyncResult<SyncSummary> {
    let mut summary = SyncSummary::default();
    let devices = root.join("generations").join(generation).join("devices");
    if !devices.exists() {
        return Ok(summary);
    }
    let transaction = if conn.is_autocommit() {
        Some(
            crate::db::write_transaction(conn)
                .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?,
        )
    } else {
        None
    };
    let conn = transaction.as_deref().unwrap_or(conn);
    conn.execute_batch("PRAGMA defer_foreign_keys=ON")
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    let seq = sequence(conn)?;
    scan_local(conn, own_device, seq)?;
    conn.execute(
        "UPDATE sync_local_state SET suppress_capture=1 WHERE id=1",
        [],
    )
    .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    let mut files = Vec::new();
    for entry in
        fs::read_dir(&devices).map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?
    {
        let entry = entry.map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?;
        if entry.file_name().to_string_lossy() == own_device {
            continue;
        }
        let states = entry.path().join("states");
        if !states.is_dir() {
            continue;
        }
        for file in fs::read_dir(states)
            .map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?
        {
            let file = file.map_err(|e| SyncError::new("SYNC_DIRECTORY_READ", e.to_string()))?;
            if file.file_name().to_string_lossy().starts_with("state-")
                && file.path().extension().and_then(|v| v.to_str()) == Some("json")
            {
                files.push(file.path());
            }
        }
    }
    files.sort();
    for file in files {
        crate::storage::paths::reject_link_components(&file)
            .map_err(|message| SyncError::new("SYNC_STATE_UNSAFE", message))?;
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        let state = read_verified_state(&file)?;
        if state.application != "MSLDesktop"
            || !matches!(state.format_version, 1 | 2)
            || state.dataset_id != dataset
            || state.generation != generation
        {
            return Err(SyncError::new(
                "SYNC_STATE_WRONG_DATASET",
                "同步状态属于其他数据集或版本",
            ));
        }
        let id = format!("{}:{}", state.device_id, filename);
        let already: Option<String> = conn
            .query_row(
                "SELECT change_id FROM sync_applied WHERE device_id=?1 AND seq=?2",
                params![state.device_id, state.sequence],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        if let Some(previous) = already {
            if previous != id {
                return Err(SyncError::new("SYNC_DEVICE_SEQUENCE_CONFLICT", "同一设备序号出现不同内容，可能存在设备副本或云盘冲突。已停止导入，请保留双方文件并重新连接发生冲突的设备"));
            }
            continue;
        }
        for record in &state.records {
            map_remote_key(conn, record)?;
        }
        for record in &state.records {
            let (applied, conflicts) = merge_remote(conn, record, &state.device_id)?;
            summary.applied += applied;
            summary.conflicts += conflicts;
        }
        conn.execute("INSERT INTO sync_applied(change_id,device_id,seq,applied_at) VALUES(?1,?2,?3,strftime('%s','now'))",params![id,state.device_id,state.sequence]).map_err(|e|SyncError::new("SYNC_DB_WRITE",e.to_string()))?;
    }
    // A late historical answer may arrive after its source tombstone. Re-apply
    // source retirement after the entire batch, including duplicate checkpoints.
    let deleted_sources = {
        let mut statement = conn
            .prepare("SELECT table_name,row_key FROM sync_row_versions WHERE deleted=1")
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let mut sources = Vec::new();
        for row in rows {
            let (table, key) = row.map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
            let kind = match table.as_str() {
                "works" => "work",
                "tasks" => "task",
                "waiting_items" => "waiting",
                "calendar_events" => "calendar",
                "inbox_items" => "inbox",
                "resume_points" => "resume",
                "kol_notes" => "kol_note",
                "kol_insights" => "kol_insight",
                "material_segments" => "kol_material",
                "reports" => "report",
                _ => continue,
            };
            sources.push(format!("{kind}:{key}"));
        }
        sources
    };
    crate::db::source_lifecycle::retire(conn, &deleted_sources)
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    conn.execute(
        "UPDATE sync_local_state SET suppress_capture=0 WHERE id=1",
        [],
    )
    .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    if let Some(tx) = transaction {
        tx.commit()
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    }
    Ok(summary)
}

fn read_verified_state(file: &Path) -> SyncResult<StateFile> {
    crate::storage::paths::reject_link_components(file)
        .map_err(|m| SyncError::new("SYNC_STATE_UNSAFE", m))?;
    let meta =
        fs::symlink_metadata(file).map_err(|e| SyncError::new("SYNC_STATE_READ", e.to_string()))?;
    if !meta.is_file() || meta.len() > MAX_STATE_BYTES {
        return Err(SyncError::new(
            "SYNC_STATE_TOO_LARGE",
            "同步文件类型或大小不符合限制",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(file)
        .map_err(|e| SyncError::new("SYNC_STATE_READ", e.to_string()))?
        .take(MAX_STATE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| SyncError::new("SYNC_STATE_READ", e.to_string()))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(SyncError::new("SYNC_STATE_TOO_LARGE", "同步文件超出限制"));
    }
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let claimed = name
        .strip_prefix("state-")
        .and_then(|s| s.split_once('-'))
        .map(|(_, s)| s.get(..64))
        .flatten();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if claimed != Some(digest.as_str()) {
        return Err(SyncError::new(
            "SYNC_STATE_INTEGRITY",
            "同步文件校验失败，请等待云盘下载完整后重试；本机数据未被此文件改写",
        ));
    }
    let mut state: StateFile = serde_json::from_slice(&bytes)
        .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "同步文件结构无效"))?;
    let directory_device = file
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|v| v.to_str());
    if directory_device != Some(state.device_id.as_str())
        || !name.starts_with(&format!("state-{:020}-", state.sequence))
    {
        return Err(SyncError::new(
            "SYNC_STATE_IDENTITY",
            "同步文件身份与位置不一致",
        ));
    }
    if !state.chunks.is_empty() {
        if state.format_version != 2 || !state.records.is_empty() || state.chunks.len() > 2048 {
            return Err(SyncError::new("SYNC_STATE_INVALID", "同步分片清单无效"));
        }
        let mut total = 0u64;
        let mut seen = std::collections::BTreeSet::new();
        for hash in &state.chunks {
            if hash.len() != 64
                || !hash.bytes().all(|c| c.is_ascii_hexdigit())
                || !seen.insert(hash)
            {
                return Err(SyncError::new("SYNC_STATE_INVALID", "同步分片标识无效"));
            }
            let path = file
                .parent()
                .unwrap()
                .join("chunks")
                .join(format!("{hash}.json"));
            crate::storage::paths::reject_link_components(&path)
                .map_err(|e| SyncError::new("SYNC_STATE_UNSAFE", e))?;
            let mut chunk = Vec::new();
            fs::File::open(&path)
                .map_err(|_| {
                    SyncError::new(
                        "SYNC_CHUNK_PENDING",
                        "部分同步分片尚未到达，请等待云盘完成后重试；本机数据未被此包改写",
                    )
                })?
                .take(MAX_STATE_BYTES + 1)
                .read_to_end(&mut chunk)
                .map_err(|e| SyncError::new("SYNC_STATE_READ", e.to_string()))?;
            total += chunk.len() as u64;
            if chunk.len() as u64 > MAX_STATE_BYTES
                || total > 128 * 1024 * 1024
                || format!("{:x}", Sha256::digest(&chunk)) != *hash
            {
                return Err(SyncError::new(
                    "SYNC_STATE_INTEGRITY",
                    "同步分片校验失败，未应用此包",
                ));
            }
            let records: Vec<RecordState> = serde_json::from_slice(&chunk)
                .map_err(|_| SyncError::new("SYNC_STATE_INVALID", "同步分片内容无效"))?;
            state.records.extend(records);
        }
    }
    Ok(state)
}

fn write_chunk(directory: &Path, records: &[RecordState]) -> SyncResult<String> {
    let bytes =
        serde_json::to_vec(records).map_err(|e| SyncError::new("SYNC_SERIALIZE", e.to_string()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let target = directory.join(format!("{hash}.json"));
    if !target.exists() {
        let temporary = directory.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temporary, &bytes)
            .map_err(|e| SyncError::new("SYNC_STATE_WRITE", e.to_string()))?;
        fs::rename(&temporary, &target)
            .map_err(|e| SyncError::new("SYNC_STATE_WRITE", e.to_string()))?;
    }
    Ok(hash)
}
