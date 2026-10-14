use crate::{
    db::Store,
    domain::{self, AppError, ImportFailure, ImportJob, Result},
    image_pipeline,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Mutex;

type ImportJobs = Arc<Mutex<HashMap<String, (ImportJob, Arc<AtomicBool>)>>>;

#[derive(Clone, Default)]
pub struct ImportManager {
    jobs: ImportJobs,
}

impl ImportManager {
    pub async fn start(
        &self,
        store: Store,
        cache: PathBuf,
        project: String,
        paths: Vec<PathBuf>,
        emit: impl Fn(&ImportJob) + Send + Sync + 'static,
    ) -> Result<String> {
        store.get(&project).await?;
        let job_id = domain::id();
        let job = ImportJob {
            job_id: job_id.clone(),
            project_id: project,
            status: "running".into(),
            total: 0,
            completed: 0,
            imported: 0,
            skipped: 0,
            failures: Vec::new(),
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut jobs = self.jobs.lock().await;
            if jobs.values().any(|(current, _)| {
                current.project_id == job.project_id && current.status == "running"
            }) {
                return Err(AppError::new(
                    "IMPORT_BUSY",
                    "此项目已有导入任务，请等待或取消",
                ));
            }
            jobs.insert(job_id.clone(), (job.clone(), cancelled.clone()));
        }
        let manager = self.clone();
        tokio::spawn(async move {
            let mut job = job;
            let failure_path = paths
                .first()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            let scan_cancelled = cancelled.clone();
            let discovered =
                tokio::task::spawn_blocking(move || expand(paths, &scan_cancelled)).await;
            let paths = match discovered {
                Ok(Ok(paths)) => paths,
                failure => {
                    let error = match failure {
                        Ok(Err(error)) => error,
                        Err(error) => AppError::new("BACKGROUND", error.to_string()),
                        _ => unreachable!(),
                    };
                    job.failures.push(ImportFailure {
                        path: failure_path,
                        code: error.code,
                        message: error.message,
                    });
                    job.status = "completed".into();
                    manager
                        .jobs
                        .lock()
                        .await
                        .insert(job.job_id.clone(), (job.clone(), cancelled));
                    emit(&job);
                    return;
                }
            };
            job.total = paths.len();
            manager
                .jobs
                .lock()
                .await
                .insert(job.job_id.clone(), (job.clone(), cancelled.clone()));
            emit(&job);
            for path in paths {
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                let source = path.clone();
                let directory = cache.clone();
                let prepared = tokio::task::spawn_blocking(move || {
                    image_pipeline::prepare(&source, &directory)
                })
                .await;
                // 取消时不提交仍在解码中的这一张，已提交的照片保留。
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                let result = match prepared {
                    Ok(Ok(prepared)) => store.insert_photo(&job.project_id, &prepared).await,
                    Ok(Err(error)) => Err(error),
                    Err(error) => Err(AppError::new("BACKGROUND", error.to_string())),
                };
                match result {
                    Ok(true) => job.imported += 1,
                    Ok(false) => job.skipped += 1,
                    Err(error) => job.failures.push(ImportFailure {
                        path: path.to_string_lossy().into(),
                        code: error.code,
                        message: error.message,
                    }),
                }
                job.completed += 1;
                manager
                    .jobs
                    .lock()
                    .await
                    .insert(job.job_id.clone(), (job.clone(), cancelled.clone()));
                emit(&job);
            }
            job.status = if cancelled.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "completed"
            }
            .into();
            manager
                .jobs
                .lock()
                .await
                .insert(job.job_id.clone(), (job.clone(), cancelled));
            emit(&job);
        });
        Ok(job_id)
    }

    pub async fn get(&self, id: &str) -> Result<ImportJob> {
        self.jobs
            .lock()
            .await
            .get(id)
            .map(|(job, _)| job.clone())
            .ok_or_else(|| AppError::new("NOT_FOUND", "导入任务不存在；重启后已导入照片仍在项目中"))
    }
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let jobs = self.jobs.lock().await;
        let (_, cancelled) = jobs
            .get(id)
            .ok_or_else(|| AppError::new("NOT_FOUND", "导入任务不存在"))?;
        cancelled.store(true, Ordering::Release);
        Ok(())
    }
}

fn expand(paths: Vec<PathBuf>, cancelled: &AtomicBool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let root = path.canonicalize()?;
        if root.is_dir() {
            scan(&root, &root, &mut files, cancelled)?;
        } else {
            files.push(root);
        }
    }
    files.sort();
    files.dedup();
    if files.len() > 10_000 {
        return Err(AppError::new(
            "IMPORT_LIMIT",
            "单次最多导入 10000 张照片，请分批选择",
        ));
    }
    Ok(files)
}
fn scan(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    cancelled: &AtomicBool,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        let entry = entry?;
        let kind = entry.file_type()?;
        // 不跟随符号链接，也不扫描越出系统选择目录的路径。
        if kind.is_symlink() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path().canonicalize()?;
        if !path.starts_with(root) {
            continue;
        }
        if kind.is_dir() {
            scan(root, &path, files, cancelled)?;
        } else if image_pipeline::EXTENSIONS.contains(
            &path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
        ) {
            files.push(path);
        }
        if files.len() > 10_000 {
            return Err(AppError::new("IMPORT_LIMIT", "单次最多导入 10000 张照片"));
        }
    }
    Ok(())
}
