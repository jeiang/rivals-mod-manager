use std::collections::{BTreeSet, HashMap};
use std::f32;
use std::path::PathBuf;
use std::time::Duration;

use egui::{
    Align2,
    Color32,
    Direction,
    Frame,
    Grid,
    Id,
    Layout,
    Margin,
    Modal,
    Rangef,
    ScrollArea,
    TextEdit,
    TextStyle,
    TextWrapMode,
    Ui,
    WidgetText,
};
use egui_async::Bind;
use egui_autocomplete::AutoCompleteTextEdit;
use egui_material_icons::icons::{
    ICON_DELETE,
    ICON_KEYBOARD_ARROW_DOWN,
    ICON_KEYBOARD_ARROW_RIGHT,
    ICON_KEYBOARD_ARROW_UP,
};
use egui_table::{AutoSizeMode, CellInfo, Column, HeaderCellInfo, HeaderRow, Table, TableDelegate};
use egui_toast::{Toast, ToastOptions, Toasts};
use regex::Regex;

use crate::categories::{CategoryMatcher, CategoryMatchers, default_matchers};
use crate::mods::{ModInfo, ModList, ModsRefreshResult, refresh_mods_with_nexusmods};
use crate::nexusmods::CachedModInfo;
use crate::settings::Settings;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    current_page: Page,
    settings: Settings,
    first_time_setup: bool,
    #[serde(skip)]
    state: State,
    categories: CategoryMatchers,
    mods: ModList,
    nexusmods_mod_cache: HashMap<u32, CachedModInfo>,
}

