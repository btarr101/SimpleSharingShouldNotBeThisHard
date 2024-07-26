use std::net::SocketAddr;

use axum::Router;
use shuttle_runtime::Service;
use tokio_cron_scheduler::JobScheduler;

pub struct TempShareService {
    pub router: Router,
    pub scheduler: JobScheduler,
}

#[shuttle_runtime::async_trait]
impl Service for TempShareService {
    async fn bind(mut self, addr: SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let tcp_listener = shuttle_runtime::tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|err| shuttle_runtime::Error::BindPanic(err.to_string()))?;
        let server = async move { axum::serve(tcp_listener, self.router).await };

        let scheduler = self.scheduler.start();

        let (_scheduler_handle, _server_handle) = tokio::join!(server, scheduler);

        Ok(())
    }
}
