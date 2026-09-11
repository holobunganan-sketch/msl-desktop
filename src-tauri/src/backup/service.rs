//! Folder backup scheduler. Cloud upload status belongs to the user's sync client.
use super::{
    archive::{self, err, Result},
    restore::write_json,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
static OPERATION: Mutex<()> = Mutex::new(());
pub static RUNNING: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub directory: String,
    pub enabled: bool,
    pub interval_minutes: u32,
    pub keep_count: usize,
    pub owner: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            directory: String::new(),
            enabled: false,
            interval_minutes: 1440,
            keep_count: 7,
            owner: uuid::Uuid::new_v4().to_string(),
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub last_attempt: i64,
    pub last_success: i64,
    pub last_file: String,
    pub error: String,
    pub warning: String,
    #[serde(default)]
    pub files: Vec<String>,
}
pub fn config(data: &Path) -> Result<Config> {
    let file = data.join("backup-settings.json");
    if !file.exists() {
        return Ok(Config::default());
    }
    let c: Config = serde_json::from_slice(&fs::read(file).map_err(err)?).map_err(err)?;
    if !(30..=10080).contains(&c.interval_minutes)
        || !(1..=100).contains(&c.keep_count)
        || c.owner.is_empty()
        || c.owner.len() > 100
    {
        return Err("备份设置无效，已停止自动备份和旧备份清理。请检查备份设置文件。".into());
    }
    Ok(c)
}
pub fn state(data: &Path) -> Result<State> {
    let file = data.join("backup-state.json");
    if !file.exists() {
        return Ok(State::default());
    }
    serde_json::from_slice(&fs::read(file).map_err(err)?).map_err(err)
}
fn normalized(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .trim_end_matches('\\')
        .to_lowercase()
}
pub fn validate_destination(
    folder: &Path,
    protected: &[PathBuf],
    allow_test_repo: bool,
) -> Result<()> {
    if !folder.is_absolute() || !folder.is_dir() {
        return Err("请选择已存在、可在本机访问的备份文件夹".into());
    }
    crate::storage::paths::reject_link_components(folder)?;
    let candidate = normalized(folder);
    for p in protected {
        let p = normalized(p);
        if candidate == p
            || candidate.starts_with(&format!("{p}\\"))
            || p.starts_with(&format!("{candidate}\\"))
        {
            return Err("备份文件夹须与应用数据、安装目录和工作目录分开，且不能相互包含".into());
        }
    }
    if !allow_test_repo
        && folder
            .canonicalize()
            .map_err(err)?
            .ancestors()
            .any(|p| p.join(".git").exists())
    {
        if !private_git_folder(folder)? {
            return Err("GIT_PRIVATE_FOLDER_REQUIRED：该目录被上级 Git 仓库覆盖。可点击“创建私有云盘子文件夹”，将备份和同步数据排除在 Git 提交之外。请勿强制添加私有目录。".into());
        }
    }
    Ok(())
}

fn git_read(folder: &Path, args: &[&str]) -> Result<std::process::Output> {
    let mut command = std::process::Command::new("git");
    command
        .args(["-c", "core.fsmonitor=false", "-C"])
        .arg(folder)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .output()
        .map_err(|_| "无法核验上级 Git 仓库的排除规则；请安装 Git 或选择仓库之外的云盘目录".into())
}

fn private_git_folder(folder: &Path) -> Result<bool> {
    // A complete local exclusion is intentional: partial filename patterns cannot
    // protect every present/future sync file. Already tracked files bypass ignores.
    let ignore = folder.join(".gitignore");
    crate::storage::paths::reject_link_components(&ignore)?;
    if !ignore.is_file() || fs::metadata(&ignore).map_err(err)?.len() > 16 {
        return Ok(false);
    }
    if fs::read_to_string(ignore).map_err(err)?.trim() != "*" {
        return Ok(false);
    }
    let tracked = git_read(folder, &["ls-files", "-z", "--", "."])?;
    if !tracked.status.success() || !tracked.stdout.is_empty() {
        return Ok(false);
    }
    let ignored = git_read(
        folder,
        &["check-ignore", "--quiet", "--no-index", "--", ".gitignore"],
    )?;
    Ok(ignored.status.success())
}

