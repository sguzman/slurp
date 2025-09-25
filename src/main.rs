use std::fs;
use std::time::Duration;

use clap::{ArgAction, Parser};
use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde_json::{self as json, Value};
use tracing::{debug, error, info, warn};

mod log; // uses src/log.rs

/// Slurp: chunk JSON items and INSERT them into a SurrealDB table.
///
/// Reads a JSON array from --data payload.json, splits it into --batch sized
/// chunks, and performs parallel INSERTs into SurrealDB using SurrealQL.
/// Connection URL is taken from env SURREAL_URL or defaults to http://localhost:8000.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// SurrealDB namespace
    #[arg(long = "ns")]
    ns: String,

    /// SurrealDB database
    #[arg(long = "db")]
    db: String,

    /// Destination table name
    #[arg(long = "table")]
    table: String,

    /// Path to a JSON array file (e.g., [ {..}, {..}, ... ])
    #[arg(long = "data")]
    data_path: String,

    /// Batch size (number of items per INSERT)
    #[arg(long = "batch", default_value_t = 500, value_parser = clap::value_parser!(usize).range(1..))]
    batch: usize,

    /// Number of parallel worker threads
    #[arg(long = "thread", default_value_t = 4, value_parser = clap::value_parser!(usize).range(1..))]
    threads: usize,

    /// Verbosity level: 0=warn, 1=info, 2=debug
    #[arg(long = "verbosity", default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=2))]
    verbosity: u8,

    /// Dry-run: parse and show what would be inserted, but do not send requests
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Init colorized, timestamped logging
    log::init(log::level_from_verbosity(args.verbosity));

    // Resolve Surreal endpoint URL
    let url = std::env::var("SURREAL_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let sql_endpoint = format!("{}/sql", url);

    info!("loading JSON: {}", args.data_path);
    let raw = fs::read_to_string(&args.data_path)?;
    let value: Value = json::from_str(&raw)?;

    // Expect a JSON array of objects
    let items = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("input must be a JSON array"))?
        .iter()
        .cloned()
        .collect::<Vec<Value>>();

    if items.is_empty() {
        warn!("no items found in input; nothing to insert");
        return Ok(());
    }

    // Prepare the batches as immutable chunks
    let batches: Vec<Vec<Value>> = items.chunks(args.batch).map(|c| c.to_vec()).collect();

    info!(
        "items: {}, batch size: {}, batches: {}, threads: {}",
        items.len(),
        args.batch,
        batches.len(),
        args.threads
    );

    if args.dry_run {
        info!("dry-run enabled; not sending INSERTs");
    }

    // Shared HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    // Build a rayon pool with the requested thread count and run the work inside it
    let (ok_count, err_count): (usize, usize) = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build()?
        .install(|| {
            // Process in parallel; each batch maps to a Result
            batches
                .par_iter()
                .enumerate()
                .map(|(idx, batch)| {
                    let stmt = build_insert_stmt(&args.table, batch)?;
                    debug!("batch #{idx}: stmt size={}", stmt.len());

                    if args.dry_run {
                        info!("DRY batch #{idx}: {} records", batch.len());
                        return Ok(());
                    }

                    // POST /sql with required headers
                    let resp = client
                        .post(&sql_endpoint)
                        .header(ACCEPT, "application/json")
                        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                        .header("Surreal-NS", &args.ns)
                        .header("Surreal-DB", &args.db)
                        .body(stmt)
                        .send();

                    match resp {
                        Ok(r) if r.status().is_success() => {
                            if args.verbosity >= 2 {
                                // In debug, try to read body to surface Surreal response messages
                                let _ = r.text().map(|t| debug!("batch #{idx} ok: {t}"));
                            }
                            info!("batch #{idx} ok ({} records)", batch.len());
                            Ok(())
                        }
                        Ok(r) => {
                            let status = r.status();
                            let text = r.text().unwrap_or_default();
                            Err(anyhow::anyhow!(
                                "batch #{idx} failed: HTTP {}: {}",
                                status,
                                text
                            ))
                        }
                        Err(e) => Err(anyhow::anyhow!("batch #{idx} transport error: {e}")),
                    }
                })
                // Fold success/failure counts immutably
                .fold(
                    || (0usize, 0usize),
                    |(ok, err), res| {
                        if res.is_ok() {
                            (ok + 1, err)
                        } else {
                            (ok, err + 1)
                        }
                    },
                )
                .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
        });

    if err_count > 0 {
        error!("done with errors: ok={}, err={}", ok_count, err_count);
        // Exit non-zero so this can be scripted
        std::process::exit(1);
    } else {
        info!("done: all {} batches ok", ok_count);
    }

    Ok(())
}

/// Build a single SurrealQL INSERT statement that inserts an array of objects.
/// We ask SurrealDB not to echo large results back to us.
fn build_insert_stmt(table: &str, batch: &[Value]) -> anyhow::Result<String> {
    // Serialize the batch to a compact JSON array string
    let json_array = serde_json::to_string(batch)?;
    // INSERT INTO table [ {..}, {..}, ... ] RETURN NONE;
    Ok(format!("INSERT INTO {table} {json_array} RETURN NONE;"))
}
