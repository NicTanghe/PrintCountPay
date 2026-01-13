use std::future::Future;

use iced::executor::Executor;
use iced::futures;
use tokio::runtime::{Builder, Runtime};

const TOKIO_WORKER_STACK_SIZE_BYTES: usize = 4_200_000;

#[derive(Debug)]
pub struct StackSizedTokioExecutor {
    runtime: Runtime,
}

impl Executor for StackSizedTokioExecutor {
    fn new() -> Result<Self, futures::io::Error> {
        let runtime = Builder::new_multi_thread()
            .thread_stack_size(TOKIO_WORKER_STACK_SIZE_BYTES)
            .enable_all()
            .build()?;
        Ok(Self { runtime })
    }

    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let _ = self.runtime.spawn(future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.runtime.enter();
        f()
    }
}
