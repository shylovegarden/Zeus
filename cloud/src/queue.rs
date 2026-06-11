// Job queue for compilation tasks

use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{info, error, debug};
use crate::compile::{CompileRequest, compile_request, CompilationResult};

#[derive(Clone)]
pub struct CompilationQueue {
    sender: mpsc::Sender<QueueMessage>,
    jobs: Arc<Mutex<HashMap<String, JobInfo>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: String,
    pub status: JobStatus,
    pub request: CompileRequest,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<CompilationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

enum QueueMessage {
    NewJob { id: String, request: CompileRequest },
    JobComplete { id: String, result: CompilationResult },
    GetStatus { id: String, respond_to: tokio::sync::oneshot::Sender<Option<JobInfo>> },
}

impl CompilationQueue {
    pub async fn new() -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel::<QueueMessage>(100);
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let jobs_clone = jobs.clone();
        
        // Spawn queue processor
        tokio::spawn(async move {
            let mut workers = 0;
            let max_workers = 4; // Parallel compilation limit
            
            while let Some(msg) = receiver.recv().await {
                match msg {
                    QueueMessage::NewJob { id, request } => {
                        debug!("New job queued: {}", id);
                        
                        let job_info = JobInfo {
                            id: id.clone(),
                            status: JobStatus::Queued,
                            request: request.clone(),
                            created_at: chrono::Utc::now(),
                            completed_at: None,
                            result: None,
                        };
                        
                        {
                            let mut jobs = jobs_clone.lock().await;
                            jobs.insert(id.clone(), job_info);
                        }
                        
                        // Process job if workers available
                        if workers < max_workers {
                            workers += 1;
                            let jobs = jobs_clone.clone();
                            
                            tokio::spawn(async move {
                                Self::process_job(&id, &request, jobs).await;
                            });
                        }
                    }
                    
                    QueueMessage::JobComplete { id, result } => {
                        debug!("Job completed: {}", id);
                        workers -= 1;
                        
                        let mut jobs = jobs_clone.lock().await;
                        if let Some(job) = jobs.get_mut(&id) {
                            job.status = if result.success {
                                JobStatus::Completed
                            } else {
                                JobStatus::Failed
                            };
                            job.completed_at = Some(chrono::Utc::now());
                            job.result = Some(result);
                        }
                    }
                    
                    QueueMessage::GetStatus { id, respond_to } => {
                        let jobs = jobs_clone.lock().await;
                        let status = jobs.get(&id).cloned();
                        let _ = respond_to.send(status);
                    }
                }
            }
        });
        
        Ok(CompilationQueue { sender, jobs })
    }
    
    /// Submit a new compilation job
    pub async fn submit(&self, id: String, request: CompileRequest) -> Result<()> {
        self.sender
            .send(QueueMessage::NewJob { id, request })
            .await
            .context("Failed to queue job")?;
        
        Ok(())
    }
    
    /// Get job status
    pub async fn status(&self, id: &str) -> Result<Option<JobInfo>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        self.sender
            .send(QueueMessage::GetStatus {
                id: id.to_string(),
                respond_to: tx,
            })
            .await
            .context("Failed to send status request")?;
        
        rx.await.context("Failed to receive status")
    }
    
    /// Process a compilation job
    async fn process_job(id: &str, request: &CompileRequest, jobs: Arc<Mutex<HashMap<String, JobInfo>>>) {
        // Update status to running
        {
            let mut jobs = jobs.lock().await;
            if let Some(job) = jobs.get_mut(id) {
                job.status = JobStatus::Running;
            }
        }
        
        info!("Processing job: {}", id);
        
        // Run compilation
        match compile_request(request).await {
            Ok(result) => {
                info!("Job {} completed: success={}", id, result.success);
                
                // Store result
                let mut jobs = jobs.lock().await;
                if let Some(job) = jobs.get_mut(id) {
                    job.status = if result.success {
                        JobStatus::Completed
                    } else {
                        JobStatus::Failed
                    };
                    job.completed_at = Some(chrono::Utc::now());
                    job.result = Some(result);
                }
            }
            Err(e) => {
                error!("Job {} failed: {}", id, e);
                
                let mut jobs = jobs.lock().await;
                if let Some(job) = jobs.get_mut(id) {
                    job.status = JobStatus::Failed;
                    job.completed_at = Some(chrono::Utc::now());
                    job.result = Some(CompilationResult {
                        success: false,
                        binary: None,
                        certificate: None,
                        verification_report: None,
                        gas_estimate: None,
                        output: String::new(),
                        errors: vec![e.to_string()],
                    });
                }
            }
        }
    }
    
    /// Get all jobs
    pub async fn list_jobs(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.lock().await;
        jobs.values().cloned().collect()
    }
}
