use crownui::prelude::Cx;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{state::Pad, visibility};

pub fn follow(cx: &mut Cx, pad: Pad, mut requests: UnboundedReceiver<bool>) {
    let next = async move {
        let request = requests.recv().await;
        (requests, request)
    };
    cx.spawn(next, move |cx, (requests, request)| {
        let Some(visible) = request else {
            return;
        };
        visibility::set(cx, pad, visible);
        follow(cx, pad, requests);
    });
}
