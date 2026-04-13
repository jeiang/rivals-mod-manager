use std::path::PathBuf;
use std::time::Duration;
use std::{f32, io};

use egui::{Grid, Id, Layout, Modal, ScrollArea, TextEdit};
use egui_async::Bind;
use egui_material_icons::icons::ICON_DELETE;

use crate::categories::{CategoryMatcher, CategoryMatchers, default_matchers};
use crate::mods::{ModInfo, refresh_mod_list};
use crate::settings::Settings;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    current_page: Page,
    settings: Settings,
    first_time_setup: bool,
    #[serde(skip)]
    inputs: InputControls,
    categories: CategoryMatchers,
    mods: Vec<ModInfo>,
}

#[derive(Default, PartialEq)]
pub enum CategoryFilter {
    #[default]
    None,
    Uncategorized,
    Category(String),
}

#[derive(Default)]
pub struct InputControls {
    settings_game_folder: Bind<PathBuf, ()>,
    settings_input_folder: Bind<PathBuf, ()>,
    categories_new_category: String,
    categories_modal_is_open: bool,
    categories_modal_idx: usize,
    categories_modal_matchers: Vec<String>,
    mods_category_filter: CategoryFilter,
    mods_name_filter: String,
    mods_refresh_list: Bind<Vec<ModInfo>, io::Error>,
    misc_needs_save: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Clone)]
pub enum Page {
    Main,
    #[default]
    Settings,
    Categories,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_page: Default::default(),
            settings: Default::default(),
            first_time_setup: true,
            inputs: Default::default(),
            categories: default_matchers(),
            mods: vec![],
        }
    }
}

