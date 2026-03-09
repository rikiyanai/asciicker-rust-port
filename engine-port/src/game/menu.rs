//! Main menu: title screen, item navigation, and menu rendering.
//!
//! Maps to C++ mainmenu.cpp: MainMenu struct, Paint/Scale.
//! Uses AsciiCellGrid for direct ASCII rendering during MainMenu state.

use bevy::prelude::*;

use crate::game::state::GameState;
use crate::output::ascii_cell_grid::AsciiCellGrid;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Action performed when a menu item is activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Start a new game: transitions to Loading state.
    StartGame,
    /// Exit the application.
    Quit,
}

/// A single menu item with a display label and associated action.
#[derive(Clone, Debug)]
pub struct MenuItem {
    /// Text label displayed in the menu.
    pub label: String,
    /// Action performed when this item is activated.
    pub action: MenuAction,
}

/// Main menu resource: items list and current selection.
///
/// Default: two items ("Start Game", "Quit"), selected_index = 0.
#[derive(Resource, Debug)]
pub struct MainMenu {
    /// Ordered list of menu items.
    pub items: Vec<MenuItem>,
    /// Index of the currently selected item.
    pub selected_index: usize,
}

impl Default for MainMenu {
    fn default() -> Self {
        Self {
            items: vec![
                MenuItem {
                    label: "Start Game".to_string(),
                    action: MenuAction::StartGame,
                },
                MenuItem {
                    label: "Quit".to_string(),
                    action: MenuAction::Quit,
                },
            ],
            selected_index: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Navigate menu items with Up/Down arrows. Wraps around.
pub fn menu_navigation(keys: Res<ButtonInput<KeyCode>>, mut menu: ResMut<MainMenu>) {
    let count = menu.items.len();
    if count == 0 {
        return;
    }

    if keys.just_pressed(KeyCode::ArrowUp) {
        menu.selected_index = if menu.selected_index == 0 {
            count - 1
        } else {
            menu.selected_index - 1
        };
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        menu.selected_index = (menu.selected_index + 1) % count;
    }
}

/// Activate selected menu item on Enter key.
///
/// P7-103 FIX: AppExit uses MessageWriter<AppExit> in Bevy 0.18
/// (EventWriter<AppExit> was removed).
pub fn menu_activate(
    keys: Res<ButtonInput<KeyCode>>,
    menu: Res<MainMenu>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    if let Some(item) = menu.items.get(menu.selected_index) {
        match item.action {
            MenuAction::StartGame => {
                next_state.set(GameState::Loading);
                info!("Menu: Start Game -> Loading");
            }
            MenuAction::Quit => {
                // AppExit requires MessageWriter<AppExit> in Bevy 0.18 -- NOT EventWriter<AppExit>.
                app_exit.write(AppExit::Success);
                info!("Menu: Quit -> AppExit");
            }
        }
    }
}

/// Render menu to AsciiCellGrid.
///
/// R19-007 FIX: Selection indicator via `>` prefix and gold color.
/// P7-046 FIX: Uses Option<ResMut<AsciiCellGrid>> for test environments.
/// P7-064 FIX: Import from crate::output::ascii_cell_grid::AsciiCellGrid.
pub fn render_menu(grid: Option<ResMut<AsciiCellGrid>>, menu: Res<MainMenu>) {
    let Some(mut grid) = grid else { return };

    let w = grid.width as usize;
    let h = grid.height as usize;

    // Clear the entire grid (prevents stale data from Playing state leaking through)
    for i in 0..(w * h) {
        grid.char_indices[i] = b' ' as u16;
        grid.fg_colors[i] = [0, 0, 0, 255];
        grid.bg_colors[i] = [0, 0, 0, 255];
    }

    // Title: "ASCIICKER" centered, bright cyan
    let title = b"ASCIICKER";
    let title_fg: [u8; 4] = [0, 255, 255, 255]; // bright cyan
    let title_row = h / 3;
    let title_start = w.saturating_sub(title.len()) / 2;
    for (i, &ch) in title.iter().enumerate() {
        let idx = title_row * w + title_start + i;
        if idx < w * h {
            grid.char_indices[idx] = ch as u16;
            grid.fg_colors[idx] = title_fg;
        }
    }

    // Menu items below title
    let items_start_row = title_row + 3;
    let unselected_fg: [u8; 4] = [255, 255, 255, 255]; // white
    let selected_fg: [u8; 4] = [255, 200, 0, 255]; // gold

    for (i, item) in menu.items.iter().enumerate() {
        let row = items_start_row + i * 2; // spacing between items
        if row >= h {
            break;
        }

        let is_selected = i == menu.selected_index;

        // Build display text: "> Label" for selected, "  Label" for unselected
        let prefix = if is_selected { "> " } else { "  " };
        let display = format!("{}{}", prefix, item.label);
        let fg = if is_selected {
            selected_fg
        } else {
            unselected_fg
        };

        let col_start = w.saturating_sub(display.len()) / 2;
        for (j, ch) in display.bytes().enumerate() {
            let idx = row * w + col_start + j;
            if idx < w * h {
                grid.char_indices[idx] = ch as u16;
                grid.fg_colors[idx] = fg;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
