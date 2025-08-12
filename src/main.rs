use anyhow::Result;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use colored::Colorize;
use serde_json;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};
use uuid::Uuid;

mod cli;
mod engine;
mod sandbox;

use cli::{Args, Commands};
use engine::RuneEngine;
use sandbox::SandboxManager;

const MAIN_RUNE_FILE: &str = "configure.rn";

fn format_result_output(result: &Result<String, String>, parse_json: bool) {
    match result {
        Ok(output) => {
            if parse_json {
                match serde_json::from_str::<serde_json::Value>(output) {
                    Ok(json_value) => {
                        println!(
                            "{} {} {}",
                            "Script is compiled and run successfully:".green(),
                            "Ok()".blue(),
                            "-> Parsed & Pretty".cyan()
                        );
                        if let Some(string_value) = json_value.as_str() {
                            println!("{}", string_value);
                        } else {
                            println!("{}", serde_json::to_string_pretty(&json_value).unwrap());
                        }
                    }
                    Err(_) => {
                        println!(
                            "{} {} {}",
                            "Script is compiled and run successfully:".green(),
                            "Ok()".blue(),
                            "->  Parsed & Pretty failed".yellow()
                        );
                    }
                }
            } else {
                println!(
                    "{} {}",
                    "Script is compiled and run successfully:".green(),
                    "Ok()".blue()
                );
                println!("{}", output);
            }
        }
        Err(error_msg) => {
            println!(
                "{} {}",
                "Script is compiled and run successfully:".green(),
                "Err()".red()
            );
            println!("{}", error_msg);
        }
    }
}

#[derive(Clone)]
struct AppState {
    rune_engine: Arc<RuneEngine>,
    sandbox_manager: Arc<SandboxManager>,
    context_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Listen {
            port,
            host,
            context,
            exec,
        } => run_server(port, host, context, exec).await,
        Commands::Collect {
            exec,
            context,
            parse,
        } => run_collect(exec, context, parse).await,
        Commands::Check {
            exec,
            input,
            context,
            parse,
        } => run_check(exec, input, context, parse).await,
    }
}

async fn run_server(
    port: u16,
    host: String,
    context: PathBuf,
    exec: Option<PathBuf>,
) -> Result<()> {
    // Determine Rune script path
    let rune_script_path = match exec {
        Some(path) => path,
        None => context.join(MAIN_RUNE_FILE),
    };

    // Check if file exists
    if !rune_script_path.exists() {
        eprintln!(
            "Error: Rune script file does not exist: {}",
            rune_script_path.display()
        );
        std::process::exit(1);
    }

    if !context.exists() {
        eprintln!(
            "Error: Context directory does not exist: {}",
            context.display()
        );
        std::process::exit(1);
    }

    println!("Startup parameters:");
    println!("  Server address: {}:{}", host, port);
    println!("  Context directory: {}", context.display());
    println!("  Rune script: {}", rune_script_path.display());

    // Initialize components
    let rune_engine = Arc::new(RuneEngine::new(&rune_script_path, &context).await?);
    let sandbox_manager = Arc::new(SandboxManager::new());

    let state = AppState {
        rune_engine,
        sandbox_manager,
        context_path: context.clone(),
    };

    // Create routes
    let app = Router::new()
        .route("/api/collect", get(handle_collect))
        .route("/api/submit", post(handle_submit))
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .with_state(state);

    let bind_address = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    println!("Server running at http://{}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn run_collect(exec: Option<PathBuf>, context: PathBuf, parse_json: bool) -> Result<()> {
    // Determine Rune script path
    let file = match exec {
        Some(path) => path,
        None => context.join(MAIN_RUNE_FILE),
    };

    if !file.exists() {
        eprintln!("Error: Rune script file does not exist: {}", file.display());
        std::process::exit(1);
    }

    if !context.exists() {
        eprintln!(
            "Error: Context directory does not exist: {}",
            context.display()
        );
        std::process::exit(1);
    }

    let rune_engine = RuneEngine::new(&file, &context).await?;
    let result = rune_engine.call_collect().await?;

    format_result_output(&result, parse_json);
    Ok(())
}

async fn run_check(
    exec: Option<PathBuf>,
    user_input: String,
    context: PathBuf,
    parse_json: bool,
) -> Result<()> {
    // Determine Rune script path
    let file = match exec {
        Some(path) => path,
        None => context.join(MAIN_RUNE_FILE),
    };

    if !file.exists() {
        eprintln!("Error: Rune script file does not exist: {}", file.display());
        std::process::exit(1);
    }

    if !context.exists() {
        eprintln!(
            "Error: Context directory does not exist: {}",
            context.display()
        );
        std::process::exit(1);
    }

    let rune_engine = RuneEngine::new(&file, &context).await?;
    let sandbox_manager = SandboxManager::new();

    // Create temporary sandbox
    let sandbox_id = uuid::Uuid::new_v4().to_string();
    // let sandbox = sandbox_manager.create_sandbox(&sandbox_id).await?;

    let result = rune_engine.call_check(&user_input).await?;

    // Clean up sandbox
    if let Err(err) = sandbox_manager.cleanup_sandbox(&sandbox_id).await {
        eprintln!("Warning: Failed to clean up sandbox: {}", err);
    }

    format_result_output(&result, parse_json);
    Ok(())
}

async fn handle_collect(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    match state.rune_engine.call_collect().await {
        Ok(result) => {
            // result is now a String, needs to be parsed as JSON or returned directly
            match result {
                Ok(json_str) => (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    json_str,
                )
                    .into_response(),
                Err(error_msg) => (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/plain")],
                    error_msg,
                )
                    .into_response(),
            }
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn handle_submit(
    axum::extract::State(state): axum::extract::State<AppState>,
    body: String,
) -> impl IntoResponse {
    // Create sandbox environment
    let sandbox_id = Uuid::new_v4().to_string();
    // let sandbox = match state.sandbox_manager.create_sandbox(&sandbox_id).await {
    //     Ok(sandbox) => sandbox,
    //     Err(err) => {
    //         return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
    //     }
    // };

    // Execute rune script in sandbox
    let deep_result = state.rune_engine.call_check(&body).await;

    // Clean up sandbox
    if let Err(err) = state.sandbox_manager.cleanup_sandbox(&sandbox_id).await {
        eprintln!("Failed to cleanup sandbox {}: {}", sandbox_id, err);
    }

    match deep_result {
        Ok(result) => {
            // output is now a String, try to parse as JSON
            match result {
                Ok(json_str) => (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    json_str,
                )
                    .into_response(),
                Err(error_msg) => (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/plain")],
                    error_msg,
                )
                    .into_response(),
            }
        }
        Err(err) => (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}