/// User-selected, app-owned child only. Never change an existing ignore file,
/// parent repository configuration, or existing work files.
pub fn prepare_private_cloud_folder(data: &Path, parent: &Path) -> Result<PathBuf> {
    match validate_for_app(data, parent) {
        Ok(()) => (),
        Err(error) if error.starts_with("GIT_PRIVATE_FOLDER_REQUIRED") => (),
        Err(error) => return Err(error),
    }
    let target = parent.join("MSLDesktop-private");
    crate::storage::paths::reject_link_components(&target)?;
    // Check the index before creating anything, including deleted tracked files.
    if parent
        .canonicalize()
        .map_err(err)?
        .ancestors()
        .any(|p| p.join(".git").exists())
    {
        let tracked = git_read(parent, &["ls-files", "-z", "--", "MSLDesktop-private"])?;
        if !tracked.status.success() || !tracked.stdout.is_empty() {
            return Err("私有子目录中已有 Git 跟踪记录，请选择另一个云盘文件夹".into());
        }
    }
    if target.exists() {
        if private_git_folder(&target)? {
            validate_for_app(data, &target)?;
            return Ok(target);
        }
        return Err(
            "MSLDesktop-private 已存在且无法验证为私有目录，现有文件保持原样；请选择另一个文件夹"
                .into(),
        );
    }
    fs::create_dir(&target).map_err(err)?;
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target.join(".gitignore"))
        .map_err(err)?;
    file.write_all(b"*\n").map_err(err)?;
    file.sync_all().map_err(err)?;
    validate_for_app(data, &target)?;
    Ok(target)
}
pub fn validate_for_app(data: &Path, destination: &Path) -> Result<()> {
    let mut protected = vec![data.to_path_buf(), crate::storage::paths::local_app_root()];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            protected.push(p.to_path_buf());
        }
    }
    let db = archive::read_db(&data.join(crate::db::DB_FILE_NAME))?;
    let mut q = db
        .prepare("SELECT root_path FROM workspaces")
        .map_err(err)?;
    for p in q.query_map([], |r| r.get::<_, String>(0)).map_err(err)? {
        protected.push(p.map_err(err)?.into());
    }
    let test = (cfg!(test) || std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1"))
        && destination
            .components()
            .any(|c| c.as_os_str() == ".test-runtime");
    validate_destination(destination, &protected, test)
}
pub fn save(
    data: &Path,
    directory: String,
    enabled: bool,
    interval_minutes: u32,
    keep_count: usize,
) -> Result<Config> {
    let _lock = OPERATION
        .try_lock()
        .map_err(|_| "正在备份或准备恢复，请稍后再修改设置")?;
    if !(30..=10080).contains(&interval_minutes) || !(1..=100).contains(&keep_count) {
        return Err("备份间隔为 30–10080 分钟，保留数量为 1–100 份".into());
    }
    if !directory.is_empty() {
        validate_for_app(data, Path::new(&directory))?;
    } else if enabled {
        return Err("请先选择备份文件夹".into());
    }
    let mut c = config(data)?;
    c.directory = directory;
    c.enabled = enabled;
    c.interval_minutes = interval_minutes;
    c.keep_count = keep_count;
    write_json(&data.join("backup-settings.json"), &c)?;
    Ok(c)
}
pub fn prune(data: &Path, c: &Config, s: &mut State) -> Result<()> {
    // Only exact files recorded as published by this installation qualify.
    let mut owned = Vec::new();
    for value in &s.files {
        let p = PathBuf::from(value);
        if p.parent().map(normalized) != Some(normalized(Path::new(&c.directory))) {
            continue;
        }
        if !p.file_name().is_some_and(|n| {
            n.to_string_lossy().starts_with("msl-backup-")
                && n.to_string_lossy().ends_with(".mslbackup")
        }) {
            continue;
        }
        if let Ok(m) = archive::inspect(&p) {
            if m.owner == c.owner {
                owned.push((m.created_at, p));
            }
        }
    }
    // The durable ledger records completion order; wall clocks can repeat or go backwards.
    owned.sort_by_key(|(_, p)| p.to_string_lossy() == s.last_file);
    let remove = owned.len().saturating_sub(c.keep_count);
    for (_, p) in owned.into_iter().take(remove) {
        let check = data.join(format!(".retention-{}", uuid::Uuid::new_v4()));
        archive::unpack(&p, &check)?;
        fs::remove_dir_all(check).map_err(err)?;
        fs::remove_file(&p).map_err(err)?;
        s.files.retain(|v| Path::new(v) != p);
    }
    Ok(())
}
pub fn run(data: &Path) -> Result<PathBuf> {
    let _lock = OPERATION
        .try_lock()
        .map_err(|_| "已有备份或恢复操作正在执行")?;
    let c = config(data)?;
    if c.directory.is_empty() {
        return Err("请先选择备份文件夹".into());
    }
    validate_for_app(data, Path::new(&c.directory))?;
    if data.join("restore-pending.json").exists() {
        return Err("请先完成待执行恢复".into());
    }
    struct Running;
    impl Drop for Running {
        fn drop(&mut self) {
            RUNNING.store(false, Ordering::Release);
        }
    }
    RUNNING.store(true, Ordering::Release);
    let _running = Running;
    let mut s = state(data)?;
    s.last_attempt = crate::db::now_unix();
    s.error.clear();
    s.warning.clear();
    write_json(&data.join("backup-state.json"), &s)?;
    let result = archive::create_snapshot(data, Path::new(&c.directory), &c.owner);
    match &result {
        Ok(p) => {
            s.last_success = crate::db::now_unix();
            s.last_file = p.to_string_lossy().into();
            s.files.push(s.last_file.clone());
            write_json(&data.join("backup-state.json"), &s)?;
            if let Err(e) = prune(data, &c, &mut s) {
                s.warning = format!("新备份已完成，旧备份清理暂未完成：{e}");
            }
        }
        Err(e) => s.error = e.clone(),
    };
    write_json(&data.join("backup-state.json"), &s)?;
    result
}
pub fn stage(data: &Path, path: &Path) -> Result<String> {
    let _lock = OPERATION
        .try_lock()
        .map_err(|_| "已有备份或恢复操作正在执行")?;
    super::restore::stage(data, path)
}
pub fn confirm(data: &Path, token: &str, name: &str) -> Result<()> {
    let _lock = OPERATION
        .try_lock()
        .map_err(|_| "已有备份或恢复操作正在执行")?;
    super::restore::confirm(data, token, name)
}
pub fn spawn() {
    tauri::async_runtime::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = tauri::async_runtime::spawn_blocking(|| {
                let data = crate::db::default_app_data_dir();
                let _ = super::restore::retry_cleanup(&data);
                let Ok(c) = config(&data) else { return };
                let Ok(s) = state(&data) else { return };
                if c.enabled
                    && !c.directory.is_empty()
                    && crate::db::now_unix() - s.last_attempt >= i64::from(c.interval_minutes) * 60
                {
                    if let Err(e) = run(&data) {
                        if !RUNNING.load(Ordering::Acquire) {
                            let mut s = state(&data).unwrap_or_default();
                            s.last_attempt = crate::db::now_unix();
                            s.error = e;
                            let _ = write_json(&data.join("backup-state.json"), &s);
                        }
                    }
                }
            })
            .await;
        }
    });
}
