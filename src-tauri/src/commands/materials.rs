use super::with_db;
use crate::{app_state::AppState, db, materials};
use serde_json::{json, Value};
use tauri::State;
#[tauri::command]
pub fn list_kol_materials(
    state: State<AppState>,
    expert_id: Option<i64>,
) -> Result<Vec<Value>, String> {
    with_db(&state, |db| materials::list(db, expert_id))
}
#[tauri::command]
pub fn update_kol_material(
    state: State<AppState>,
    id: i64,
    expected_revision: i64,
    description: String,
) -> Result<(), String> {
    if description.chars().count() > 2000 {
        return Err("资料说明最多2000字".into());
    }
    with_db(&state, |db| {
        if db.conn().execute("UPDATE kol_materials SET description=?1,revision=revision+1 WHERE id=?2 AND revision=?3 AND status!='reading'",rusqlite::params![description.trim(),id,expected_revision])?!=1{return Err(db::DbError::Migration("资料已更新，请刷新".into()));}
        Ok(())
    })
}
#[tauri::command]
pub async fn remove_kol_material(id: i64, expected_revision: i64) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = db::Database::open(&db::default_db_path()).map_err(|e| e.to_string())?;
        materials::remove(&db, id, expected_revision).map_err(|e| e.to_string())?;
        let pending =
            materials::purge_unused(&db, &materials::root()).map_err(|e| e.to_string())?;
        Ok(json!({"removed":true,"pending_cleanup":pending}))
    })
    .await
    .map_err(|_| "资料移除任务中断")?
}
#[tauri::command]
pub async fn get_kol_material_preview(
    id: i64,
    segment_id: Option<i64>,
    expected_blob_hash: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = db::Database::open(&db::default_db_path()).map_err(|e| e.to_string())?;
        let id = materials::preview_id(&db, id, segment_id, expected_blob_hash.as_deref())
            .map_err(|e| e.to_string())?;
        let record = materials::get(&db, id).map_err(|e| e.to_string())?;
        let segments = db::knowledge::rows(
            db.conn(),
            "SELECT locator,text,kind FROM material_segments WHERE material_id=?1 ORDER BY ordinal",
            &[&id],
        )
        .map_err(|e| e.to_string())?;
        Ok(json!({"material":record,"segments":segments}))
    })
    .await
    .map_err(|_| "资料预览任务中断")?
}
#[tauri::command]
pub fn reveal_kol_material(state: State<AppState>, id: i64) -> Result<(), String> {
    let path = with_db(&state, |db| {
        let record = materials::get(db, id)?;
        materials::blob_path(db, &materials::root(), &record)
    })?;
    crate::workspace::reveal_in_explorer(&path).map_err(|e| e.to_string())
}
pub async fn import(expert: i64, paths: Vec<String>) -> Result<Value, String> {
    if paths.is_empty() || paths.len() > 100 {
        return Err("请每批选择1–100份资料".into());
    }
    let imported=tauri::async_runtime::spawn_blocking(move ||->Result<Value,String>{
  let db=db::Database::open(&db::default_db_path()).map_err(|e|e.to_string())?;let mut saved=Vec::new();let mut failed=Vec::new();
  for path in paths {let source=std::path::Path::new(&path);match materials::import_one(&db,&materials::root(),expert,source){Ok(row)=>saved.push(row),Err(e)=>failed.push(json!({"filename":source.file_name().and_then(|s|s.to_str()).unwrap_or("文件"),"error":e.to_string()}))}}
  Ok(json!({"saved":saved,"failed":failed}))
 }).await.map_err(|_|"上传任务中断")??;
    let ids = imported["saved"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["id"].as_i64())
        .collect::<Vec<_>>();
    if !ids.is_empty() {
        materials::reading::read(expert, ids, false).await?;
    }
    Ok(imported)
}