#[derive(Default, PartialEq)]
pub enum CategoryFilter {
    #[default]
    None,
    Uncategorized,
    Category(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSortColumn {
    Name,
    Author,
    ModId,
    Category,
    LastModified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

pub struct State {
    toasts: Toasts,
    settings_game_folder: Bind<PathBuf, ()>,
    settings_input_folder: Bind<PathBuf, ()>,
    categories_new_category: String,
    categories_modal_is_open: bool,
    categories_modal_idx: usize,
    categories_modal_matchers: Vec<String>,
    mods_category_filter: CategoryFilter,
    mods_name_filter: String,
    mods_refresh_list: Bind<ModsRefreshResult, String>,
    mods_sort_column: Option<ModSortColumn>,
    mods_sort_direction: SortDirection,
    mods_selected_indices: BTreeSet<usize>,
    mods_selection_anchor: Option<usize>,
    mods_expanded_indices: BTreeSet<usize>,
    mods_edit_modal: Option<ModEditModal>,
    misc_needs_save: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            toasts: Default::default(),
            settings_game_folder: Default::default(),
            settings_input_folder: Default::default(),
            categories_new_category: Default::default(),
            categories_modal_is_open: Default::default(),
            categories_modal_idx: Default::default(),
            categories_modal_matchers: Default::default(),
            mods_category_filter: Default::default(),
            mods_name_filter: Default::default(),
            mods_refresh_list: Default::default(),
            mods_sort_column: Some(ModSortColumn::Name),
            mods_sort_direction: SortDirection::Ascending,
            mods_selected_indices: Default::default(),
            mods_selection_anchor: Default::default(),
            mods_expanded_indices: Default::default(),
            mods_edit_modal: Default::default(),
            misc_needs_save: Default::default(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Clone)]
pub enum Page {
    Main,
    #[default]
    Settings,
    Categories,
}

#[derive(Clone)]
enum ModEditModal {
    Rename { mod_index: usize, name: String },
    ChangeAuthor { mod_indices: Vec<usize>, author: String },
    SetModId { mod_index: usize, mod_id: String },
    ChooseCategory { mod_indices: Vec<usize>, category: String },
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_page: Default::default(),
            settings: Default::default(),
            first_time_setup: true,
            state: Default::default(),
            categories: default_matchers(),
            mods: Default::default(),
            nexusmods_mod_cache: Default::default(),
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

const MODS_TABLE_ID: &str = "mods_table";
const MODS_TABLE_HEADER_HEIGHT: f32 = 24.0;
const MODS_TABLE_ROW_MIN_HEIGHT: f32 = 40.0;
const MODS_TABLE_FILE_ROW_HEIGHT: f32 = 28.0;
const MODS_TABLE_CELL_MARGIN_X: i8 = 8;
const MODS_TABLE_CELL_MARGIN_Y: i8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModTableColumn {
    Enabled,
    Name,
    Author,
    ModId,
    Category,
    LastModified,
}

impl ModTableColumn {
    const ALL: [Self; 6] =
        [Self::Enabled, Self::Name, Self::Author, Self::ModId, Self::Category, Self::LastModified];

    fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    fn index(self) -> usize {
        match self {
            Self::Enabled => 0,
            Self::Name => 1,
            Self::Author => 2,
            Self::ModId => 3,
            Self::Category => 4,
            Self::LastModified => 5,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Name => "Name",
            Self::Author => "Author",
            Self::ModId => "Mod ID",
            Self::Category => "Category",
            Self::LastModified => "Last Modified",
        }
    }

    fn sort_column(self) -> Option<ModSortColumn> {
        match self {
            Self::Enabled => None,
            Self::Name => Some(ModSortColumn::Name),
            Self::Author => Some(ModSortColumn::Author),
            Self::ModId => Some(ModSortColumn::ModId),
            Self::Category => Some(ModSortColumn::Category),
            Self::LastModified => Some(ModSortColumn::LastModified),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ModTableRow {
    mod_index: usize,
    kind: ModTableRowKind,
}

impl ModTableRow {
    fn summary(mod_index: usize) -> Self {
        Self { mod_index, kind: ModTableRowKind::Summary }
    }

    fn file(mod_index: usize, file_index: usize) -> Self {
        Self { mod_index, kind: ModTableRowKind::File { file_index } }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModTableRowKind {
    Summary,
    File { file_index: usize },
}

#[derive(Debug, Clone)]
struct ModContextMenuData {
    clicked_mod_index: usize,
    mod_indices: Vec<usize>,
    name: String,
    author: String,
    mod_id: String,
    category: String,
}

struct ModsTableDelegate<'a> {
    mods: &'a mut [ModInfo],
    rows: Vec<ModTableRow>,
    row_tops: Vec<f32>,
    selection_handled_mod_indices: BTreeSet<usize>,
    context_menu_handled_mod_indices: BTreeSet<usize>,
    categories: &'a CategoryMatchers,
    edit_modal: &'a mut Option<ModEditModal>,
    needs_save: &'a mut bool,
    selected_mod_indices: &'a mut BTreeSet<usize>,
    selection_anchor: &'a mut Option<usize>,
    expanded_mod_indices: &'a mut BTreeSet<usize>,
    sort_column: &'a mut Option<ModSortColumn>,
    sort_direction: &'a mut SortDirection,
}

impl ModsTableDelegate<'_> {
    fn row(&self, row_nr: u64) -> Option<ModTableRow> {
        usize::try_from(row_nr).ok().and_then(|index| self.rows.get(index)).copied()
    }

    fn mod_info_mut(&mut self, row_nr: u64) -> Option<&mut ModInfo> {
        let row = self.row(row_nr)?;
        self.mods.get_mut(row.mod_index)
    }

    fn visible_mod_range(
        &self,
        start_mod_index: usize,
        end_mod_index: usize,
    ) -> Option<Vec<usize>> {
        let start = self.rows.iter().position(|row| {
            row.mod_index == start_mod_index && row.kind == ModTableRowKind::Summary
        })?;
        let end = self.rows.iter().position(|row| {
            row.mod_index == end_mod_index && row.kind == ModTableRowKind::Summary
        })?;
        let (from, to) = if start <= end { (start, end) } else { (end, start) };

        Some(
            self.rows[from..=to]
                .iter()
                .filter(|row| row.kind == ModTableRowKind::Summary)
                .map(|row| row.mod_index)
                .collect(),
        )
    }

    fn mod_context_menu_data(&self, mod_index: usize) -> Option<ModContextMenuData> {
        let mod_info = self.mods.get(mod_index)?;
        let mod_indices = if self.selected_mod_indices.contains(&mod_index) {
            self.selected_mod_indices
                .iter()
                .copied()
                .filter(|index| *index < self.mods.len())
                .collect()
        } else {
            vec![mod_index]
        };

        Some(ModContextMenuData {
            clicked_mod_index: mod_index,
            mod_indices,
            name: mod_info.name().to_string(),
            author: mod_info.author().to_string(),
            mod_id: mod_info.mod_id().map(|id| id.to_string()).unwrap_or_default(),
            category: mod_info.category().to_string(),
        })
    }

    fn handle_mod_row_response(&mut self, response: &egui::Response, mod_index: usize) -> bool {
        if response.clicked_by(egui::PointerButton::Primary) {
            if self.selection_handled_mod_indices.insert(mod_index) {
                let modifiers = response.ctx.input(|input| input.modifiers);
                let toggle_selection = modifiers.command || modifiers.ctrl;

                if modifiers.shift {
                    let anchor = if let Some(anchor) = *self.selection_anchor {
                        anchor
                    } else {
                        *self.selection_anchor = Some(mod_index);
                        mod_index
                    };
                    if let Some(range) = self.visible_mod_range(anchor, mod_index) {
                        if !toggle_selection {
                            self.selected_mod_indices.clear();
                        }
                        self.selected_mod_indices.extend(range);
                    } else {
                        if !toggle_selection {
                            self.selected_mod_indices.clear();
                        }
                        self.selected_mod_indices.insert(mod_index);
                        *self.selection_anchor = Some(mod_index);
                    }
                } else if toggle_selection {
                    if !self.selected_mod_indices.remove(&mod_index) {
                        self.selected_mod_indices.insert(mod_index);
                    }
                    *self.selection_anchor = Some(mod_index);
                } else {
                    self.selected_mod_indices.clear();
                    self.selected_mod_indices.insert(mod_index);
                    *self.selection_anchor = Some(mod_index);
                }
            }
        }

        if response.secondary_clicked() {
            if !self.context_menu_handled_mod_indices.insert(mod_index) {
                return false;
            }
            if !self.selected_mod_indices.contains(&mod_index) {
                self.selected_mod_indices.clear();
                self.selected_mod_indices.insert(mod_index);
                *self.selection_anchor = Some(mod_index);
            }
        }

        true
    }

    fn attach_mod_context_menu(&mut self, response: &egui::Response, mod_index: usize) {
        if !self.handle_mod_row_response(response, mod_index) {
            return;
        }

        let Some(data) = self.mod_context_menu_data(mod_index) else {
            return;
        };
        let selected_count = data.mod_indices.len();

        response.context_menu(|ui| {
            if selected_count == 1 {
                if ui.button("Rename mod").clicked() {
                    *self.edit_modal = Some(ModEditModal::Rename {
                        mod_index: data.clicked_mod_index,
                        name: data.name.clone(),
                    });
                    ui.close();
                }
            }
            if ui.button("Change author").clicked() {
                *self.edit_modal = Some(ModEditModal::ChangeAuthor {
                    mod_indices: data.mod_indices.clone(),
                    author: data.author.clone(),
                });
                ui.close();
            }
            if selected_count == 1 {
                if ui.button("Set mod id").clicked() {
                    *self.edit_modal = Some(ModEditModal::SetModId {
                        mod_index: data.clicked_mod_index,
                        mod_id: data.mod_id.clone(),
                    });
                    ui.close();
                }
            }
            if ui.button("Choose category").clicked() {
                *self.edit_modal = Some(ModEditModal::ChooseCategory {
                    mod_indices: data.mod_indices.clone(),
                    category: data.category.clone(),
                });
                ui.close();
            }
            if ui.button("Reset to default").clicked() {
                for mod_index in data.mod_indices.iter().copied() {
                    if let Some(mod_info) = self.mods.get_mut(mod_index) {
                        mod_info.reset_to_default(self.categories);
                        *self.needs_save = true;
                    }
                }
                ui.close();
            }
        });
    }

    fn render_header_cell(&mut self, ui: &mut Ui, column: ModTableColumn) {
        Self::cell_frame().show(ui, |ui| {
            let Some(sort_column) = column.sort_column() else {
                ui.label(column.label());
                return;
            };

            let response = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let mut response =
                        ui.add(egui::Label::new(column.label()).sense(egui::Sense::click()));

                    if *self.sort_column == Some(sort_column) {
                        let icon = match self.sort_direction {
                            SortDirection::Ascending => ICON_KEYBOARD_ARROW_UP,
                            SortDirection::Descending => ICON_KEYBOARD_ARROW_DOWN,
                        };
                        response |= ui
                            .with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(egui::Label::new(icon).sense(egui::Sense::click()))
                            })
                            .inner;
                    }

                    response
                })
                .inner;

            if response.clicked() {
                match (*self.sort_column, *self.sort_direction) {
                    (Some(current), SortDirection::Ascending) if current == sort_column => {
                        *self.sort_direction = SortDirection::Descending;
                    }
                    (Some(current), SortDirection::Descending) if current == sort_column => {
                        *self.sort_direction = SortDirection::Ascending;
                    }
                    _ => {
                        *self.sort_column = Some(sort_column);
                        *self.sort_direction = SortDirection::Ascending;
                    }
                }
            }
        });
    }

    fn render_summary_cell(&mut self, ui: &mut Ui, row_nr: u64, column: ModTableColumn) {
        Self::cell_frame().show(ui, |ui| {
            let Some(row) = self.row(row_nr) else {
                return;
            };
            let mod_index = row.mod_index;

            if self.mods.get(mod_index).is_none() {
                return;
            }

            let response = match column {
                ModTableColumn::Enabled => {
                    let has_files = self
                        .mods
                        .get(mod_index)
                        .is_some_and(|mod_info| !mod_info.files().is_empty());
                    let is_expanded = self.expanded_mod_indices.contains(&mod_index);
                    let enabled_file_count = self
                        .mods
                        .get(mod_index)
                        .map(|mod_info| {
                            mod_info.files().iter().filter(|file_info| file_info.enabled()).count()
                        })
                        .unwrap_or_default();
                    let mut enabled = has_files
                        && self
                            .mods
                            .get(mod_index)
                            .is_some_and(|mod_info| enabled_file_count == mod_info.files().len());
                    let indeterminate = has_files && enabled_file_count > 0 && !enabled;
                    let mut enabled_changed = false;

                    let response = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let mut response = if has_files {
                                let icon = if is_expanded {
                                    ICON_KEYBOARD_ARROW_DOWN
                                } else {
                                    ICON_KEYBOARD_ARROW_RIGHT
                                };
                                let response = ui.add(
                                    egui::Button::new(icon)
                                        .frame(false)
                                        .min_size(egui::vec2(18.0, 18.0)),
                                );
                                if response.clicked() {
                                    if is_expanded {
                                        self.expanded_mod_indices.remove(&mod_index);
                                    } else {
                                        self.expanded_mod_indices.insert(mod_index);
                                    }
                                }
                                response
                            } else {
                                ui.allocate_response(egui::vec2(18.0, 18.0), egui::Sense::hover())
                            };

                            let checkbox = ui.add(
                                egui::Checkbox::without_text(&mut enabled)
                                    .indeterminate(indeterminate),
                            );
                            if checkbox.changed() {
                                enabled_changed = true;
                            }
                            response |= checkbox;
                            response
                        })
                        .inner;

                    if enabled_changed {
                        if let Some(mod_info) = self.mods.get_mut(mod_index) {
                            for file_info in mod_info.files_mut() {
                                file_info.set_enabled(enabled);
                            }
                            *self.needs_save = true;
                        }
                    }
                    response
                }
                ModTableColumn::Name => {
                    let name =
                        self.mods.get(mod_index).map(|mod_info| mod_info.name()).unwrap_or("");
                    ui.add(egui::Label::new(name).wrap().sense(egui::Sense::click()))
                }
                ModTableColumn::Author => ui.add(
                    egui::Label::new(
                        self.mods.get(mod_index).map(|mod_info| mod_info.author()).unwrap_or(""),
                    )
                    .truncate()
                    .sense(egui::Sense::click()),
                ),
                ModTableColumn::ModId => {
                    if let Some(id) =
                        self.mods.get(mod_index).and_then(|mod_info| mod_info.mod_id())
                    {
                        ui.add(egui::Label::new(id.to_string()).sense(egui::Sense::click()))
                    } else {
                        ui.add(egui::Label::new("").sense(egui::Sense::click()))
                    }
                }
                ModTableColumn::Category => ui.add(
                    egui::Label::new(
                        self.mods.get(mod_index).map(|mod_info| mod_info.category()).unwrap_or(""),
                    )
                    .truncate()
                    .sense(egui::Sense::click()),
                ),
                ModTableColumn::LastModified => ui.add(
                    egui::Label::new(
                        self.mods
                            .get(mod_index)
                            .map(|mod_info| mod_info.last_modified())
                            .unwrap_or(""),
                    )
                    .sense(egui::Sense::click()),
                ),
            };

            self.attach_mod_context_menu(&response, mod_index);
        });
    }

    fn render_file_cell(&mut self, ui: &mut Ui, row: ModTableRow, column: ModTableColumn) {
        let ModTableRowKind::File { file_index } = row.kind else {
            return;
        };

        Self::cell_frame().show(ui, |ui| match column {
            ModTableColumn::Enabled => {
                ui.add_space(22.0);
                if let Some(file_info) = self
                    .mods
                    .get_mut(row.mod_index)
                    .and_then(|mod_info| mod_info.files_mut().get_mut(file_index))
                {
                    let mut enabled = file_info.enabled();
                    if ui.checkbox(&mut enabled, "").changed() {
                        file_info.set_enabled(enabled);
                        *self.needs_save = true;
                    }
                }
            }
            ModTableColumn::Name => {
                let subpath = self
                    .mods
                    .get(row.mod_index)
                    .and_then(|mod_info| mod_info.files().get(file_index))
                    .map(|file_info| file_info.subpath().display().to_string())
                    .unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.add(
                        egui::Label::new(egui::RichText::new(subpath).small())
                            .truncate()
                            .sense(egui::Sense::click()),
                    );
                });
            }
            ModTableColumn::Author
            | ModTableColumn::ModId
            | ModTableColumn::Category
            | ModTableColumn::LastModified => {
                ui.label("");
            }
        });
    }

    fn cell_frame() -> Frame {
        Frame::NONE
            .inner_margin(Margin::symmetric(MODS_TABLE_CELL_MARGIN_X, MODS_TABLE_CELL_MARGIN_Y))
    }
}

