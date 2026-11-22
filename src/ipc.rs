use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use crownos_ipc::{RemoteError, ServiceBuilder, blocking};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::protocol::protocol;

#[derive(Clone, Default)]
pub struct VisibilityReport(Arc<AtomicBool>);

impl VisibilityReport {
    pub fn set(&self, visible: bool) {
        self.0.store(visible, Ordering::Relaxed);
    }

    fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub struct Remote {
    pub requests: UnboundedReceiver<bool>,
    pub report: VisibilityReport,
}

pub fn serve() -> Result<Remote> {
    let mut server = ServiceBuilder::new(protocol::SERVICE)
        .build()
        .context("bind the crownpad socket; is another crownpad running?")?;
    let (sender, requests) = unbounded_channel();
    let report = VisibilityReport::default();
    let mut handler = Handler {
        report: report.clone(),
        sender,
    };
    std::thread::Builder::new()
        .name("crownpad-ipc".into())
        .spawn(move || {
            let served = blocking::serve(&mut server, |server, peer, message| {
                protocol::dispatch(&mut handler, server, peer, message).map(|()| true)
            });
            if let Err(error) = served {
                log::error!("crownpad IPC stopped: {error}");
            }
        })
        .context("start the IPC thread")?;
    Ok(Remote { requests, report })
}

struct Handler {
    report: VisibilityReport,
    sender: UnboundedSender<bool>,
}

impl Handler {
    fn request(&mut self, visible: bool) -> Result<bool, RemoteError> {
        self.sender
            .send(visible)
            .map_err(|_| RemoteError::handler("crownpad is shutting down"))?;
        self.report.set(visible);
        Ok(visible)
    }
}

impl protocol::Handler for Handler {
    fn show(&mut self) -> Result<bool, RemoteError> {
        self.request(true)
    }

    fn hide(&mut self) -> Result<bool, RemoteError> {
        self.request(false)
    }

    fn toggle(&mut self) -> Result<bool, RemoteError> {
        let visible = !self.report.get();
        self.request(visible)
    }

    fn is_visible(&mut self) -> Result<bool, RemoteError> {
        Ok(self.report.get())
    }
}
