use async_channel::{Receiver, Sender};
use bevy::prelude::*;
use bevy_cef_core::prelude::WebviewPopupEvent;

#[derive(Resource, Debug, Deref)]
pub struct WebviewPopupSender(pub Sender<WebviewPopupEvent>);

#[derive(Resource, Debug)]
pub struct WebviewPopupReceiver(pub Receiver<WebviewPopupEvent>);

pub(super) struct WebviewPopupPlugin;

impl Plugin for WebviewPopupPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = async_channel::unbounded();
        app.insert_resource(WebviewPopupSender(tx))
            .insert_resource(WebviewPopupReceiver(rx));
    }
}