impl TableDelegate for ModsTableDelegate<'_> {
    fn header_cell_ui(&mut self, ui: &mut Ui, cell: &HeaderCellInfo) {
        let Some(column) = ModTableColumn::from_index(cell.col_range.start) else {
            return;
        };

        self.render_header_cell(ui, column);
    }

    fn row_ui(&mut self, ui: &mut Ui, row_nr: u64) {
        let Some(row) = self.row(row_nr) else {
            return;
        };

        let row_color = match row.kind {
            ModTableRowKind::Summary if self.selected_mod_indices.contains(&row.mod_index) => {
                ui.visuals().selection.bg_fill
            }
            ModTableRowKind::Summary if row_nr % 2 == 0 => ui.visuals().extreme_bg_color,
            ModTableRowKind::File { .. } => ui.visuals().faint_bg_color,
            ModTableRowKind::Summary => Color32::TRANSPARENT,
        };
        ui.painter().rect_filled(ui.max_rect(), 0.0, row_color);

        if row.kind == ModTableRowKind::Summary {
            let response = ui.interact(
                ui.max_rect(),
                ui.id().with(("mods_table_context_menu", row_nr)),
                egui::Sense::click(),
            );
            self.attach_mod_context_menu(&response, row.mod_index);
        }
    }

    fn cell_ui(&mut self, ui: &mut Ui, cell: &CellInfo) {
        let Some(row) = self.row(cell.row_nr) else {
            return;
        };
        let Some(column) = ModTableColumn::from_index(cell.col_nr) else {
            return;
        };

        match row.kind {
            ModTableRowKind::Summary => self.render_summary_cell(ui, cell.row_nr, column),
            ModTableRowKind::File { .. } => self.render_file_cell(ui, row, column),
        }
    }

    fn row_top_offset(&self, _ctx: &egui::Context, _table_id: Id, row_nr: u64) -> f32 {
        usize::try_from(row_nr)
            .ok()
            .and_then(|index| self.row_tops.get(index))
            .copied()
            .unwrap_or_else(|| self.row_tops.last().copied().unwrap_or(0.0))
    }

    fn default_row_height(&self) -> f32 {
        MODS_TABLE_ROW_MIN_HEIGHT
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
            Self::default()
        }
    }

    fn main_page(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("categories_panel")
            .show_separator_line(true)
            .min_size(200.0)
            .show_inside(ui, |ui| {
                ui.heading("Categories");
                ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                    ScrollArea::vertical().max_width(f32::INFINITY).show(ui, |ui| {
                        ui.selectable_value(
                            &mut self.state.mods_category_filter,
                            CategoryFilter::None,
                            "All",
                        );
                        for i in self.categories.iter() {
                            ui.selectable_value(
                                &mut self.state.mods_category_filter,
                                CategoryFilter::Category(i.name().to_string()),
                                i.name(),
                            );
                        }
                        ui.selectable_value(
                            &mut self.state.mods_category_filter,
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
                    TextEdit::singleline(&mut self.state.mods_name_filter)
                        .hint_text("Filter mods..."),
                );
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_refreshing = self.state.mods_refresh_list.is_pending();
                    if ui.add_enabled(!is_refreshing, egui::Button::new("Apply Mods")).clicked() {
                        // TODO:
                    }
                    if ui.add_enabled(!is_refreshing, egui::Button::new("Clear Mods")).clicked() {
                        // TODO:
                    }
                    if ui.add_enabled(!is_refreshing, egui::Button::new("Refresh Mods")).clicked() {
                        let input_folder = self.settings.input_folder().clone();
                        let matchers = self.categories.clone();
                        let api_key = self.settings.nexusmods_api_key().to_string();
                        let nexusmods_mod_cache = self.nexusmods_mod_cache.clone();
                        self.state.mods_refresh_list.request(async move {
                            refresh_mods_with_nexusmods(
                                input_folder,
                                matchers,
                                api_key,
                                nexusmods_mod_cache,
                            )
                            .await
                        });
                    }
                });
            });

            if let Some(refresh_result) = self.state.mods_refresh_list.read().clone() {
                match refresh_result {
                    Ok(refresh_result) => {
                        self.mods = ModList::new(refresh_result.mods);
                        self.nexusmods_mod_cache = refresh_result.nexusmods_mod_cache;
                        for error in refresh_result.toast_errors {
                            self.add_error_toast(error);
                        }
                        self.state.misc_needs_save = true;
                    }
                    Err(err) => {
                        self.add_error_toast(format!("Failed to refresh mods: {err}"));
                    }
                }
                self.state.mods_refresh_list.clear();
            }

            if self.state.mods_refresh_list.is_pending() {
                self.render_mods_loading(ui);
            } else {
                self.render_mods_table(ui);
            }
            self.render_mod_edit_modal(ui);
        });
    }

    fn render_mods_loading(&mut self, ui: &mut Ui) {
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.centered_and_justified(|ui| {
                ui.add(egui::Spinner::new().size(32.0));
            });
        });
    }

    fn add_error_toast(&mut self, text: impl Into<String>) {
        self.state.toasts.add(Toast {
            kind: egui_toast::ToastKind::Error,
            text: text.into().into(),
            options: ToastOptions::default().show_progress(true).duration_in_seconds(5.0),
            ..Default::default()
        });
    }

    fn render_mod_edit_modal(&mut self, ui: &mut Ui) {
        let Some(mut edit_modal) = self.state.mods_edit_modal.take() else {
            return;
        };

        let mut keep_open = true;
        let modal = Modal::new(Id::new("Mod Edit Modal")).show(ui.ctx(), |ui| {
            ui.set_width(420.0);

            match &mut edit_modal {
                ModEditModal::Rename { mod_index, name } => {
                    ui.heading("Rename Mod");
                    ui.add(TextEdit::singleline(name).desired_width(ui.available_width()));
                    self.render_mod_edit_actions(ui, &mut keep_open, |app| {
                        if let Some(mod_info) = app.mods.mods_mut().get_mut(*mod_index) {
                            mod_info.set_name(name.trim().to_string());
                            app.state.misc_needs_save = true;
                        }
                        true
                    });
                }
                ModEditModal::ChangeAuthor { mod_indices, author } => {
                    ui.heading(if mod_indices.len() == 1 {
                        "Change Author"
                    } else {
                        "Change Authors"
                    });
                    let existing_authors = self.existing_mod_authors();
                    let available_width = ui.available_width();
                    ui.add(
                        AutoCompleteTextEdit::new(author, &existing_authors)
                            .max_suggestions(8)
                            .highlight_matches(true)
                            .popup_on_focus(true)
                            .width(available_width)
                            .set_text_edit_properties(move |text_edit| {
                                text_edit.desired_width(available_width)
                            }),
                    );
                    self.render_mod_edit_actions(ui, &mut keep_open, |app| {
                        for mod_index in mod_indices.iter().copied() {
                            if let Some(mod_info) = app.mods.mods_mut().get_mut(mod_index) {
                                mod_info.set_author(author.trim().to_string());
                                app.state.misc_needs_save = true;
                            }
                        }
                        true
                    });
                }
                ModEditModal::SetModId { mod_index, mod_id } => {
                    ui.heading("Set Mod ID");
                    ui.add(TextEdit::singleline(mod_id).desired_width(ui.available_width()));
                    self.render_mod_edit_actions(ui, &mut keep_open, |app| {
                        let mod_id = mod_id.trim();
                        let parsed_mod_id = if mod_id.is_empty() {
                            None
                        } else {
                            match mod_id.parse::<u32>() {
                                Ok(mod_id) => Some(mod_id),
                                Err(err) => {
                                    app.add_error_toast(format!("Invalid mod id: {err}"));
                                    return false;
                                }
                            }
                        };

                        if let Some(mod_info) = app.mods.mods_mut().get_mut(*mod_index) {
                            mod_info.set_mod_id(parsed_mod_id);
                            app.state.misc_needs_save = true;
                        }
                        true
                    });
                }
                ModEditModal::ChooseCategory { mod_indices, category } => {
                    ui.heading(if mod_indices.len() == 1 {
                        "Choose Category"
                    } else {
                        "Choose Categories"
                    });
                    egui::ComboBox::from_id_salt("mod_category_picker")
                        .selected_text(category.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                category,
                                "Uncategorized".to_string(),
                                "Uncategorized",
                            );
                            for category_matcher in &self.categories {
                                ui.selectable_value(
                                    category,
                                    category_matcher.name().to_string(),
                                    category_matcher.name(),
                                );
                            }
                        });
                    self.render_mod_edit_actions(ui, &mut keep_open, |app| {
                        for mod_index in mod_indices.iter().copied() {
                            if let Some(mod_info) = app.mods.mods_mut().get_mut(mod_index) {
                                mod_info.set_category(category.clone());
                                app.state.misc_needs_save = true;
                            }
                        }
                        true
                    });
                }
            }
        });

        if modal.should_close() {
            keep_open = false;
        }

        if keep_open {
            self.state.mods_edit_modal = Some(edit_modal);
        }
    }

    fn render_mod_edit_actions(
        &mut self,
        ui: &mut Ui,
        keep_open: &mut bool,
        mut save: impl FnMut(&mut Self) -> bool,
    ) {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save").clicked() && save(self) {
                    *keep_open = false;
                    ui.close();
                }
                if ui.button("Cancel").clicked() {
                    *keep_open = false;
                    ui.close();
                }
            });
        });
    }

    fn existing_mod_authors(&self) -> Vec<String> {
        let mut authors = self
            .mods
            .mods()
            .iter()
            .map(|mod_info| mod_info.author().trim())
            .filter(|author| !author.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        authors.sort();
        authors.dedup();
        authors
    }

    fn render_mods_table(&mut self, ui: &mut Ui) {
        let mods_count = self.mods.mods().len();
        self.state.mods_selected_indices.retain(|index| *index < mods_count);
        self.state.mods_expanded_indices.retain(|index| *index < mods_count);
        if self.state.mods_selection_anchor.is_some_and(|index| index >= mods_count) {
            self.state.mods_selection_anchor = None;
        }

        let rows = {
            let mods = self.mods.mods();
            let name_filter = self.state.mods_name_filter.to_lowercase();
            let mut filtered_mods: Vec<_> = mods
                .iter()
                .enumerate()
                .filter(|m| {
                    let mod_info = m.1;
                    let name_matches = mod_info.name().to_lowercase().contains(&name_filter);
                    let category_matches = match &self.state.mods_category_filter {
                        CategoryFilter::None => true,
                        CategoryFilter::Category(cat) => mod_info.category() == cat,
                        CategoryFilter::Uncategorized => mod_info.category() == "Uncategorized",
                    };
                    name_matches && category_matches
                })
                .map(|m| m.0)
                .collect();

            if let Some(sort_col) = self.state.mods_sort_column {
                filtered_mods.sort_by(|a, b| {
                    let a = &mods[*a];
                    let b = &mods[*b];
                    let cmp = match sort_col {
                        ModSortColumn::Name => a.name().cmp(b.name()),
                        ModSortColumn::Author => a.author().cmp(b.author()),
                        ModSortColumn::ModId => a.mod_id().cmp(&b.mod_id()),
                        ModSortColumn::Category => a.category().cmp(b.category()),
                        ModSortColumn::LastModified => a.last_modified().cmp(b.last_modified()),
                    };

                    match self.state.mods_sort_direction {
                        SortDirection::Ascending => cmp,
                        SortDirection::Descending => cmp.reverse(),
                    }
                });
            }

            let mut rows = Vec::with_capacity(filtered_mods.len());
            for mod_index in filtered_mods {
                rows.push(ModTableRow::summary(mod_index));
                if self.state.mods_expanded_indices.contains(&mod_index) {
                    let file_count = mods[mod_index].files().len();
                    rows.extend(
                        (0..file_count).map(|file_index| ModTableRow::file(mod_index, file_index)),
                    );
                }
            }
            rows
        };

        let columns = self.mods_table_columns(ui);
        let name_width = columns[ModTableColumn::Name.index()].current
            - f32::from(MODS_TABLE_CELL_MARGIN_X) * 2.0;
        let row_tops = self.mods_table_row_tops(ui, &rows, name_width.max(1.0));
        let row_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);

        let mut delegate = ModsTableDelegate {
            mods: self.mods.mods_mut(),
            rows,
            row_tops,
            selection_handled_mod_indices: Default::default(),
            context_menu_handled_mod_indices: Default::default(),
            categories: &self.categories,
            edit_modal: &mut self.state.mods_edit_modal,
            needs_save: &mut self.state.misc_needs_save,
            selected_mod_indices: &mut self.state.mods_selected_indices,
            selection_anchor: &mut self.state.mods_selection_anchor,
            expanded_mod_indices: &mut self.state.mods_expanded_indices,
            sort_column: &mut self.state.mods_sort_column,
            sort_direction: &mut self.state.mods_sort_direction,
        };

        Table::new()
            .id_salt(MODS_TABLE_ID)
            .columns(columns)
            .headers([HeaderRow::new(MODS_TABLE_HEADER_HEIGHT)])
            .auto_size_mode(AutoSizeMode::Always)
            .num_rows(row_count)
            .show(ui, &mut delegate);
    }

    fn mods_table_columns(&self, ui: &Ui) -> Vec<Column> {
        let mut columns = vec![
            Column::new(72.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::Enabled.index())))
                .range(Rangef::new(72.0, 72.0))
                .resizable(false),
            Column::new(260.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::Name.index())))
                .range(Rangef::new(160.0, f32::INFINITY))
                .resizable(false),
            Column::new(140.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::Author.index())))
                .range(Rangef::new(140.0, 140.0))
                .resizable(false),
            Column::new(80.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::ModId.index())))
                .range(Rangef::new(80.0, 80.0))
                .resizable(false),
            Column::new(120.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::Category.index())))
                .range(Rangef::new(120.0, 120.0))
                .resizable(false),
            Column::new(152.0)
                .id(Id::new((MODS_TABLE_ID, ModTableColumn::LastModified.index())))
                .range(Rangef::new(152.0, 152.0))
                .resizable(false),
        ];

        Column::auto_size(&mut columns, ui.available_width());

        columns
    }

    fn mods_table_row_tops(&self, ui: &Ui, rows: &[ModTableRow], name_width: f32) -> Vec<f32> {
        let mut row_tops = Vec::with_capacity(rows.len() + 1);
        let mut next_top = 0.0;
        row_tops.push(next_top);

        for row in rows {
            let height = match row.kind {
                ModTableRowKind::Summary => self.mods_table_summary_row_height(
                    ui,
                    &self.mods.mods()[row.mod_index],
                    name_width,
                ),
                ModTableRowKind::File { .. } => MODS_TABLE_FILE_ROW_HEIGHT,
            };
            next_top += height;
            row_tops.push(next_top);
        }

        row_tops
    }

    fn mods_table_summary_row_height(&self, ui: &Ui, mod_info: &ModInfo, name_width: f32) -> f32 {
        let galley = WidgetText::from(mod_info.name()).into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            name_width,
            TextStyle::Body,
        );
        let vertical_margin = f32::from(MODS_TABLE_CELL_MARGIN_Y) * 2.0;
        (galley.size().y + vertical_margin).max(MODS_TABLE_ROW_MIN_HEIGHT)
    }

    fn settings_page(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
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
                    ui.label("NexusMods Cache:");
                    ui.horizontal(|ui| {
                        if ui.button("Clear Cache").clicked() {
                            self.nexusmods_mod_cache.clear();
                            self.state.misc_needs_save = true;
                        }
                        ui.label(format!("{} mods cached", self.nexusmods_mod_cache.len()));
                    });
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
                                self.state.settings_game_folder.request((|| async {
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
                                self.state.settings_input_folder.request((|| async {
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
                if let Some(Ok(x)) = self.state.settings_game_folder.read() {
                    self.settings.set_game_folder(x.clone());
                    self.state.settings_game_folder.clear();
                }
                if let Some(Ok(x)) = self.state.settings_input_folder.read() {
                    self.settings.set_input_folder(x.clone());
                    self.state.settings_input_folder.clear();
                }
                if self.settings.game_folder().exists() && self.settings.input_folder().exists() {
                    self.first_time_setup = false;
                }
            });
        });
    }

    fn categories_page(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                ui.set_max_width(800.0);
                ui.horizontal(|ui| {
                    ui.heading("Category Management");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Use Default Matchers").clicked() {
                            self.categories = default_matchers();
                            self.state.misc_needs_save = true;
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
                            TextEdit::singleline(&mut self.state.categories_new_category)
                                .desired_width(ui.available_width()),
                        );
                        if button.clicked()
                            || (text.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            let item = self.categories.iter().position(|e| {
                                e.name().eq_ignore_ascii_case(
                                    &self.state.categories_new_category.clone(),
                                )
                            });
                            if item.is_none() {
                                self.categories.push(CategoryMatcher::new(
                                    self.state.categories_new_category.clone(),
                                    vec![],
                                ));
                            }

                            self.state.categories_new_category.clear();
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
                                        self.state.categories_modal_is_open = true;
                                        self.state.categories_modal_idx = idx;
                                        self.state.categories_modal_matchers = category
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

                if self.state.categories_modal_is_open {
                    let modal = Modal::new(Id::new("Category Edit Modal")).show(ui.ctx(), |ui| {
                        ui.set_width(500.0);
                        ui.heading(format!(
                            "Edit \"{}\" Matchers",
                            self.categories[self.state.categories_modal_idx].name()
                        ));
                        let mut to_remove = None;
                        Grid::new("Matcher Modal").num_columns(1).show(ui, |ui| {
                            for (idx, matcher) in
                                self.state.categories_modal_matchers.iter_mut().enumerate()
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
                            self.state.categories_modal_matchers.remove(idx);
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Add Matcher").clicked() {
                                self.state.categories_modal_matchers.push(String::new());
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Save").clicked() {
                                        let mut errors = vec![];
                                        let mut matchers = vec![];
                                        for matcher in self.state.categories_modal_matchers.iter() {
                                            match Regex::new(matcher) {
                                                Ok(re) => matchers.push(re.into()),
                                                Err(e) => errors.push(format!("{e}: {matcher}")),
                                            }
                                        }
                                        if errors.is_empty() {
                                            self.categories[self.state.categories_modal_idx]
                                                .set_matchers(matchers);
                                            ui.close();
                                        } else {
                                            self.state.toasts.add(Toast {
                                                kind: egui_toast::ToastKind::Error,
                                                text: format!(
                                                    "Failed to parse regex: \n\t{}",
                                                    errors.join("\t\n")
                                                )
                                                .into(),
                                                options: ToastOptions::default()
                                                    .show_progress(true)
                                                    .duration_in_seconds(5.0),
                                                ..Default::default()
                                            });
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
                        self.state.categories_modal_is_open = false;
                    }
                }
            });
        });
    }
}

impl eframe::App for App {
    fn auto_save_interval(&self) -> std::time::Duration {
        if self.state.misc_needs_save {
            std::time::Duration::from_secs(0)
        } else {
            std::time::Duration::from_secs(30)
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
        self.state.misc_needs_save = false;
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after_for(Duration::from_secs(1), ctx.viewport_id());
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.state.toasts =
            Toasts::new().anchor(Align2::RIGHT_TOP, (-15.0, 15.0)).direction(Direction::TopDown);
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
            if self.current_page != Page::Settings {
                self.state.toasts.add(Toast {
                    kind: egui_toast::ToastKind::Error,
                    text:
                        "You must first set your game folder and mod folder before you can continue"
                            .into(),
                    options: ToastOptions::default().show_progress(true).duration_in_seconds(5.0),
                    ..Default::default()
                });
            }
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
        self.state.toasts.show(ui);
    }
}
