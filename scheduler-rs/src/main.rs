// scheduler-rs/src/main.rs
// Task Scheduler Service - CRON-based task scheduling
// Port 50066

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::str::FromStr;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    scheduler_service_server::{SchedulerService, SchedulerServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    ScheduleTaskRequest,
    ScheduleTaskResponse,
    ListTasksRequest,
    ListTasksResponse,
    ScheduledTask,
    CancelTaskRequest,
    CancelTaskResponse,
    HealthRequest,
    HealthResponse,
};

/// Internal task representation
#[derive(Debug, Clone)]
struct TaskEntry {
    task_id: String,
    task_name: String,
    cron_expression: String,
    payload: String,
    status: String,
    next_run_time: DateTime<Utc>,
    last_run_time: Option<DateTime<Utc>>,
    run_count: i32,
    metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub struct SchedulerServer {
    tasks: Arc<RwLock<HashMap<String, TaskEntry>>>,
}

impl Default for SchedulerServer {
    fn default() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SchedulerServer {
    /// Calculate next run time from cron expression
    fn calculate_next_run(cron_expr: &str) -> Result<DateTime<Utc>, String> {
        let schedule = Schedule::from_str(cron_expr)
            .map_err(|e| format!("Invalid cron expression: {}", e))?;
        
        schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| "No upcoming schedule found".to_string())
    }
}

#[tonic::async_trait]
impl SchedulerService for SchedulerServer {
    async fn schedule_task(
        &self,
        request: Request<ScheduleTaskRequest>,
    ) -> Result<Response<ScheduleTaskResponse>, Status> {
        let req = request.into_inner();
        
        log::info!("ScheduleTask: name='{}', cron='{}'", req.task_name, req.cron_expression);
        
        // Validate cron expression
        let next_run = match Self::calculate_next_run(&req.cron_expression) {
            Ok(time) => time,
            Err(e) => {
                return Ok(Response::new(ScheduleTaskResponse {
                    success: false,
                    scheduled_id: String::new(),
                    next_run_time: String::new(),
                    error: e,
                }));
            }
        };
        
        // Generate task ID if not provided
        let task_id = if req.task_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            req.task_id.clone()
        };
        
        // Set priority based on source service
        let priority = if req.metadata.get("source_service").map(|s| s.as_str()) == Some("curiosity-engine-rs") {
            log::info!("Setting HIGH priority (8) for Curiosity Engine task");
            8
        } else {
            5 // Default priority
        };

        let entry = TaskEntry {
            task_id: task_id.clone(),
            task_name: req.task_name,
            cron_expression: req.cron_expression,
            payload: req.payload,
            status: "ACTIVE".to_string(),
            next_run_time: next_run,
            last_run_time: None,
            run_count: 0,
            metadata: {
                let mut meta = req.metadata;
                meta.insert("priority".to_string(), priority.to_string());
                meta
            },
        };
        
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), entry);
        
        log::info!("Task scheduled: id={}, next_run={}", task_id, next_run.to_rfc3339());
        
        Ok(Response::new(ScheduleTaskResponse {
            success: true,
            scheduled_id: task_id,
            next_run_time: next_run.to_rfc3339(),
            error: String::new(),
        }))
    }
    
    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();
        let tasks = self.tasks.read().await;
        
        let mut result: Vec<ScheduledTask> = tasks
            .values()
            .filter(|t| {
                if req.filter.is_empty() {
                    true
                } else {
                    t.status.contains(&req.filter) || t.task_name.contains(&req.filter)
                }
            })
            .map(|t| ScheduledTask {
                task_id: t.task_id.clone(),
                task_name: t.task_name.clone(),
                cron_expression: t.cron_expression.clone(),
                status: t.status.clone(),
                next_run_time: t.next_run_time.to_rfc3339(),
                last_run_time: t.last_run_time.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                run_count: t.run_count,
            })
            .collect();
        
        if req.limit > 0 {
            result.truncate(req.limit as usize);
        }
        
        let total = result.len() as i32;
        
        Ok(Response::new(ListTasksResponse {
            tasks: result,
            total_count: total,
        }))
    }
    
    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<CancelTaskResponse>, Status> {
        let req = request.into_inner();
        
        log::info!("CancelTask: id='{}'", req.task_id);
        
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(&req.task_id) {
            task.status = "CANCELLED".to_string();
            log::info!("Task cancelled: {}", req.task_id);
            Ok(Response::new(CancelTaskResponse {
                success: true,
                error: String::new(),
            }))
        } else {
            Ok(Response::new(CancelTaskResponse {
                success: false,
                error: format!("Task not found: {}", req.task_id),
            }))
        }
    }
}

#[tonic::async_trait]
impl HealthService for SchedulerServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let tasks = self.tasks.read().await;
        let active_count = tasks.values().filter(|t| t.status == "ACTIVE").count();
        
        let mut dependencies = HashMap::new();
        dependencies.insert("scheduler_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("active_tasks".to_string(), active_count.to_string());
        
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "scheduler-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addr_str = env::var("SCHEDULER_SERVICE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50066".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str.strip_prefix("http://").unwrap_or(&addr_str).parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let scheduler_server = Arc::new(SchedulerServer::default());
    let sched_for_health = scheduler_server.clone();

    log::info!("Scheduler Service starting on {}", addr);
    println!("Scheduler Service listening on {}", addr);

    Server::builder()
        .add_service(SchedulerServiceServer::from_arc(scheduler_server))
        .add_service(HealthServiceServer::from_arc(sched_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
