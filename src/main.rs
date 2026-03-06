mod compute;
mod config;
mod db;
mod fetchers;
mod models;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bydleni_rs", about = "Czech housing affordability dashboard")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch data from external sources
    Fetch {
        /// Fetch from all sources
        #[arg(long)]
        all: bool,
        /// Fetch from a specific source (fred, cnb, czso, sreality)
        #[arg(long)]
        source: Option<String>,
        /// Force re-fetch even if data is fresh
        #[arg(long)]
        force: bool,
    },
    /// Compute affordability metrics from fetched data
    Compute {
        /// Also compute historical snapshots (2005, 2010, 2015, 2020)
        #[arg(long)]
        historical: bool,
    },
    /// Start the web server
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(e) = dotenvy::dotenv_override() {
        tracing::debug!("No .env file loaded: {e}");
    }
    let cfg = config::Config::from_env()?;
    tracing::debug!("DATABASE_URL = {}", cfg.database_url);
    let pool = db::init_pool(&cfg.database_url).await?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch { all, source, force } => {
            if all || source.is_none() {
                fetchers::fetch_all(&pool, &cfg, force).await?;
            } else if let Some(src) = source {
                match src.as_str() {
                    "fred" => fetchers::fred::fetch_and_store(&pool, &cfg, force).await?,
                    "cnb" => fetchers::cnb::fetch_and_store(&pool, &cfg, force).await?,
                    "czso" => fetchers::czso::fetch_and_store(&pool, &cfg, force).await?,
                    "sreality" => fetchers::sreality::fetch_and_store(&pool, &cfg, force).await?,
                    other => {
                        anyhow::bail!("Unknown source: {other}. Use: fred, cnb, czso, sreality")
                    }
                }
            }
        }
        Commands::Compute { historical } => {
            compute::affordability::compute_all(&pool).await?;
            if historical {
                compute::historical::compute_historical_snapshots(&pool).await?;
            }
        }
        Commands::Serve => {
            server::serve(pool, cfg).await?;
        }
    }

    Ok(())
}
