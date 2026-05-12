# normie-rs 🧹

A blazing-fast, terminal-based CSV normalization tool. `normie-rs` helps you quickly clean up messy CSV data by mapping unique column values to normalized targets, with a focus on speed and keyboard-driven efficiency.

![Splash Screen Placeholder](https://via.placeholder.com/800x400?text=normie-rs+Dashboard)

## Features

- **Dashboard & Splash Screen**: Quick access to recent files or start a new session.
- **TUI File Browser**: Navigate your local file system and pick CSVs without leaving the terminal.
- **Auto-Collate Recent Files**: Remembers your last 10 opened files for instant access.
- **Intelligent Normalization**: 
  - View unique values per column.
  - Search and filter values.
  - Batch map selected values to a single target.
  - Side-by-side view of source keys (IDs) to help you identify values in context.
- **Persistent Mappings**: Saves your mapping progress to a `.normie.json` file next to your CSV, so you can resume later.
- **Blazing Fast**: Built in Rust with `ratatui` for a snappy, low-latency experience.

## Installation

### From Source

Ensure you have [Rust](https://rustup.rs/) and `cargo` installed.

```bash
git clone https://github.com/GoldTone/normie-rs.git
cd normie-rs
cargo install --path .
```

## Usage

Simply run the executable:

```bash
normie-rs
```

### Keyboard Shortcuts

#### Dashboard
- `UP/DOWN`: Navigate menu.
- `ENTER`: Select option.
- `Q / ESC`: Quit.

#### File Browser
- `UP/DOWN`: Navigate files.
- `ENTER`: Open CSV or enter Directory.
- `BACKSPACE`: Go to parent directory.
- `ESC`: Return to Dashboard.

#### Normalizer
- `TAB`: Switch between **Columns** and **Unique Values** panes.
- `UP/DOWN`: Navigate the active list.
- `SPACE`: Multi-select values for batch mapping.
- `ENTER`: Map selected/highlighted value(s) to a new string.
- `/`: Search/Filter unique values.
- `K`: Set the **Source Key** column (used for the context pane).
- `S`: Save normalized CSV.
- `DELETE / BACKSPACE`: Remove a mapping.
- `ESC`: Return to Dashboard.

## Output

When you save your progress, `normie-rs` creates:
1. `{filename}_normalized.csv`: The final cleaned data.
2. `{filename}.normie.json`: A mapping file that allows you to reload your progress in future sessions.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
