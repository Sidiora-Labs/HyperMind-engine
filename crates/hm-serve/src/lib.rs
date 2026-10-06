#![forbid(unsafe_code)]

pub mod actor;
pub mod admin;
pub mod anticipation;
pub mod auth;
pub mod config;
pub mod embedded;
pub mod errors;
pub mod grpc;
pub mod leases;
pub mod protocol;
pub mod requests;
pub mod rest;
pub mod telemetry;
pub mod uds;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod hypermid_import;
pub mod hypermid_context_import;

pub mod session_context;
pub mod context_config;
pub mod context_history;
pub mod context_jobs;
pub mod context_projection;
pub mod context_memory;
pub mod context_retrieval;

pub mod development_admission;

pub mod model_inventory;

pub mod development_profile;

pub mod development_mapping;

pub mod development_conditions;

pub mod development_curation;

pub mod development_historian;

pub mod development_scheduler;

pub mod development_retrospective;

pub mod development_service;
