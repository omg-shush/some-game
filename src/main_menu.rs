use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::MultiplayerType;

pub struct MainMenuPlugin {}

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum MainMenuState {
    #[default]
    MainMenu
}

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::draw.run_if(state_exists_and_equals(MainMenuState::MainMenu)));
    }
}

impl MainMenuPlugin {
    fn draw(mut contexts: EguiContexts, mut next_state: ResMut<NextState<MultiplayerType>>) {
        egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("pew pew game i guess");
                ui.add_space(32.);
                if ui.button("Singleplayer").clicked() {
                    next_state.set(MultiplayerType::Singleplayer);
                }
                if ui.button("Host Multiplayer").clicked() {
                    next_state.set(MultiplayerType::Server);
                }
                if ui.button("Join Multiplayer").clicked() {
                    next_state.set(MultiplayerType::Client);
                }
            });
        });
    }
}
