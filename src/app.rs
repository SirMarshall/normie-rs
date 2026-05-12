use crate::config::Config;
use crate::db::{Database, NormalizationState};
use crate::utils::StatefulList;
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

pub enum Action {
    None,
    Quit,
    Navigate(AppState),
}

pub enum AppState {
    Dashboard(DashboardState),
    FileBrowser(FileBrowserState),
    Normalizer(Box<NormalizerApp>),
}

pub struct App {
    pub state: AppState,
    pub config: Config,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Dashboard(DashboardState::new()),
            config: Config::load(),
            should_quit: false,
        }
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        let action = match &mut self.state {
            AppState::Dashboard(state) => state.handle_event(&mut self.config, key),
            AppState::FileBrowser(state) => state.handle_event(&mut self.config, key),
            AppState::Normalizer(state) => state.handle_event(&mut self.config, key),
        };

        match action {
            Action::None => {}
            Action::Quit => self.should_quit = true,
            Action::Navigate(new_state) => self.state = new_state,
        }

        Ok(())
    }
}

pub struct DashboardState {
    pub menu: StatefulList<String>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            menu: StatefulList::with_items(vec!["📂 Open New Session".to_string()]),
        }
    }

    pub fn update_menu(&mut self, config: &Config) {
        let mut items = vec!["📂 Open New Session".to_string()];
        for path in &config.recent_files {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("Unknown");
            items.push(format!("📄 Recent: {}", name));
        }
        items.push("🚪 Quit".to_string());

        let selected = self.menu.state.selected();
        self.menu.items = items;
        if let Some(i) = selected {
            if i < self.menu.items.len() {
                self.menu.state.select(Some(i));
            } else {
                self.menu.state.select(Some(0));
            }
        } else {
            self.menu.state.select(Some(0));
        }
    }

    pub fn handle_event(&mut self, config: &mut Config, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Up => {
                self.menu.previous();
                Action::None
            }
            KeyCode::Down => {
                self.menu.next();
                Action::None
            }
            KeyCode::Enter => {
                let i = self.menu.state.selected().unwrap_or(0);
                let recent_len = config.recent_files.len();
                if i == 0 {
                    Action::Navigate(AppState::FileBrowser(FileBrowserState::new(
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    )))
                } else if i <= recent_len {
                    let path = config.recent_files[i - 1].clone();
                    if let Ok(normalizer) = load_normalizer(path.clone()) {
                        config.add_recent(path);
                        Action::Navigate(AppState::Normalizer(Box::new(normalizer)))
                    } else {
                        Action::None
                    }
                } else {
                    Action::Quit
                }
            }
            _ => Action::None,
        }
    }
}

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: StatefulList<PathBuf>,
}

impl FileBrowserState {
    pub fn new(dir: PathBuf) -> Self {
        let mut s = Self {
            current_dir: dir,
            entries: StatefulList::new(),
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        let mut items: Vec<PathBuf> = fs::read_dir(&self.current_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_dir()
                            || p.extension()
                                .map(|ext| ext == "csv")
                                .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default();
        items.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });
        self.entries = StatefulList::with_items(items);
    }

