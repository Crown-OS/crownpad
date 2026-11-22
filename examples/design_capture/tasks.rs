//! `cx.spawn` futures run on a tokio thread, as the app's runtime runs them; their outputs come
//! back through [`Pump`].

use std::{
    any::Any,
    sync::{
        OnceLock,
        mpsc::{Receiver, Sender, channel},
    },
};

use anyhow::{Context, Result};
use tokio::runtime::Handle;
use view::{TaskExecutor, TaskFuture, TaskId};

pub type Finished = (TaskId, Box<dyn Any + Send>);

struct ThreadedTasks {
    runtime: Handle,
    finished: Sender<Finished>,
}

impl TaskExecutor for ThreadedTasks {
    fn spawn(&mut self, id: TaskId, future: TaskFuture) {
        let finished = self.finished.clone();
        self.runtime.spawn(async move {
            let _ = finished.send((id, future.await));
        });
    }

    fn cancel(&mut self, _id: TaskId) {}
}

pub struct Pump {
    finished: Receiver<Finished>,
}

impl Pump {
    pub fn drain(&self) -> Vec<Finished> {
        self.finished.try_iter().collect()
    }
}

pub fn threaded() -> Result<(Box<dyn TaskExecutor>, Pump)> {
    let (finished, receiver) = channel();
    let executor = ThreadedTasks {
        runtime: task_runtime()?.clone(),
        finished,
    };
    Ok((Box::new(executor), Pump { finished: receiver }))
}

fn task_runtime() -> Result<&'static Handle> {
    static RUNTIME: OnceLock<Handle> = OnceLock::new();
    if let Some(handle) = RUNTIME.get() {
        return Ok(handle);
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("a task runtime")?;
    let handle = runtime.handle().clone();
    std::thread::Builder::new()
        .name("design-capture-tasks".into())
        .spawn(move || runtime.block_on(std::future::pending::<()>()))
        .context("the task thread")?;
    Ok(RUNTIME.get_or_init(|| handle))
}
