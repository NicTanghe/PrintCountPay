use std::future::Future;

use iced::executor::Executor;
use iced::futures;
use tokio::runtime::{Builder, Runtime};

// Discovery and SNMP polling can build fairly deep stacks in debug builds on Windows.
// Keep the multithreaded runtime, but give each worker more headroom.
const TOKIO_WORKER_STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;

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
        std::mem::drop(self.runtime.spawn(future));
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.runtime.enter();
        f()
    }

    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        self.runtime.block_on(future)
    }
}
