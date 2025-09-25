use clap::{ArgAction, Parser};

/// Slurp: chunk JSON items and INSERT them into a SurrealDB table.
///
/// Reads a JSON array from --data payload.json, splits it into --batch sized
/// chunks, and performs parallel INSERTs into SurrealDB using SurrealQL.
///
/// Connection is built from --host and --port with no auth.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// SurrealDB host (no scheme)
    #[arg(long = "host", default_value = "localhost")]
    pub host: String,

    /// SurrealDB port (must be > 0)
    #[arg(long = "port", default_value_t = 8000)]
    pub port: u16,

    /// SurrealDB namespace
    #[arg(long = "ns")]
    pub ns: String,

    /// SurrealDB database
    #[arg(long = "db")]
    pub db: String,

    /// Destination table name
    #[arg(long = "table")]
    pub table: String,

    /// Path to a JSON array file (e.g., [ {..}, {..}, ... ])
    #[arg(long = "data")]
    pub data_path: String,

    /// Batch size (number of items per INSERT, must be > 0)
    #[arg(long = "batch", default_value_t = 500)]
    pub batch: usize,

    /// Number of parallel worker threads (must be > 0)
    #[arg(long = "thread", default_value_t = 4)]
    pub threads: usize,

    /// Verbosity level: 0=warn, 1=info, 2=debug
    #[arg(long = "verbosity", default_value_t = 1)]
    pub verbosity: u8,

    /// Dry-run: parse and show what would be inserted, but do not send requests
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,
}

impl Args {
    /// Build the SurrealDB /sql endpoint URL from host and port.
    pub fn sql_endpoint(&self) -> String {
        format!("http://{}:{}/sql", self.host, self.port)
    }

    /// Validate numeric constraints that clap doesn't enforce here.
    fn validate(&self) -> anyhow::Result<()> {
        if self.port == 0 {
            anyhow::bail!("--port must be > 0");
        }
        if self.batch == 0 {
            anyhow::bail!("--batch must be > 0");
        }
        if self.threads == 0 {
            anyhow::bail!("--thread must be > 0");
        }
        if self.verbosity > 2 {
            anyhow::bail!("--verbosity must be in 0..=2");
        }
        Ok(())
    }
}

/// Parse CLI args in one place so main.rs does not need clap in scope.
pub fn parse() -> Args {
    let args = Args::parse();
    // Fail-fast on invalid values
    if let Err(e) = args.validate() {
        // Print a friendly error and exit with non-zero, matching clap behavior
        eprintln!("error: {}", e);
        std::process::exit(2);
    }
    args
}
