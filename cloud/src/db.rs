// Database module for Zeus Cloud
// PostgreSQL operations

use sqlx::{Pool, Postgres, Row};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    pool: Pool<Postgres>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub source_code: String,
    pub target: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub binary_hash: Option<String>,
    pub certificate_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Certificate {
    pub hash: String,
    pub properties: Vec<String>,
    pub signature: String,
    pub created_at: DateTime<Utc>,
    pub source_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Stats {
    pub total_compilations: i64,
    pub successful_verifications: i64,
    pub certificates_issued: i64,
    pub active_users: i64,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/zeus_cloud".to_string());
        
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .context("Failed to connect to database")?;
        
        // Run migrations
        Self::run_migrations(&pool).await?;
        
        Ok(Database { pool })
    }
    
    async fn run_migrations(pool: &Pool<Postgres>) -> Result<()> {
        // Create jobs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id UUID PRIMARY KEY,
                status VARCHAR(20) NOT NULL DEFAULT 'queued',
                source_code TEXT NOT NULL,
                target VARCHAR(50) NOT NULL DEFAULT 'x86_64',
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                completed_at TIMESTAMP WITH TIME ZONE,
                output TEXT,
                error TEXT,
                binary_hash VARCHAR(64),
                certificate_hash VARCHAR(64)
            )
            "#
        )
        .execute(pool)
        .await?;
        
        // Create certificates table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS certificates (
                hash VARCHAR(64) PRIMARY KEY,
                properties TEXT[] NOT NULL,
                signature VARCHAR(128) NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                source_hash VARCHAR(64) NOT NULL
            )
            "#
        )
        .execute(pool)
        .await?;
        
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                email VARCHAR(255) UNIQUE NOT NULL,
                api_key VARCHAR(64) UNIQUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                last_active TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#
        )
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Create a new job
    pub async fn create_job(&self, source: &str, target: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT INTO jobs (id, source_code, target, status) VALUES ($1, $2, $3, 'queued')"
        )
        .bind(&id)
        .bind(source)
        .bind(target)
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    /// Get job by ID
    pub async fn get_job(&self, id: &str) -> Result<Option<Job>> {
        let row = sqlx::query(
            "SELECT * FROM jobs WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "queued" => JobStatus::Queued,
                    "running" => JobStatus::Running,
                    "completed" => JobStatus::Completed,
                    "failed" => JobStatus::Failed,
                    _ => JobStatus::Queued,
                };
                
                Ok(Some(Job {
                    id: row.get("id"),
                    status,
                    source_code: row.get("source_code"),
                    target: row.get("target"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    completed_at: row.get("completed_at"),
                    output: row.get("output"),
                    error: row.get("error"),
                    binary_hash: row.get("binary_hash"),
                    certificate_hash: row.get("certificate_hash"),
                }))
            }
            None => Ok(None),
        }
    }
    
    /// Update job status
    pub async fn update_job_status(&self, id: &str, status: JobStatus) -> Result<()> {
        let status_str = match status {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
        };
        
        sqlx::query(
            "UPDATE jobs SET status = $1, updated_at = NOW() WHERE id = $2"
        )
        .bind(status_str)
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Complete job with results
    pub async fn complete_job(
        &self,
        id: &str,
        output: Option<String>,
        error: Option<String>,
        binary_hash: Option<String>,
        certificate_hash: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs 
            SET status = $1, 
                updated_at = NOW(),
                completed_at = NOW(),
                output = $2,
                error = $3,
                binary_hash = $4,
                certificate_hash = $5
            WHERE id = $6
            "#
        )
        .bind("completed")
        .bind(output)
        .bind(error)
        .bind(binary_hash)
        .bind(certificate_hash)
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Store certificate
    pub async fn store_certificate(&self, cert: &Certificate) -> Result<()> {
        sqlx::query(
            "INSERT INTO certificates (hash, properties, signature, source_hash) VALUES ($1, $2, $3, $4)"
        )
        .bind(&cert.hash)
        .bind(&cert.properties)
        .bind(&cert.signature)
        .bind(&cert.source_hash)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get certificate by hash
    pub async fn get_certificate(&self, hash: &str) -> Result<Option<Certificate>> {
        let row = sqlx::query(
            "SELECT * FROM certificates WHERE hash = $1"
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                Ok(Some(Certificate {
                    hash: row.get("hash"),
                    properties: row.get("properties"),
                    signature: row.get("signature"),
                    created_at: row.get("created_at"),
                    source_hash: row.get("source_hash"),
                }))
            }
            None => Ok(None),
        }
    }
    
    /// Get system stats
    pub async fn get_stats(&self) -> Result<Stats> {
        let total_compilations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM jobs"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let successful_verifications: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM jobs WHERE status = 'completed' AND error IS NULL"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let certificates_issued: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM certificates"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let active_users: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE last_active > NOW() - INTERVAL '30 days'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        
        Ok(Stats {
            total_compilations,
            successful_verifications,
            certificates_issued,
            active_users,
        })
    }
}
