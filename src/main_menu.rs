use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::{Multiplayer, PlayerInfo};

pub struct MainMenuPlugin {}

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum MainMenu {
    #[default]
    MainMenu,
    InGame,
}

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::draw.run_if(state_exists_and_equals(MainMenu::MainMenu)));
    }
}

impl MainMenuPlugin {
    fn draw(
        mut contexts: EguiContexts,
        mut multiplayer_state: ResMut<NextState<Multiplayer>>,
        mut menu_state: ResMut<NextState<MainMenu>>,
        mut player_info: ResMut<PlayerInfo>,
        mut username: Local<String>,
    ) {
        egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(ui.available_width() * 0.4);
                ui.heading("pew pew game i guess");
                ui.add_space(32.);
                ui.columns(2, |columns| {
                    columns[0].label("Username");
                    columns[1].text_edit_singleline(&mut *username);
                });
                if ui.button("Singleplayer").clicked() {
                    multiplayer_state.set(Multiplayer::Singleplayer);
                    menu_state.set(MainMenu::InGame);
                    player_info.username = username.to_owned();
                }
                if ui.button("Host Multiplayer").clicked() {
                    multiplayer_state.set(Multiplayer::Server);
                    menu_state.set(MainMenu::InGame);
                    player_info.username = username.to_owned();
                }
                if ui.button("Join Multiplayer").clicked() {
                    multiplayer_state.set(Multiplayer::Client);
                    menu_state.set(MainMenu::InGame);
                    player_info.username = username.to_owned();
                }
            });
        });
    }
}