const fn game_path() -> &'static str {
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
        egui_material_icons::initialize(ctx);
        ctx.plugin_or_default::<egui_async::EguiAsyncPlugin>();
        if let Some(storage) = cc.storage {
            log::debug!("Storage loading...");
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            // TODO: do not override anymore
            let prev = Self::default();
            let mut settings = Settings::default();
            settings.set_game_folder(PathBuf::from(
                "/home/aidanp/.local/share/Steam/steamapps/common/MarvelRivals",
            ));
            settings
                .set_input_folder(PathBuf::from("/home/aidanp/Games/Mods/Marvel Rivals/downloads"));
            Self { settings, ..prev }
        }
    }

    fn main_page(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("categories_panel")
            .show_separator_line(true)
            .min_size(200.0)
            .show_inside(ui, |ui| {
                ui.heading("Categories");
                ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                    ScrollArea::vertical().max_width(f32::INFINITY).show(ui, |ui| {
                        ui.selectable_value(
                            &mut self.inputs.mods_category_filter,
                            CategoryFilter::None,
                            "All",
                        );
                        for i in self.categories.iter() {
                            ui.selectable_value(
                                &mut self.inputs.mods_category_filter,
                                CategoryFilter::Category(i.name().to_string()),
                                i.name(),
                            );
                        }
                        ui.selectable_value(
                            &mut self.inputs.mods_category_filter,
                            CategoryFilter::Uncategorized,
                            "Uncategorized",
                        );
                    });
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Mods");
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.inputs.mods_name_filter)
                        .hint_text("Filter mods..."),
                );
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Apply Mods").clicked() {
                        // TODO:
                    }
                    if ui.button("Clear Mods").clicked() {
                        // TODO:
                    }
                    if ui.button("Refresh Mods").clicked() {
                        let name = self.settings.input_folder().clone();
                        let matchers = self.categories.clone();
                        self.inputs
                            .mods_refresh_list
                            .request((|| async move { refresh_mod_list(&name, &matchers).await })());
                    }
                });
            });
            if let Some(Ok(modlist)) = self.inputs.mods_refresh_list.read() {
                self.mods = dbg!(modlist.to_vec());
                self.inputs.mods_refresh_list.clear();
            }
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let game_folder = self.settings.game_folder().to_path_buf();
        let input_folder = self.settings.input_folder().to_path_buf();
        let mut api_key = self.settings.nexusmods_api_key().to_string();
        let mut game_folder_display = self.settings.game_folder().display().to_string();
        let mut input_folder_display = self.settings.input_folder().display().to_string();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                ui.set_max_width(800.0);
                ui.horizontal(|ui| {
                    ui.heading("Settings");
                    ui.take_available_width();
                });
                Grid::new("settings").num_columns(2).max_col_width(600.0).show(ui, |ui| {
                    ui.label("NexusMods API Key:").on_hover_ui(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(
                                "API Key used for fetching mod details from NexusMods. Go to ",
                            );
                            ui.hyperlink_to(
                                "NexusMods",
                                "https://www.nexusmods.com/settings/api-keys",
                            );
                            ui.label(" to get an API Key.");
                        });
                    });
                    ui.add(
                        TextEdit::singleline(&mut api_key)
                            .password(true)
                            .desired_width(ui.available_width()),
                    );
                    ui.end_row();
                    ui.label("Game Folder:").on_hover_ui(|ui| {
                        ui.label(format!(
                            "The location of your Marvel Rivals Game. This should be something \
                             like {}.",
                            game_path()
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Browse").clicked() {
                                self.inputs.settings_game_folder.request((|| async {
                                    let dialog = rfd::AsyncFileDialog::new().pick_folder().await;
                                    Ok(dialog
                                        .map(|f| f.path().to_path_buf())
                                        .unwrap_or(game_folder))
                                })(
                                ));
                            }
                            ui.add_enabled_ui(false, |ui| {
                                ui.add(
                                    TextEdit::singleline(&mut game_folder_display)
                                        .desired_width(ui.available_width()),
                                );
                            });
                        });
                    });
                    ui.end_row();
                    ui.label("Input Folder:").on_hover_ui(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(
                                "The folder where you store your mods. This should contain \
                                 folders, such as ",
                            );
                            ui.hyperlink_to(
                                "PNG Spider Man - 5779",
                                "https://www.nexusmods.com/marvelrivals/mods/5779",
                            );
                            ui.label(
                                ", which contain the mod files (.pak files, sometimes with .ucas \
                                 and .utoc files).",
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Browse").clicked() {
                                self.inputs.settings_input_folder.request((|| async {
                                    let dialog = rfd::AsyncFileDialog::new().pick_folder().await;
                                    Ok(dialog
                                        .map(|f| f.path().to_path_buf())
                                        .unwrap_or(input_folder))
                                })(
                                ));
                            }
                            ui.add_enabled_ui(false, |ui| {
                                ui.add(
                                    TextEdit::singleline(&mut input_folder_display)
                                        .desired_width(ui.available_width()),
                                );
                            });
                        });
                    });
                    ui.end_row();
                });
                if self.first_time_setup {
                    ui.small("Set the game folder and input folder before setting up your mods.");
                }
                self.settings.set_nexusmods_api_key(api_key);
                if let Some(Ok(x)) = self.inputs.settings_game_folder.read() {
                    self.settings.set_game_folder(x.clone());
                    self.inputs.settings_game_folder.clear();
                }
                if let Some(Ok(x)) = self.inputs.settings_input_folder.read() {
                    self.settings.set_input_folder(x.clone());
                    self.inputs.settings_input_folder.clear();
                }
                if self.settings.game_folder().exists() && self.settings.input_folder().exists() {
                    self.first_time_setup = false;
                }
            });
        });
    }

    fn categories_page(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                ui.set_max_width(800.0);
                ui.horizontal(|ui| {
                    ui.heading("Category Management");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Use Default Matchers").clicked() {
                            self.categories = default_matchers();
                        }
                        if ui.button("Sort").clicked() {
                            self.categories.sort_unstable_by(|l, r| l.name().cmp(r.name()));
                        }
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Add Category: ");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button = ui.button("Add");
                        let text = ui.add(
                            TextEdit::singleline(&mut self.inputs.categories_new_category)
                                .desired_width(ui.available_width()),
                        );
                        if button.clicked()
                            || (text.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            let item = self.categories.iter().position(|e| {
                                e.name().eq_ignore_ascii_case(
                                    &self.inputs.categories_new_category.clone(),
                                )
                            });
                            if item.is_none() {
                                self.categories.push(CategoryMatcher::new(
                                    self.inputs.categories_new_category.clone(),
                                    vec![],
                                ));
                            }

                            self.inputs.categories_new_category.clear();
                        }
                    });
                });
                ui.separator();
                let mut is_second_col = false;
                let mut should_remove = None;
                ScrollArea::vertical().show(ui, |ui| {
                    Grid::new("settings")
                        .striped(true)
                        .spacing([0., ui.spacing().item_spacing.y])
                        .num_columns(4)
                        .max_col_width(200.0)
                        .min_col_width(200.0)
                        .show(ui, |ui| {
                            for (idx, category) in self.categories.iter().enumerate().clone() {
                                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(category.name());
                                        ui.small(format!(
                                            "{} matcher(s)",
                                            category.matchers().len()
                                        ));
                                    });
                                });
                                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.add_space(ui.spacing().item_spacing.x);
                                    if ui.button(ICON_DELETE.outlined()).clicked() {
                                        should_remove = Some(idx);
                                        ui.ctx().request_repaint();
                                    }
                                    if ui.button("Edit Matchers").clicked() {
                                        self.inputs.categories_modal_is_open = true;
                                        self.inputs.categories_modal_idx = idx;
                                        self.inputs.categories_modal_matchers = category
                                            .matchers()
                                            .into_iter()
                                            .map(|m| m.to_string())
                                            .collect();
                                    }
                                    ui.take_available_width();
                                });
                                if is_second_col {
                                    ui.end_row();
                                }
                                is_second_col = !is_second_col;
                            }
                        });
                });

                if let Some(idx) = should_remove {
                    self.categories.remove(idx);
                }

                if self.inputs.categories_modal_is_open {
                    let modal = Modal::new(Id::new("Category Edit Modal")).show(ui.ctx(), |ui| {
                        ui.set_width(500.0);
                        ui.heading(format!(
                            "Edit \"{}\" Matchers",
                            self.categories[self.inputs.categories_modal_idx].name()
                        ));
                        let mut to_remove = None;
                        Grid::new("Matcher Modal").num_columns(1).show(ui, |ui| {
                            for (idx, matcher) in
                                self.inputs.categories_modal_matchers.iter_mut().enumerate()
                            {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button(ICON_DELETE).clicked() {
                                            to_remove = Some(idx);
                                        }
                                        ui.add(
                                            TextEdit::singleline(matcher)
                                                .desired_width(ui.available_width()),
                                        );
                                    },
                                );
                                ui.end_row();
                            }
                        });
                        if let Some(idx) = to_remove {
                            self.inputs.categories_modal_matchers.remove(idx);
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Add Matcher").clicked() {
                                self.inputs.categories_modal_matchers.push(String::new());
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Save").clicked() {
                                        let new_matchers = self
                                            .inputs
                                            .categories_modal_matchers
                                            .iter()
                                            .map_while(|m| lazy_regex::Regex::new(m).ok())
                                            .collect::<Vec<_>>();
                                        if self.inputs.categories_modal_matchers.len()
                                            > new_matchers.len()
                                        {
                                            // TODO: if len mismatch, one of the
                                            // inputs failed regex parsing
                                        } else {
                                            self.categories[self.inputs.categories_modal_idx]
                                                .set_matchers(new_matchers);
                                            ui.close();
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        ui.close();
                                    }
                                    ui.take_available_width();
                                },
                            );
                        });
                    });
                    if modal.should_close() {
                        self.inputs.categories_modal_is_open = false;
                    }
                }
            });
        });
    }
}

impl eframe::App for App {
    fn auto_save_interval(&self) -> std::time::Duration {
        if self.inputs.misc_needs_save {
            std::time::Duration::from_secs(0)
        } else {
            std::time::Duration::from_secs(30)
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
        self.inputs.misc_needs_save = false;
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after_for(Duration::from_secs(1), ctx.viewport_id());
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

        if self.first_time_setup {
            self.settings_page(ui, frame);
            self.current_page = Page::Settings;
        } else {
            match self.current_page {
                Page::Main => {
                    self.main_page(ui, frame);
                }
                Page::Settings => {
                    self.settings_page(ui, frame);
                }
                Page::Categories => {
                    self.categories_page(ui, frame);
                }
            }
        }
    }
}
