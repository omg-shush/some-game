use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;
use bevy_inspector_egui::quick::StateInspectorPlugin;
use wasm_bindgen_futures::spawn_local;

use super::webrtc::AsyncWebRtcServer;

pub struct WebRtcServerPlugin {
    pub is_headless: bool
}

impl Plugin for WebRtcServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WebRtcServerState>();
        #[cfg(debug_assertions)]
        if !self.is_headless {
            app.add_plugins(StateInspectorPlugin::<WebRtcServerState>::new());
        }
        app.add_systems(PreUpdate, Self::update_server_state);
    }
}

type ServerExists = bool;

impl WebRtcServerPlugin {
    fn update_server_state(
        server: Option<NonSendMut<WebRtcServer>>,
        mut exists: Local<ServerExists>,
        state: Res<State<WebRtcServerState>>,
        mut next_state: ResMut<NextState<WebRtcServerState>>
    ) {
        let removed = server.is_none() && *exists;
        let changed = server.as_ref().map_or(false, |s| s.is_changed());
        if removed || changed || *state == WebRtcServerState::Registering {
            let next = match &server {
                Some(s) if s.is_listening() => WebRtcServerState::Listening,
                Some(s) if s.server.borrow().is_some() => WebRtcServerState::Registered,
                Some(_) => WebRtcServerState::Registering,
                None => WebRtcServerState::Offline,
            };
            if *state.get() != next {
                next_state.set(next);
            }
        }
        *exists = server.is_some();
    }
}

#[derive(States, Debug, Default, Hash, Eq, PartialEq, Clone, Reflect)]
pub enum WebRtcServerState {
    #[default]
    Offline,
    Registering,
    Registered,
    Listening
}

#[derive(Clone)]
pub struct WebRtcServer {
    server: Rc<RefCell<Option<AsyncWebRtcServer>>>,
    listening: bool
}

impl WebRtcServer {
    pub fn new(signaling_url: String, game_name: String, server_name: String) -> WebRtcServer {
        let server = WebRtcServer { server: Rc::new(RefCell::new(None)), listening: false };
        let server_clone = server.clone();
        spawn_local(async move {
            match AsyncWebRtcServer::new(&signaling_url, &game_name, &server_name).await {
                Ok(s) => *server_clone.server.borrow_mut() = Some(s),
                Err(e) => warn!("Error creating AsyncWebRtcServer: {:?}", e),
            }
        });
        server
    }

    pub fn is_listening(&self) -> bool {
        self.listening
    }
}
