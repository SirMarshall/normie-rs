use crate::app::{ActivePane, App, AppState, DashboardState, FileBrowserState, InputMode, NormalizerApp};
use crate::config::Config;
use crate::utils::centered_rect;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    match &mut app.state {
        AppState::Dashboard(state) => {
            state.update_menu(&app.config);
            ui_dashboard(f, state, &app.config);
        }
        AppState::FileBrowser(state) => ui_file_browser(f, state),
        AppState::Normalizer(norm_app) => ui_normalizer(f, norm_app),
    }
}

fn ui_dashboard(f: &mut Frame, state: &mut DashboardState, _config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)].as_ref())
        .split(f.size());

    let title = Paragraph::new(vec![
        Line::from(""),
        Line::from(r#"  _   _    ____    ____    __  __   ___  _____     ____    ____  "#),
        Line::from(r#" | \ | |  / __ \  |  _ \  |  \/  | |_ _| | ____|   |  _ \  / ___| "#),
        Line::from(r#" |  \| | | |  | | | |_) | | |\/| |  | |  |  _|     | |_) | \___ \ "#),
        Line::from(r#" | |\  | | |__| | |  _ <  | |  | |  | |  | |___    |  _ <   ___) |"#),
        Line::from(r#" |_| \_|  \____/  |_| \_\ |_|  |_| |___| |_____|   |_| \_\ |____/ "#),
        Line::from(""),
    ])
    .style(Style::default().fg(Color::Cyan).bold())
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = state.menu.items.iter().map(|s| ListItem::new(format!(" {} ", s))).collect();

    let menu = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Main Menu "))
        .highlight_style(Style::default().bg(Color::Indexed(237)).fg(Color::Yellow).bold())
        .highlight_symbol(">> ");

    let area = centered_rect(50, 50, chunks[1]);
    f.render_stateful_widget(menu, area, &mut state.menu.state);
}

fn ui_file_browser(f: &mut Frame, state: &mut FileBrowserState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.size());

    let header = Paragraph::new(format!(" Path: {:?}", state.current_dir))
        .block(Block::default().borders(Borders::ALL).title(" File Browser "));
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = state.entries.items.iter().map(|p| {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("..");
        if p.is_dir() {
            ListItem::new(format!(" 📁 {}/", name)).style(Style::default().fg(Color::Blue))
        } else {
            ListItem::new(format!(" 📄 {}", name)).style(Style::default().fg(Color::Green))
        }
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Files "))
        .highlight_style(Style::default().bg(Color::Indexed(237)).fg(Color::Yellow).bold());
    f.render_stateful_widget(list, chunks[1], &mut state.entries.state);

    let footer = Paragraph::new(" ENTER: Open/Enter | BACKSPACE: Up | ESC: Dashboard ");
    f.render_widget(footer, chunks[2]);
}

fn ui_normalizer(f: &mut Frame, app: &mut NormalizerApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref())
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    let active_key_name = &app.db.headers[app.ui.registry_key_col];
    let col_items: Vec<ListItem> = app
        .db
        .headers
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            let style = if idx == app.ui.registry_key_col {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(h, style)]))
        })
        .collect();

    let col_block = Block::default()
        .borders(Borders::ALL)
        .title(" 1. Columns ")
        .border_style(
            Style::default().fg(if app.ui.active_pane == ActivePane::Columns {
                Color::Yellow
            } else {
                Color::White
            }),
        );

    f.render_stateful_widget(
        List::new(col_items)
            .block(col_block)
            .highlight_style(Style::default().bg(Color::Indexed(237))),
        main_chunks[0],
        &mut app.ui.col_list_state,
    );

    let col_idx = app.ui.col_list_state.selected().unwrap_or(0);
    let col_name = &app.db.headers[col_idx];
    let mut val_items = Vec::new();

    let col_map = app.norm.mappings.get(col_name);
    for &orig_idx in &app.ui.filtered_indices {
        let val = &app.db.unique_values[col_idx][orig_idx].0;
        let mut spans = vec![];
        let display_val = if val.is_empty() { "(EMPTY)" } else { val };

        if app.ui.multi_selected.contains(&(col_idx, val.clone())) {
            spans.push("[x] ".cyan().bold());
        } else if col_map.and_then(|m| m.get(val)).is_some() {
            spans.push("[✓] ".green());
        } else {
            spans.push("[ ] ".dark_gray());
        }

        if let Some(norm) = col_map.and_then(|m| m.get(val)) {
            spans.push(Span::raw(format!("{} ➜ ", display_val)));
            spans.push(norm.clone().yellow().bold());
        } else {
            spans.push(Span::raw(display_val));
        }
        val_items.push(ListItem::new(Line::from(spans)));
    }

    let val_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" 2. Unique Values for '{}' ", col_name))
        .border_style(
            Style::default().fg(if app.ui.active_pane == ActivePane::Values {
                Color::Yellow
            } else {
                Color::White
            }),
        );

    f.render_stateful_widget(
        List::new(val_items)
            .block(val_block)
            .highlight_style(Style::default().bg(Color::Indexed(237))),
        main_chunks[1],
        &mut app.ui.val_list_state,
    );

    let info_items: Vec<ListItem> = app
        .ui
        .info_results
        .iter()
        .map(|s| ListItem::new(s.as_str()))
        .collect();

    let key_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" 3. Source Key [{}] ", active_key_name))
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_stateful_widget(
        List::new(info_items)
            .block(key_block)
            .highlight_style(Style::default().bg(Color::Indexed(237))),
        main_chunks[2],
        &mut app.ui.val_list_state.clone(),
    );

    let selection_count = app
        .ui
        .multi_selected
        .iter()
        .filter(|(c, _)| *c == col_idx)
        .count();
    let sel_tag = if selection_count > 0 {
        format!(" [{} SELECTED] ", selection_count)
            .on_cyan()
            .black()
            .bold()
    } else {
        Span::raw("")
    };

    let footer = Line::from(vec![
        Span::styled(format!(" Status: {} ", app.status_msg), Style::default().bg(Color::Blue).fg(Color::White)),
        sel_tag,
        Span::raw(" | TAB: Switch | /: Search | SPACE: Select | ENTER: Map | DEL: Undo | K: Set Key | S: Save | ESC: Dashboard "),
    ]);
    f.render_widget(Paragraph::new(footer), chunks[1]);

    if app.ui.input_mode == InputMode::Searching || !app.ui.search_buffer.is_empty() {
        let area = Rect::new(
            main_chunks[1].x + 2,
            main_chunks[1].y + 1,
            main_chunks[1].width - 4,
            3,
        );
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Search Values ")
            .border_style(
                Style::default().fg(if app.ui.input_mode == InputMode::Searching {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            );
        let p = Paragraph::new(format!(" /{}", app.ui.search_buffer)).block(block);
        f.render_widget(p, area);
    }

    if app.ui.input_mode == InputMode::Editing {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Normalize To: ")
            .border_style(Style::default().fg(Color::Green));
        let p = Paragraph::new(format!("\n  {}", app.ui.input_buffer)).block(block);
        f.render_widget(p, area);
        f.set_cursor(
            area.x + 3 + app.ui.input_buffer.chars().count() as u16,
            area.y + 2,
        );
    }

    if app.ui.input_mode == InputMode::SelectingKey {
        let area = centered_rect(40, 60, f.size());
        f.render_widget(Clear, area);
        let items: Vec<ListItem> = app
            .db
            .headers
            .iter()
            .map(|h| ListItem::new(h.as_str()))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Select Registry Key Column ")
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .highlight_style(Style::default().bg(Color::Magenta).fg(Color::White));
        f.render_stateful_widget(list, area, &mut app.ui.key_list_state);
    }
}
