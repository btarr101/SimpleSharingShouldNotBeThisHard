use std::path::PathBuf;

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use cleanup::cleanup;
use opendal::Operator;
use service::TempShareService;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod cleanup;
mod components;
mod routes;
mod service;
mod util;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_opendal::Opendal(scheme = "s3")] storage: Operator,
) -> Result<TempShareService, shuttle_runtime::Error> {
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
        .with_state(storage.clone());

    let scheduler = JobScheduler::new()
        .await
        .map_err(|err| shuttle_runtime::Error::BuildPanic(err.to_string()))?;

    scheduler
        .add(
            Job::new_async("* 1/30 * * * *", move |_uuid, _l| {
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