    pub fn handle_event(&mut self, config: &mut Config, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::Navigate(AppState::Dashboard(DashboardState::new())),
            KeyCode::Up => {
                self.entries.previous();
                Action::None
            }
            KeyCode::Down => {
                self.entries.next();
                Action::None
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.current_dir.parent() {
                    self.current_dir = parent.to_path_buf();
                    self.refresh();
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(i) = self.entries.state.selected() {
                    let path = self.entries.items[i].clone();
                    if path.is_dir() {
                        self.current_dir = path;
                        self.refresh();
                        Action::None
                    } else {
                        if let Ok(normalizer) = load_normalizer(path.clone()) {
                            config.add_recent(path);
                            Action::Navigate(AppState::Normalizer(Box::new(normalizer)))
                        } else {
                            Action::None
                        }
                    }
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }
}

#[derive(PartialEq)]
pub enum ActivePane {
    Columns,
    Values,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
    SelectingKey,
    Searching,
}

pub struct UiState {
    pub active_pane: ActivePane,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub search_buffer: String,
    pub filtered_indices: Vec<usize>,
    pub col_list_state: ListState,
    pub val_list_state: ListState,
    pub key_list_state: ListState,
    pub multi_selected: HashSet<(usize, String)>,
    pub registry_key_col: usize,
    pub info_results: Vec<String>,
}

pub struct NormalizerApp {
    pub path: PathBuf,
    pub db: Database,
    pub norm: NormalizationState,
    pub ui: UiState,
    pub status_msg: String,
}

impl NormalizerApp {
    pub fn new(path: PathBuf, headers: csv::StringRecord, rows: Vec<Vec<String>>) -> Self {
        let headers: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
        let mut row_registry: Vec<HashMap<String, Vec<usize>>> =
            vec![HashMap::new(); headers.len()];
        let mut unique_values_sets: Vec<HashSet<String>> = vec![HashSet::new(); headers.len()];

        for (r_idx, row) in rows.iter().enumerate() {
            for (c_idx, val) in row.iter().enumerate() {
                let trimmed = val.trim().to_string();
                unique_values_sets[c_idx].insert(trimmed.clone());
                row_registry[c_idx].entry(trimmed).or_default().push(r_idx);
            }
        }

        let unique_values: Vec<Vec<(String, String)>> = unique_values_sets
            .into_iter()
            .map(|set| {
                let mut v: Vec<String> = set.into_iter().collect();
                v.sort_by_cached_key(|a| a.to_lowercase());
                v.into_iter()
                    .map(|s| {
                        let lower = s.to_lowercase();
                        (s, lower)
                    })
                    .collect()
            })
            .collect();

        let mut col_state = ListState::default();
        col_state.select(Some(0));

        let initial_key_idx = headers
            .iter()
            .position(|h| h.to_lowercase() == "id")
            .unwrap_or(0);

        let mut app = Self {
            norm: NormalizationState::load(&path),
            path,
            db: Database {
                headers,
                rows,
                unique_values,
                row_registry,
            },
            ui: UiState {
                active_pane: ActivePane::Columns,
                input_mode: InputMode::Normal,
                input_buffer: String::new(),
                search_buffer: String::new(),
                filtered_indices: Vec::new(),
                col_list_state: col_state,
                val_list_state: ListState::default(),
                key_list_state: ListState::default(),
                multi_selected: HashSet::new(),
                registry_key_col: initial_key_idx,
                info_results: Vec::new(),
            },
            status_msg: "Ready (Mappings Loaded)".to_string(),
        };
        app.update_filtered_vals();
        app
    }

    pub fn handle_event(&mut self, _config: &mut Config, key: KeyEvent) -> Action {
        match self.ui.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Action::Navigate(AppState::Dashboard(DashboardState::new()));
                }
                KeyCode::Char('s') => {
                    let _ = self.save_csv();
                }
                KeyCode::Tab => {
                    self.ui.active_pane = if self.ui.active_pane == ActivePane::Columns {
                        ActivePane::Values
                    } else {
                        ActivePane::Columns
                    };
                }
                KeyCode::Down => self.move_list(1),
                KeyCode::Up => self.move_list(-1),
                KeyCode::Char(' ') => self.toggle_select(),
                KeyCode::Char('k') => self.ui.input_mode = InputMode::SelectingKey,
                KeyCode::Char('/') => {
                    self.ui.input_mode = InputMode::Searching;
                }
                KeyCode::Enter => {
                    if self.ui.active_pane == ActivePane::Values {
                        self.ui.input_mode = InputMode::Editing;
                        self.ui.input_buffer.clear();
                    }
                }
                KeyCode::Delete | KeyCode::Backspace => self.delete_current_mapping(),
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    self.apply_mapping();
                    self.ui.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => self.ui.input_buffer.push(c),
                KeyCode::Backspace => {
                    self.ui.input_buffer.pop();
                }
                KeyCode::Esc => self.ui.input_mode = InputMode::Normal,
                _ => {}
            },
            InputMode::Searching => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.ui.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    self.ui.search_buffer.push(c);
                    self.update_filtered_vals();
                }
                KeyCode::Backspace => {
                    self.ui.search_buffer.pop();
                    self.update_filtered_vals();
                }
                _ => {}
            },
            InputMode::SelectingKey => match key.code {
                KeyCode::Up => {
                    let i = self.ui.key_list_state.selected().unwrap_or(0);
                    let next = if i == 0 {
                        self.db.headers.len() - 1
                    } else {
                        i - 1
                    };
                    self.ui.key_list_state.select(Some(next));
                }
                KeyCode::Down => {
                    let i = self.ui.key_list_state.selected().unwrap_or(0);
                    let next = if i >= self.db.headers.len() - 1 {
                        0
                    } else {
                        i + 1
                    };
                    self.ui.key_list_state.select(Some(next));
                }
                KeyCode::Enter => {
                    if let Some(i) = self.ui.key_list_state.selected() {
                        self.ui.registry_key_col = i;
                        self.calculate_info();
                    }
                    self.ui.input_mode = InputMode::Normal;
                }
                KeyCode::Esc | KeyCode::Char('k') => self.ui.input_mode = InputMode::Normal,
                _ => {}
            },
        }
        Action::None
    }

    pub fn move_list(&mut self, delta: i32) {
        match self.ui.active_pane {
            ActivePane::Columns => {
                let len = self.db.headers.len();
                if len == 0 {
                    return;
                }
                let i = self.ui.col_list_state.selected().unwrap_or(0) as i32;
                let next = (i + delta).rem_euclid(len as i32) as usize;
                self.ui.col_list_state.select(Some(next));
                self.ui.search_buffer.clear();
                self.update_filtered_vals();
                self.ui.val_list_state.select(Some(0));
            }
            ActivePane::Values => {
                let len = self.ui.filtered_indices.len();
                if len == 0 {
                    return;
                }
                let i = self.ui.val_list_state.selected().unwrap_or(0) as i32;
                let next = (i + delta).rem_euclid(len as i32) as usize;
                self.ui.val_list_state.select(Some(next));
            }
        };
    }

    pub fn toggle_select(&mut self) {
        if self.ui.active_pane != ActivePane::Values {
            return;
        }
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);
        if let Some(val_idx) = self.ui.val_list_state.selected() {
            if val_idx >= self.ui.filtered_indices.len() {
                return;
            }
            let orig_idx = self.ui.filtered_indices[val_idx];
            let val = self.db.unique_values[col_idx][orig_idx].0.clone();
            let key = (col_idx, val);
            if !self.ui.multi_selected.remove(&key) {
                self.ui.multi_selected.insert(key);
            }
            self.move_list(1);
        }
    }

    pub fn update_filtered_vals(&mut self) {
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);
        let search = self.ui.search_buffer.to_lowercase();
        self.ui.filtered_indices = self.db.unique_values[col_idx]
            .iter()
            .enumerate()
            .filter(|(_, (_, lower))| lower.contains(&search))
            .map(|(i, _)| i)
            .collect();

        if let Some(selected) = self.ui.val_list_state.selected() {
            if selected >= self.ui.filtered_indices.len() {
                self.ui
                    .val_list_state
                    .select(if self.ui.filtered_indices.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
            }
        } else if !self.ui.filtered_indices.is_empty() {
            self.ui.val_list_state.select(Some(0));
        }

        self.calculate_info();
    }

    pub fn calculate_info(&mut self) {
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);

        let mut results: Vec<String> = Vec::with_capacity(self.ui.filtered_indices.len());
        for &orig_idx in &self.ui.filtered_indices {
            let val = &self.db.unique_values[col_idx][orig_idx].0;
            let key_val = if let Some(indices) = self.db.row_registry[col_idx].get(val) {
                let mut keys: Vec<&str> = indices
                    .iter()
                    .map(|&idx| self.db.rows[idx][self.ui.registry_key_col].as_str())
                    .collect();
                keys.sort_unstable();
                keys.dedup();
                if keys.is_empty() {
                    "-".to_string()
                } else {
                    let mut joined = String::new();
                    for (i, &k) in keys.iter().enumerate() {
                        if i > 0 {
                            joined.push_str(", ");
                        }
                        if k.is_empty() {
                            joined.push_str("(EMPTY)");
                        } else {
                            joined.push_str(k);
                        }
                    }
                    joined
                }
            } else {
                "-".to_string()
            };
            results.push(key_val);
        }
        self.ui.info_results = results;
    }

    pub fn has_selections_in_current_col(&self) -> bool {
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);
        self.ui.multi_selected.iter().any(|(c, _)| *c == col_idx)
    }

    pub fn delete_current_mapping(&mut self) {
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);
        let col_name = self.db.headers[col_idx].clone();
        if let Some(val_idx) = self.ui.val_list_state.selected() {
            if val_idx >= self.ui.filtered_indices.len() {
                return;
            }
            let orig_idx = self.ui.filtered_indices[val_idx];
            let val = &self.db.unique_values[col_idx][orig_idx].0;
            if let Some(col_map) = self.norm.mappings.get_mut(&col_name) {
                col_map.remove(val);
                self.status_msg = format!("Removed mapping for '{}'", val);
                let _ = self.norm.save(&self.path);
            }
        }
    }

    pub fn apply_mapping(&mut self) {
        let col_idx = self.ui.col_list_state.selected().unwrap_or(0);
        let col_name = self.db.headers[col_idx].clone();

        let targets: Vec<String> = if !self.has_selections_in_current_col() {
            self.ui
                .val_list_state
                .selected()
                .and_then(|i| {
                    self.ui
                        .filtered_indices
                        .get(i)
                        .map(|&orig_idx| vec![self.db.unique_values[col_idx][orig_idx].0.clone()])
                })
                .unwrap_or_default()
        } else {
            let (mine, others): (HashSet<_>, HashSet<_>) = self
                .ui
                .multi_selected
                .drain()
                .partition(|(c, _)| *c == col_idx);
            self.ui.multi_selected = others;
            mine.into_iter().map(|(_, v)| v).collect()
        };

        if targets.is_empty() {
            self.status_msg = "Nothing selected".to_string();
            return;
        }

        let batch_size = targets.len();
        let total_mapped = {
            let col_map = self.norm.mappings.entry(col_name.clone()).or_default();
            for t in targets {
                col_map.insert(t, self.ui.input_buffer.clone());
            }
            col_map.len()
        };

        let _ = self.norm.save(&self.path);

        self.status_msg = format!(
            "Applied to {} items. Total mapped in '{}': {}",
            batch_size, col_name, total_mapped
        );
    }

    pub fn save_csv(&mut self) -> Result<()> {
        let stem = self.path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let output_path = self.path.with_file_name(format!("{}_normalized.csv", stem));
        let mut wtr = csv::Writer::from_path(&output_path)?;
        wtr.write_record(&self.db.headers)?;

        for row in &self.db.rows {
            let mut new_row = Vec::new();
            for (c_idx, val) in row.iter().enumerate() {
                let trimmed = val.trim();
                let col_name = &self.db.headers[c_idx];
                let final_val = self
                    .norm
                    .mappings
                    .get(col_name)
                    .and_then(|m| m.get(trimmed))
                    .cloned()
                    .unwrap_or_else(|| trimmed.to_string());
                new_row.push(final_val);
            }
            wtr.write_record(&new_row)?;
        }
        wtr.flush()?;
        self.status_msg = format!("SUCCESS: Saved to {:?}", output_path);
        Ok(())
    }
}

pub fn load_normalizer(path: PathBuf) -> Result<NormalizerApp> {
    let mut rdr = csv::ReaderBuilder::new()
        .from_path(&path)
        .context(format!("Could not find {:?}", path))?;
    let headers = rdr.headers()?.clone();
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    Ok(NormalizerApp::new(path, headers, rows))
}
