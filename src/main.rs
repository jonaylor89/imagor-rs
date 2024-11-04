use color_eyre::Result;
use imagor_rs::config::get_configuration;
use imagor_rs::startup::Application;
use imagor_rs::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let parse_dotenv = dotenvy::dotenv();
    if let Err(e) = parse_dotenv {
        tracing::warn!("failed to parse .env file: {}", e);
    }

    let configuration = get_configuration().expect("Failed to read configuration");

    let subscriber = get_subscriber("imagor_rs".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let app = Application::build(configuration).await?;
    let outcome = app.run_until_stopped().await;

    match outcome {
        Ok(()) => {
            tracing::info!("server has exited")
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "server failed",
            )
        }
    }

    Ok(())
}
