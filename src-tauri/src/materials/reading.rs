use super::*;
use crate::ai::{
    provider::{AiAttachment, AiMessage, AiTextRequest},
    router::ResolvedModel,
};
use serde_json::json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileReading {
    id: i64,
    segments: Vec<String>,
    limitations: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadOutput {
    files: Vec<FileReading>,
}
pub fn accept_model_reading(
    db: &Database,
    records: &[Value],
    reader: &str,
    raw: &str,
) -> DbResult<()> {
    let output: ReadOutput = serde_json::from_str(&crate::ai::schema::strip_single_code_fence(raw))
        .map_err(|_| err("模型未返回有效资料结构，请重试"))?;
    let expected = records
        .iter()
        .filter_map(|r| r["id"].as_i64())
        .collect::<std::collections::BTreeSet<_>>();
    let actual = output
        .files
        .iter()
        .map(|r| r.id)
        .collect::<std::collections::BTreeSet<_>>();
    if expected != actual || output.files.len() != expected.len() {
        return Err(err("模型返回了错误的资料归属，读取结果未保存"));
    }
    // Validate the complete batch before writing any result.
    for file in &output.files {
        if (file.segments.is_empty() && file.limitations.is_empty())
            || file.segments.len() > 120
            || file
                .segments
                .iter()
                .any(|s| s.trim().is_empty() || s.chars().count() > 3500)
            || file.limitations.len() > 10
            || file.limitations.iter().any(|s| s.chars().count() > 500)
        {
            return Err(err("模型读取内容为空或超限，未保存为成功结果"));
        }
    }
    for file in output.files {
        let record = records.iter().find(|r| r["id"] == file.id).unwrap();
        if file.segments.is_empty() {
            set_failure(
                db,
                file.id,
                record["revision"].as_i64().unwrap(),
                &file.limitations.join("；"),
                true,
            )?;
            continue;
        }
        let mut segments = file
            .segments
            .into_iter()
            .enumerate()
            .map(|(i, text)| Segment {
                text,
                locator: format!("文件级 · 模型解读段 {}", i + 1),
                kind: "model_interpretation".into(),
            })
            .collect::<Vec<_>>();
        let local=knowledge::rows(db.conn(),"SELECT text,locator,kind FROM material_segments WHERE material_id=?1 AND kind='extracted_text' ORDER BY ordinal",&[&file.id])?;
        let mut combined = local
            .into_iter()
            .map(|r| Segment {
                text: r["text"].as_str().unwrap().into(),
                locator: r["locator"].as_str().unwrap().into(),
                kind: "extracted_text".into(),
            })
            .collect::<Vec<_>>();
        combined.append(&mut segments);
        let segments = combined;
        let note = format!(
            "模型解读，需回到原资料核对；未验证页码定位。{}",
            file.limitations.join("；")
        );
        // A removed or superseded item is discarded while unrelated items can finish.
        if get(db, file.id).is_ok_and(|current| current["revision"] == record["revision"]) {
            save_reading(
                db,
                file.id,
                record["revision"].as_i64().unwrap(),
                reader,
                &segments,
                true,
                &note,
            )?;
        }
    }
    Ok(())
}
pub fn set_failure(
    db: &Database,
    id: i64,
    revision: i64,
    message: &str,
    unsupported: bool,
) -> DbResult<()> {
    db.conn().execute("UPDATE kol_materials SET status=CASE WHEN EXISTS(SELECT 1 FROM material_segments WHERE material_id=?1) THEN 'partial' ELSE ?2 END,error=?3 WHERE id=?1 AND revision=?4",params![id,if unsupported{"unsupported"}else{"failed"},crate::cognition::bounded(message,1500),revision])?;
    Ok(())
}
fn resolve(db: &Database) -> DbResult<ResolvedModel> {
    crate::ai::router::resolve(
        &crate::db::provider::ProviderCatalogRepo::new(db.conn()),
        &crate::ai::router::KeyringCredentialSource,
        "kol_analysis",
    )
    .map_err(|e| err(e.to_string()))
}
pub async fn read(expert: i64, ids: Vec<i64>, force: bool) -> Result<Value, String> {
    if ids.is_empty() || ids.len() > 100 {
        return Err("请每次读取1–100份资料".into());
    }
    let prepared=tauri::async_runtime::spawn_blocking(move ||->DbResult<(Option<ResolvedModel>,Vec<Value>,usize)>{
  let db=Database::open(&crate::db::default_db_path())?;let mut pending=Vec::new();let mut local=0;
  for id in &ids{if let Ok(r)=get(&db,*id){if r["expert_id"]!=expert{return Err(err("资料归属与当前专家不一致"));}}}
  for id in ids {
   let Ok(record)=get(&db,id) else{continue};if record["expert_id"]!=expert{return Err(err("资料归属与当前专家不一致"));}
   if !force&&record["status"]=="ready"{local+=1;continue;}
   // Claim increments the version, so an older reader can never overwrite a retry.
   db.conn().execute("UPDATE kol_materials SET revision=revision+1,status='reading',error='' WHERE id=?1",[id])?;
   if !force && !db.conn().query_row("SELECT EXISTS(SELECT 1 FROM material_segments WHERE material_id=?1)",[id],|r|r.get::<_,bool>(0))? {match index_local(&db,&root(),id){Ok(true)=>{let item=get(&db,id)?;if item["status"]=="ready"{local+=1;continue;}},Ok(false)=>(),Err(e)=>{let item=get(&db,id)?;set_failure(&db,id,item["revision"].as_i64().unwrap(),&e.to_string(),false)?;continue;}}}
   pending.push(get(&db,id)?);
  }
  if pending.is_empty(){return Ok((None,pending,local));}
  match resolve(&db){Ok(model)=>Ok((Some(model),pending,local)),Err(e)=>{for r in pending{set_failure(&db,r["id"].as_i64().unwrap(),r["revision"].as_i64().unwrap(),&e.to_string(),false)?;}Ok((None,Vec::new(),local))}}
 }).await.map_err(|_|"资料读取任务中断")?.map_err(|e|e.to_string())?;
    let (Some(resolved), records, local) = prepared else {
        return Ok(json!({"local":prepared.2,"message":"资料状态已更新，可在资料列表查看详情"}));
    };
    let reader = format!(
        "model-reader-v1:{}:{}:{}:{}",
        resolved.connection.id,
        resolved.model.model_id,
        resolved.model.protocol,
        crate::cognition::digest(&resolved.model.capabilities_json)
    );
    let mut batch = Vec::<Value>::new();
    let mut parts = Vec::<AiAttachment>::new();
    let mut bytes = 0usize;
    let mut processed = local;
    for record in records {
        let copy = record.clone();
        let model = resolved.model.clone();
        let reader_copy = reader.clone();
        let prepared =
            tauri::async_runtime::spawn_blocking(move || -> DbResult<Option<AiAttachment>> {
                let db = Database::open(&crate::db::default_db_path())?;
                let id = copy["id"].as_i64().unwrap();
                let revision = copy["revision"].as_i64().unwrap();
                let current = match get(&db, id) {
                    Ok(r) if r["revision"] == revision => r,
                    _ => return Ok(None),
                };
                if !force && reuse_reading(&db, id, &reader_copy)? {
                    return Ok(None);
                }
                let _ = current;
                let mime = copy["media_type"].as_str().unwrap_or("");
                if let Err(error) = crate::ai::provider::adapters::attachment_supported(
                    &model,
                    mime,
                    copy["byte_size"].as_u64().unwrap_or(u64::MAX),
                ) {
                    set_failure(&db, id, revision, &error.to_string(), true)?;
                    return Ok(None);
                }
                let data = match blob_path(&db, &root(), &copy).and_then(|path| io(fs::read(path)))
                {
                    Ok(data) => data,
                    Err(e) => {
                        set_failure(&db, id, revision, &e.to_string(), false)?;
                        return Ok(None);
                    }
                };
                Ok(Some(AiAttachment {
                    filename: copy["filename"].as_str().unwrap_or("file").into(),
                    media_type: mime.into(),
                    data,
                }))
            })
            .await
            .map_err(|_| "资料读取任务中断")?
            .map_err(|e| e.to_string())?;
        if let Some(part) = prepared {
            if !parts.is_empty() && (parts.len() >= 4 || bytes + part.data.len() > 20 * 1024 * 1024)
            {
                processed += send(&resolved, &reader, &batch, &parts).await?;
                batch.clear();
                parts.clear();
                bytes = 0;
            }
            bytes += part.data.len();
            batch.push(record);
            parts.push(part);
        } else {
            processed += 1;
        }
    }
    if !parts.is_empty() {
        processed += send(&resolved, &reader, &batch, &parts).await?;
    }
    Ok(json!({"processed":processed,"message":"资料读取结束，请查看每份资料的覆盖范围与状态"}))
}
async fn send(
    resolved: &ResolvedModel,
    reader: &str,
    records: &[Value],
    parts: &[AiAttachment],
) -> Result<usize, String> {
    let manifest=records.iter().map(|r|json!({"id":r["id"],"filename":r["filename"],"description":r["description"],"file_hash":r["blob_hash"]})).collect::<Vec<_>>();
    let request = AiTextRequest {
        model_id: resolved.model.model_id.clone(),
        system: Some(include_str!("reading-spec.md").into()),
        messages: vec![AiMessage {
            role: "user".into(),
            content: json!({"files_in_attachment_order":manifest}).to_string(),
        }],
        temperature: Some(0.1),
        max_output_tokens: Some(8000),
        output_format: crate::ai::output::OutputFormat::PromptJson,
        budget: Default::default(),
    };
    let result = async {
        let key = if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1") {
            "synthetic-material-key".into()
        } else if resolved.connection.auth_mode == "none" {
            String::new()
        } else {
            crate::ai::provider::get_api_key(&resolved.connection.credential_ref)
                .map_err(|e| e.to_string())?
                .ok_or("请在设置中填写专家分析模型的 API Key")?
        };
        crate::ai::provider::adapters::complete_multimodal(
            &resolved.connection,
            &resolved.model,
            &key,
            &request,
            parts,
        )
        .await
        .map_err(|e| e.to_string())
    }
    .await;
    let records = records.to_vec();
    let reader = reader.to_string();
    tauri::async_runtime::spawn_blocking(move || -> Result<usize, String> {
        let db = Database::open(&crate::db::default_db_path()).map_err(|e| e.to_string())?;
        let result = result.and_then(|reply| {
            accept_model_reading(&db, &records, &reader, &reply.content).map_err(|e| e.to_string())
        });
        if let Err(error) = result {
            for record in &records {
                set_failure(
                    &db,
                    record["id"].as_i64().unwrap(),
                    record["revision"].as_i64().unwrap(),
                    &error,
                    false,
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(records.len())
    })
    .await
    .map_err(|_| "读取结果保存中断".to_string())?
}
