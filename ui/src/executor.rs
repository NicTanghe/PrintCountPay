use std::future::Future;

use iced::executor::Executor;
use iced::futures;
use tokio::runtime::{Builder, Runtime};

const TOKIO_WORKER_STACK_SIZE_BYTES: usize = 4_194_304;

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


//   - ui/src/app.rs sets type Executor = crate::executor::StackSizedTokioExecutor;
//     in the Application impl. That tells Iced which executor to construct.
//   - ui/src/lib.rs:14 calls PrintCountApp::run(iced::Settings::with_flags(flags)).
//     When run starts the app, Iced creates the executor by calling Executor::new()
//     and then uses it for all async Command::perform work.
//   - app/src/main.rs:7 calls run(...), which kicks off the chain.

//   So the call site is effectively inside Iced’s runtime initialization. If you
//   want, I can point to the exact spot in the Iced crate where it constructs the
//   executor.


//   Here’s the wiring

//   - ui/src/app.rs:175 sets the associated type Executor =
//     crate::executor::StackSizedTokioExecutor; for your PrintCountApp.
//   - ui/src/lib.rs:14 calls PrintCountApp::run(...), which goes into Iced’s
//     Application::run and forwards the associated executor type into the internal
//     runner (Self::Executor).
//   - The actual creation happens inside Iced’s runner: C:
//     \Users\duplico\.cargo\registry\src\index.crates.io-
//     1949cf8c6b5b557f\iced_winit-0.12.2\src\application.rs:124 constructs the
//     executor via E::new() and builds the runtime. That runtime is held by Iced
//     and used for all Command::perform and subscriptions.

//   So the executor “passes” as a type parameter, not as a value you see in your
//   code.