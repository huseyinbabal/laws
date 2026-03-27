use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "laws",
    about = "Local AWS - a lightweight alternative to LocalStack"
)]
pub struct Config {
    /// Port to listen on
    #[arg(short, long, default_value = "4566")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// AWS region to emulate
    #[arg(long, default_value = "us-east-1")]
    pub region: String,

    /// AWS account ID to use
    #[arg(long, default_value = "000000000000")]
    pub account_id: String,

    /// Enable SQLite persistence so state survives restarts
    #[arg(long, default_value_t = false)]
    pub persist: bool,

    /// Override the default database file path
    /// (default: ~/.config/laws/state.db or platform equivalent)
    #[arg(long)]
    pub db_path: Option<String>,

    /// Wipe the persisted database on startup (only meaningful with --persist)
    #[arg(long, default_value_t = false)]
    pub reset: bool,
}

impl Config {
    /// Resolve the SQLite database path. Uses `--db-path` if given, otherwise
    /// falls back to the platform config directory.
    pub fn resolve_db_path(&self) -> String {
        if let Some(ref p) = self.db_path {
            return p.clone();
        }
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("laws");
        config_dir.join("state.db").to_string_lossy().into_owned()
    }
}
