use std::{path::PathBuf, str::FromStr};

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use chrono::Utc;
use cleanup::cleanup;
use opendal::Operator;
use routes::not_found::not_found;
use service::TempShareService;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod cleanup;
mod components;
mod routes;
mod service;
mod util;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_opendal::Opendal(scheme = env!("OPENDAL_SCHEME"))] storage: Operator,
) -> Result<TempShareService, shuttle_runtime::Error> {
    let format = tracing_subscriber::fmt::format().without_time().compact();
    tracing_subscriber::fmt()
        .event_format(format)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    tracing::info!("Tracing is initialized!");

    let router = Router::new()
        .route(
            "/",
            get(routes::index::get_index).post(routes::index::post_index),
        )
        .layer(DefaultBodyLimit::disable())
        .route("/shared/:file_name", get(routes::shared::get_shared))
        .route("/stream/:file_name", get(routes::stream::get_stream))
        .nest_service(
            "/public",
            ServeDir::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("public")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(storage.clone())
        .fallback(not_found);

    let scheduler = JobScheduler::new()
        .await
        .map_err(|err| shuttle_runtime::Error::BuildPanic(err.to_string()))?;

    let cron_schedule = if cfg!(debug_assertions) {
        "0/30 * * * * *"
    } else {
        "0 0/30 * * * *"
    };

    let schedule = cron::Schedule::from_str(cron_schedule).unwrap();
    tracing::info!("Upcoming fire times:");
    schedule
        .upcoming(Utc)
        .take(10)
        .for_each(|time| tracing::info!("-> {}", time));

    scheduler
        .add(
            Job::new_async(cron_schedule, move |_uuid, _l| {
                let storage = storage.clone(); // Clone storage just for this task
                Box::pin(async move {
                    if let Err(err) = cleanup(storage).await {
                        tracing::error!("{err}");
                    }
                })
            })
            .map_err(|err| shuttle_runtime::Error::BuildPanic(err.to_string()))?,
        )
        .await
        .map_err(|err| shuttle_runtime::Error::BuildPanic(err.to_string()))?;

    Ok(TempShareService { router, scheduler })
}
