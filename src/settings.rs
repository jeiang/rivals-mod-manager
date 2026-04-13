use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
pub struct Settings {
    nexusmods_api_key: String,
    game_folder: PathBuf,
    input_folder: PathBuf,
}

impl Settings {
    pub fn nexusmods_api_key(&self) -> &str {
        &self.nexusmods_api_key
    }

    pub fn set_nexusmods_api_key(&mut self, nexusmods_api_key: String) {
        self.nexusmods_api_key = nexusmods_api_key;
    }

    pub fn game_folder(&self) -> &PathBuf {
        &self.game_folder
    }

    pub fn set_game_folder(&mut self, game_folder: PathBuf) {
        self.game_folder = game_folder;
    }

    pub fn input_folder(&self) -> &PathBuf {
        &self.input_folder
    }

    pub fn set_input_folder(&mut self, input_folder: PathBuf) {
        self.input_folder = input_folder;
    }
}
