use std::path::PathBuf;
use std::time::Duration;

use egui::TextEdit;
use egui_async::Bind;

use crate::settings::Settings;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct App {
    current_page: Page,
    settings: Settings,
    #[serde(skip)]
    binds: AsyncBinds,
}

#[derive(Default)]
pub struct AsyncBinds {
    settings_game_folder: Bind<PathBuf, ()>,
    settings_input_folder: Bind<PathBuf, ()>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Clone)]
pub enum Page {
    #[default]
    Main,
    Settings,
    Categories,
}

fn game_path() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\MarvelRivals"
    }
    #[cfg(target_os = "linux")]
    {
        "$HOME/.local/share/Steam/steamapps/common/MarvelRivals"
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_zoom_factor(2.0);
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    fn main(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label(":)");
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let game_folder = self.settings.game_folder().to_path_buf();
        let input_folder = self.settings.input_folder().to_path_buf();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Settings");
            ui.horizontal(|ui| {
                ui.label("NexusMods API Key:").on_hover_ui(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("API Key used for fetching mod details from NexusMods. Go to ");
                        ui.hyperlink_to("NexusMods", "https://www.nexusmods.com/settings/api-keys");
                        ui.label(" to get an API Key.");
                    });
                });
                let mut api_key = self.settings.nexusmods_api_key().to_string();
                ui.add(
                    TextEdit::singleline(&mut api_key)
                        .password(true)
                        .desired_width(ui.available_width().min(600.0)),
                );
                self.settings.set_nexusmods_api_key(api_key);
            });
            ui.horizontal(|ui| {
                ui.label("Game Folder:").on_hover_ui(|ui| {
                    ui.label(format!(
                        "The location of your Marvel Rivals Game. This should be something like \
                         {}.",
                        game_path()
                    ));
                });
                let mut path = self.settings.game_folder().display().to_string();
                ui.add_enabled_ui(false, |ui| {
                    ui.spacing_mut().text_edit_width = ui.available_width().min(600.0);
                    ui.text_edit_singleline(&mut path);
                });
                if ui.button("Browse").clicked() {
                    self.binds.settings_game_folder.request((|| async {
                        let dialog = rfd::AsyncFileDialog::new().pick_folder().await;
                        Ok(dialog.map(|f| f.path().to_path_buf()).unwrap_or(game_folder))
                    })());
                }
                if let Some(Ok(x)) = self.binds.settings_game_folder.read() {
                    self.settings.set_game_folder(x.clone());
                }
            });
            ui.horizontal(|ui| {
                let mut path = self.settings.input_folder().display().to_string();
                ui.label("Input Folder:").on_hover_ui(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(
                            "The folder where you store your mods. This should contain folders, \
                             such as ",
                        );
                        ui.hyperlink_to(
                            "PNG Spider Man - 5779",
                            "https://www.nexusmods.com/marvelrivals/mods/5779",
                        );
                        ui.label(
                            ", which contain the mod files (.pak files, sometimes with .ucas and \
                             .utoc files).",
                        );
                    });
                });
                ui.add_enabled_ui(false, |ui| {
                    ui.spacing_mut().text_edit_width = ui.available_width().min(600.0);
                    ui.text_edit_singleline(&mut path);
                });
                if ui.button("Browse").clicked() {
                    self.binds.settings_input_folder.request((|| async {
                        let dialog = rfd::AsyncFileDialog::new().pick_folder().await;
                        Ok(dialog.map(|f| f.path().to_path_buf()).unwrap_or(input_folder))
                    })());
                }
                if let Some(Ok(x)) = self.binds.settings_input_folder.read() {
                    self.settings.set_input_folder(x.clone());
                }
            });
        });
    }

    fn categories(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label(":(");
        });
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.plugin_or_default::<egui_async::EguiAsyncPlugin>();
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.separator();
                ui.selectable_value(&mut self.current_page, Page::Main, "Mods");
                ui.selectable_value(&mut self.current_page, Page::Settings, "Settings");
                ui.selectable_value(&mut self.current_page, Page::Categories, "Categories");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                    ui.take_available_space();
                });
            });
        });

        match self.current_page {
            Page::Main => {
                self.main(ui, frame);
            }
            Page::Settings => {
                self.settings(ui, frame);
            }
            Page::Categories => {
                self.categories(ui, frame);
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after_for(Duration::from_secs(1), ctx.viewport_id());
    }
}
