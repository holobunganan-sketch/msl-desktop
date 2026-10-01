use super::{
    protocol::{self, Api, ApiError},
    store,
};
use crate::{
    app_state::AppState,
    db::{now_unix, DbResult},
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

const KEYRING_SERVICE: &str = "MSLDesktop.Weixin";
const SAFE_CREDENTIAL_ERROR: &str = "微信凭据不可用，请重新扫码绑定";

struct PendingLogin {
    qrcode: String,
    base: String,
    started: Instant,
    code: Option<String>,
    busy: bool,
}
#[derive(Default)]
struct Control {
    generation: u64,
    pending: Option<PendingLogin>,
    handles: Vec<tauri::async_runtime::JoinHandle<()>>,
}
impl Control {
    fn cancel(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }
}
#[derive(Default)]
pub struct WeixinRuntime {
    control: Mutex<Control>,
}
impl WeixinRuntime {
    pub fn stop(&self) {
        let mut control = self.control.lock().unwrap_or_else(|p| p.into_inner());
        control.cancel();
    }
}
impl Drop for WeixinRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Serialize)]
pub struct WeixinStatus {
    pub bound: bool,
    pub enabled: bool,
    pub connection_state: String,
    pub last_received_at: Option<i64>,
}
#[derive(Serialize)]
pub struct LoginView {
    pub state: String,
    pub qr_image: Option<String>,
}
fn view(state: &str) -> LoginView {
    LoginView {
        state: state.into(),
        qr_image: None,
    }
}
fn isolated() -> bool {
    cfg!(test) || std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1")
}
fn live_only() -> Result<(), String> {
    if isolated() {
        Err("隔离测试环境不会连接真实微信账号".into())
    } else {
        Ok(())
    }
}
fn db<T>(app: &tauri::AppHandle, f: impl FnOnce(&Connection) -> DbResult<T>) -> Result<T, String> {
    app.state::<AppState>()
        .with_database(|database| f(database.conn()))
        .ok_or("本地数据库尚未就绪")?
        .map_err(|_| "微信入口的本地记录未能保存，请稍后重试".into())
}
fn credential(reference: &str) -> Result<keyring::Entry, String> {
    live_only()?;
    keyring::Entry::new(KEYRING_SERVICE, reference).map_err(|_| SAFE_CREDENTIAL_ERROR.into())
}
fn read_token(reference: &str) -> Result<Option<String>, String> {
    match credential(reference)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(SAFE_CREDENTIAL_ERROR.into()),
    }
}
fn delete_token(reference: &str) -> Result<(), String> {
    match credential(reference)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(SAFE_CREDENTIAL_ERROR.into()),
    }
}
fn connection_state(app: &tauri::AppHandle, value: &str) -> Result<(), String> {
    db(app, |conn| {
        conn.execute(
            "UPDATE weixin_binding SET connection_state=?1 WHERE id=1",
            [value],
        )?;
        Ok(())
    })
}

pub fn status(app: &tauri::AppHandle) -> Result<WeixinStatus, String> {
    db(app, |conn| {
        let binding = store::binding(conn)?;
        if let Some(binding) = binding {
            let (connection_state, last_received_at) = conn.query_row(
                "SELECT connection_state,last_received_at FROM weixin_binding WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            Ok(WeixinStatus {
                bound: true,
                enabled: binding.enabled,
                connection_state,
                last_received_at,
            })
        } else {
            Ok(WeixinStatus {
                bound: false,
                enabled: false,
                connection_state: "unbound".into(),
                last_received_at: None,
            })
        }
    })
}

