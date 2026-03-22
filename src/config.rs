use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "laws", about = "Local AWS - a lightweight alternative to LocalStack")]
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
}
