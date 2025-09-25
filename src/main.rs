use std::fs;
use std::time::Duration;

use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde_json::{self as json, Value};
use tracing::{debug, error, info, warn};

mod args;
mod log;

fn main() -> anyhow::Result<()> {
    let args = args::parse();

    // Init colorized, timestamped logging
    log::init(log::level_from_verbosity(args.verbosity));

    info!("connecting to SurrealDB at {}:{}", args.host, args.port);
    let sql_endpoint = args.sql_endpoint();

    info!("loading JSON: {}", args.data_path);
    let raw = fs::read_to_string(&args.data_path)?;
    let value: Value = json::from_str(&raw)?;

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

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    let (ok_count, err_count): (usize, usize) = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build()?
        .install(|| {
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
        std::process::exit(1);
    } else {
        info!("done: all {} batches ok", ok_count);
    }

    Ok(())
}

fn build_insert_stmt(table: &str, batch: &[Value]) -> anyhow::Result<String> {
    let json_array = serde_json::to_string(batch)?;
    Ok(format!("INSERT INTO {table} {json_array} RETURN NONE;"))
}