/// Called after app state and database are ready. No network activity by default.
pub fn start(app: tauri::AppHandle) -> Result<(), String> {
    start_if_current(app, None)
}
fn start_if_current(app: tauri::AppHandle, expected_generation: Option<u64>) -> Result<(), String> {
    if isolated() {
        return Ok(());
    }
    let runtime = app.state::<WeixinRuntime>();
    let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
    if expected_generation.is_some_and(|expected| control.generation != expected) {
        return Ok(());
    }
    control.cancel();
    let Some(binding) = db(&app, store::binding)?.filter(|b| b.enabled) else {
        return Ok(());
    };
    let token = match read_token(&binding.credential_ref)? {
        Some(token) => token,
        None => {
            db(&app, |conn| {
                conn.execute(
                    "UPDATE weixin_binding SET enabled=0,connection_state='needs_login' WHERE id=1",
                    [],
                )?;
                Ok(())
            })?;
            return Err(SAFE_CREDENTIAL_ERROR.into());
        }
    };
    let api = Api::new(&binding.base_url, Some(token))?;
    connection_state(&app, "connecting")?;
    let generation = control.generation;
    let (ack_tx, ack_rx) = mpsc::sync_channel::<(String, String)>(32);
    let ack_api = api.clone();
    let owner = binding.owner_id.clone();
    control
        .handles
        .push(tauri::async_runtime::spawn(async move {
            loop {
                match ack_rx.try_recv() {
                    Ok((text, context)) => {
                        let _ = ack_api.acknowledge(&owner, &context, &text).await;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        tokio::time::sleep(Duration::from_millis(150)).await
                    }
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }));
    let worker_app = app.clone();
    control
        .handles
        .push(tauri::async_runtime::spawn(async move {
            poll_messages(worker_app, api, binding, generation, ack_tx).await;
        }));
    Ok(())
}

async fn poll_messages(
    app: tauri::AppHandle,
    api: Api,
    mut binding: store::Binding,
    generation: u64,
    ack: mpsc::SyncSender<(String, String)>,
) {
    let mut failures = 0u32;
    loop {
        if app.state::<AppState>().quit_requested() {
            break;
        }
        let result = api.updates(&binding.cursor).await;
        // Synchronous processing is serialized with disable/unlink and pairing.
        // No network await runs while the lifecycle mutex is held.
        let delay = {
            let runtime = app.state::<WeixinRuntime>();
            let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
            if control.generation != generation {
                return;
            }
            match result {
                Ok(value) => {
                    if let Ok((messages, cursor)) = protocol::update_batch(&value) {
                        match db(&app, |conn| {
                            store::ingest_batch(conn, &binding, messages, cursor, now_unix())
                        }) {
                            Ok(receipts) => {
                                if !cursor.is_empty() {
                                    binding.cursor = cursor.into();
                                }
                                failures = 0;
                                let _ = connection_state(&app, "connected");
                                let changed = !receipts.is_empty();
                                for receipt in receipts {
                                    if let Some(job) = receipt.job {
                                        match request_for_job(&job) {
                                            Ok(request) => {
                                                crate::commands::jobs::spawn_existing_job(
                                                    app.clone(),
                                                    job.id,
                                                    request,
                                                )
                                            }
                                            Err(error) => {
                                                let _ = db(&app, |conn| {
                                                    crate::db::jobs::finish(
                                                        conn,
                                                        job.id,
                                                        Err(error),
                                                    )
                                                });
                                            }
                                        }
                                    }
                                    if !receipt.context.is_empty() {
                                        let _ = ack.try_send((receipt.ack, receipt.context));
                                    }
                                }
                                if changed {
                                    let _ = app.emit("weixin-received", ());
                                }
                                Duration::from_millis(500)
                            }
                            Err(_) => {
                                failures = failures.saturating_add(1);
                                let _ = connection_state(&app, "reconnecting");
                                backoff(failures)
                            }
                        }
                    } else {
                        failures = failures.saturating_add(1);
                        let _ = connection_state(&app, "reconnecting");
                        backoff(failures)
                    }
                }
                Err(ApiError::SessionExpired) => {
                    let _ = db(&app, |conn| {
                        conn.execute("UPDATE weixin_binding SET enabled=0,connection_state='needs_login' WHERE id=1",[])?;
                        Ok(())
                    });
                    let _ = delete_token(&binding.credential_ref);
                    let _ = app.emit("weixin-received", ());
                    control.cancel();
                    return;
                }
                Err(_) => {
                    failures = failures.saturating_add(1);
                    let _ = connection_state(&app, "reconnecting");
                    backoff(failures)
                }
            }
        };
        tokio::time::sleep(delay).await;
    }
}
fn backoff(failures: u32) -> Duration {
    Duration::from_secs((1u64 << failures.min(6)).min(60))
}

fn request_for_job(
    job: &crate::db::jobs::AiJob,
) -> Result<crate::commands::jobs::JobRequest, String> {
    serde_json::from_value(serde_json::json!({"command":job.command,"args":job.args}))
        .map_err(|_| "后台任务参数无效，请从菜单重新提交。".into())
}

pub fn set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<WeixinStatus, String> {
    let runtime = app.state::<WeixinRuntime>();
    let generation = {
        let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
        control.cancel();
        let binding = db(&app, store::binding)?.ok_or("请先扫码绑定微信")?;
        if enabled {
            live_only()?;
            if read_token(&binding.credential_ref)?.is_none() {
                return Err(SAFE_CREDENTIAL_ERROR.into());
            }
        }
        db(&app, |conn| {
            conn.execute(
                "UPDATE weixin_binding SET enabled=?1,connection_state=?2 WHERE id=1",
                params![enabled, if enabled { "connecting" } else { "disabled" }],
            )?;
            Ok(())
        })?;
        control.generation
    };
    if enabled {
        start_if_current(app.clone(), Some(generation))?;
    }
    status(&app)
}
pub fn unlink(app: &tauri::AppHandle) -> Result<WeixinStatus, String> {
    let runtime = app.state::<WeixinRuntime>();
    let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
    control.cancel();
    if let Some(binding) = db(app, store::binding)? {
        db(app, |conn| {
            conn.execute(
                "UPDATE weixin_binding SET enabled=0,connection_state='disabled' WHERE id=1",
                [],
            )?;
            Ok(())
        })?;
        delete_token(&binding.credential_ref)?;
        db(app, |conn| {
            conn.execute("DELETE FROM weixin_binding WHERE id=1", [])?;
            Ok(())
        })?;
    }
    // Deduplication tombstones intentionally survive unlink/rebind.
    status(app)
}

pub async fn begin_login(app: tauri::AppHandle) -> Result<LoginView, String> {
    live_only()?;
    let runtime = app.state::<WeixinRuntime>();
    let generation = {
        let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
        control.cancel();
        db(&app, |conn| {
            conn.execute(
                "UPDATE weixin_binding SET enabled=0,connection_state='disabled' WHERE id=1",
                [],
            )?;
            Ok(())
        })?;
        control.generation
    };
    let value = Api::new(protocol::DEFAULT_BASE, None)?
        .request_qr()
        .await
        .map_err(|_| "微信扫码服务暂时不可用，请稍后重试")?;
    let qrcode = value["qrcode"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .ok_or("微信未返回有效二维码，请重试")?
        .to_owned();
    let qr_image = protocol::qr_image(
        value["qrcode_img_content"]
            .as_str()
            .ok_or("微信未返回有效二维码，请重试")?,
    )?;
    let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
    if control.generation != generation {
        return Err("扫码已取消，请重新开始".into());
    }
    control.pending = Some(PendingLogin {
        qrcode,
        base: protocol::DEFAULT_BASE.into(),
        started: Instant::now(),
        code: None,
        busy: false,
    });
    Ok(LoginView {
        state: "wait".into(),
        qr_image: Some(qr_image),
    })
}

pub fn submit_code(app: &tauri::AppHandle, code: String) -> Result<(), String> {
    if code.len() < 4 || code.len() > 12 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err("请输入手机显示的数字验证码".into());
    }
    let runtime = app.state::<WeixinRuntime>();
    let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
    let pending = control.pending.as_mut().ok_or("请先获取二维码")?;
    pending.code = Some(code);
    Ok(())
}

pub async fn poll_login(app: tauri::AppHandle) -> Result<LoginView, String> {
    live_only()?;
    let runtime = app.state::<WeixinRuntime>();
    let (generation, base, qrcode, code) = {
        let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
        let generation = control.generation;
        let pending = control.pending.as_mut().ok_or("请先获取二维码")?;
        if pending.started.elapsed() > Duration::from_secs(300) {
            control.pending = None;
            return Ok(view("expired"));
        }
        if pending.busy {
            return Ok(view("wait"));
        }
        pending.busy = true;
        (
            generation,
            pending.base.clone(),
            pending.qrcode.clone(),
            pending.code.clone(),
        )
    };
    let result = Api::new(&base, None)?
        .poll_qr(&qrcode, code.as_deref())
        .await;
    let completed = {
        let mut control = runtime.control.lock().unwrap_or_else(|p| p.into_inner());
        if control.generation != generation {
            return Err("扫码已取消，请重新开始".into());
        }
        let pending = control.pending.as_mut().ok_or("扫码已取消")?;
        pending.busy = false;
        if pending.started.elapsed() > Duration::from_secs(300) {
            control.pending = None;
            return Ok(view("expired"));
        }
        let value = match result {
            Ok(value) => value,
            Err(_) => return Ok(view("wait")),
        };
        match value["status"].as_str().unwrap_or("") {
            "wait" => return Ok(view("wait")),
            "scaned" => {
                pending.code = None;
                return Ok(view("scaned"));
            }
            "need_verifycode" => {
                if pending.code == code {
                    pending.code = None;
                }
                return Ok(view("need_verifycode"));
            }
            "scaned_but_redirect" => {
                let host = value["redirect_host"]
                    .as_str()
                    .ok_or("微信返回的连接地址无效")?;
                pending.base = protocol::validate_base(&format!("https://{host}"))?.to_string();
                return Ok(view("scaned"));
            }
            state @ ("expired" | "verify_code_blocked" | "binded_redirect") => {
                let result = view(state);
                control.pending = None;
                return Ok(result);
            }
            "confirmed" => {
                let confirmed = protocol::confirmation(&value, &pending.base)?;
                let existing = db(&app, store::binding)?;
                let reference = existing
                    .as_ref()
                    .map(|b| b.credential_ref.clone())
                    .unwrap_or_else(|| format!("weixin-{}", uuid::Uuid::new_v4()));
                let old_token = read_token(&reference)?;
                credential(&reference)?
                    .set_password(&confirmed.token)
                    .map_err(|_| SAFE_CREDENTIAL_ERROR)?;
                let saved = db(&app, |conn| {
                    conn.execute("INSERT INTO weixin_binding(id,bot_id,owner_id,base_url,credential_ref,bound_at,enabled,cursor,connection_state) VALUES(1,?1,?2,?3,?4,?5,1,'','connecting') ON CONFLICT(id) DO UPDATE SET bot_id=excluded.bot_id,owner_id=excluded.owner_id,base_url=excluded.base_url,credential_ref=excluded.credential_ref,bound_at=excluded.bound_at,enabled=1,cursor='',connection_state='connecting',last_received_at=NULL,last_global_at=NULL,last_brief_at=NULL,menu_mode=NULL,menu_expires_at=NULL,menu_after_ms=0,menu_after_id='0'",params![confirmed.bot_id,confirmed.owner_id,confirmed.base,reference,now_unix()])?;
                    Ok(())
                });
                if saved.is_err() {
                    if let Some(old) = old_token {
                        let _ = credential(&reference)?.set_password(&old);
                    } else {
                        let _ = delete_token(&reference);
                    }
                    saved?;
                }
                control.pending = None;
                true
            }
            _ => return Err("微信返回了未支持的扫码状态，请重新扫码".into()),
        }
    };
    if completed {
        start_if_current(app, Some(generation))?;
    }
    Ok(view("confirmed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    // Dispatching all receipts as global analysis would run the wrong task for a brief.
    #[test]
    fn menu_receipt_dispatches_the_persisted_job_kind_and_arguments() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let args = serde_json::json!({"date":"2026-10-01","periodStart":1790784000i64,"periodEnd":1790870400i64,"todayStart":1790870400i64,"todayEnd":1790956800i64,"locale":"zh-CN","force":false});
        let (job, _) = crate::db::jobs::start(db.conn(), "generate_brief", &args).unwrap();
        let dispatched = serde_json::to_value(request_for_job(&job).unwrap()).unwrap();
        assert_eq!(dispatched["command"], "generate_brief");
        assert_eq!(dispatched["args"], args);
    }
    #[test]
    fn cancel_invalidates_late_pairing_and_clears_pending_secrets() {
        let mut control = Control {
            generation: 7,
            pending: Some(PendingLogin {
                qrcode: "synthetic-qr".into(),
                base: protocol::DEFAULT_BASE.into(),
                started: Instant::now(),
                code: Some("123456".into()),
                busy: true,
            }),
            handles: vec![],
        };
        control.cancel();
        assert_ne!(control.generation, 7);
        assert!(control.pending.is_none());
    }
}
