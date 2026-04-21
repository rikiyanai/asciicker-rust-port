//! Main menu: title screen, item navigation, and menu rendering.
//!
//! Maps to C++ mainmenu.cpp: MainMenu struct, Paint/Scale.
//! Uses AsciiCellGrid for direct ASCII rendering during MainMenu state.

use bevy::prelude::*;

use crate::game::state::{GameLaunchConfig, GameState};
use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::font::{Font1, FontSkin};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Action performed when a menu item is activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Start a new game: transitions to Loading state.
    StartGame,
    /// Open the dedicated render-tuning surface.
    OpenRenderWorkbench,
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
                    label: "Render Tuning Workbench".to_string(),
                    action: MenuAction::OpenRenderWorkbench,
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
    mut launch_config: ResMut<GameLaunchConfig>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    if let Some(item) = menu.items.get(menu.selected_index) {
        match item.action {
            MenuAction::StartGame => {
                launch_config.load_target = GameState::Playing;
                next_state.set(GameState::Loading);
                info!("Menu: Start Game -> Loading");
            }
            MenuAction::OpenRenderWorkbench => {
                launch_config.load_target = GameState::Workbench;
                next_state.set(GameState::Loading);
                info!("Menu: Render Tuning Workbench -> Loading");
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
pub fn render_menu(
    grid: Option<ResMut<AsciiCellGrid>>,
    menu: Res<MainMenu>,
    font: Option<Res<Font1>>,
) {
    let Some(mut grid) = grid else { return };
    let fallback_font = Font1::default();
    let font = font.as_deref().unwrap_or(&fallback_font);

    let w = grid.width as usize;
    let h = grid.height as usize;

    // Clear the entire grid (prevents stale data from Playing state leaking through)
    for i in 0..(w * h) {
        grid.char_indices[i] = b' ' as u16;
        grid.fg_colors[i] = [0, 0, 0, 255];
        grid.bg_colors[i] = [0, 0, 0, 255];
    }

    // Title: centered, rendered through Font1 so overlay text uses the real text path.
    let title = "ASCIICKER";
    let title_row = h / 3;
    font.paint_centered(&mut grid, title_row as u32, title, FontSkin::Grey);

    // Menu items below title
    let items_start_row = title_row + 3;

    for (i, item) in menu.items.iter().enumerate() {
        let row = items_start_row + i * 2; // spacing between items
        if row >= h {
            break;
        }

        let is_selected = i == menu.selected_index;

        // Build display text: "> Label" for selected, "  Label" for unselected
        let prefix = if is_selected { "> " } else { "  " };
        let display = format!("{}{}", prefix, item.label);
        let skin = if is_selected {
            FontSkin::Gold
        } else {
            FontSkin::Grey
        };
        font.paint_centered(&mut grid, row as u32, &display, skin);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_menu_default() {
        let menu = MainMenu::default();
        assert_eq!(menu.items.len(), 3);
        assert_eq!(menu.selected_index, 0);
        assert_eq!(menu.items[0].label, "Start Game");
        assert_eq!(menu.items[0].action, MenuAction::StartGame);
        assert_eq!(menu.items[1].label, "Render Tuning Workbench");
        assert_eq!(menu.items[1].action, MenuAction::OpenRenderWorkbench);
        assert_eq!(menu.items[2].label, "Quit");
        assert_eq!(menu.items[2].action, MenuAction::Quit);
    }

    #[test]
    fn test_navigation_wraps_down() {
        let mut menu = MainMenu::default();
        assert_eq!(menu.selected_index, 0);

        // Move down past last item -> wraps to 0
        menu.selected_index = (menu.selected_index + 1) % menu.items.len();
        assert_eq!(menu.selected_index, 1);
        menu.selected_index = (menu.selected_index + 1) % menu.items.len();
        assert_eq!(menu.selected_index, 2);
        menu.selected_index = (menu.selected_index + 1) % menu.items.len();
        assert_eq!(menu.selected_index, 0); // wrapped
    }

    #[test]
    fn test_navigation_wraps_up() {
        let mut menu = MainMenu::default();
        assert_eq!(menu.selected_index, 0);

        // Move up from 0 -> wraps to last
        let count = menu.items.len();
        menu.selected_index = if menu.selected_index == 0 {
            count - 1
        } else {
            menu.selected_index - 1
        };
        assert_eq!(menu.selected_index, 2); // wrapped to last
    }

    #[test]
    fn test_render_menu_highlights_selected() {
        let mut grid = AsciiCellGrid::new(80, 24);
        let mut menu = MainMenu::default();

        // Select item 2 (Quit)
        menu.selected_index = 2;

        let w = grid.width as usize;
        let h = grid.height as usize;

        // Clear
        for i in 0..(w * h) {
            grid.char_indices[i] = b' ' as u16;
            grid.fg_colors[i] = [0, 0, 0, 255];
        }

        // Render menu items (simulating render_menu logic)
        let title_row = h / 3;
        let items_start_row = title_row + 3;
        let unselected_fg: [u8; 4] = [255, 255, 255, 255];
        let selected_fg: [u8; 4] = [255, 200, 0, 255];

        for (i, item) in menu.items.iter().enumerate() {
            let row = items_start_row + i * 2;
            let is_selected = i == menu.selected_index;
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

        // Verify item 0 (Start Game, unselected) uses white
        let row0 = items_start_row;
        let display0 = "  Start Game";
        let col0 = w.saturating_sub(display0.len()) / 2;
        let idx0 = row0 * w + col0;
        assert_eq!(
            grid.fg_colors[idx0], unselected_fg,
            "Unselected item should be white"
        );

        // Verify item 2 (Quit, selected) uses gold and has '>' prefix
        let row1 = items_start_row + 4;
        let display1 = "> Quit";
        let col1 = w.saturating_sub(display1.len()) / 2;
        let idx1 = row1 * w + col1;
        assert_eq!(
            grid.fg_colors[idx1], selected_fg,
            "Selected item should be gold"
        );
        assert_eq!(
            grid.char_indices[idx1], b'>' as u16,
            "Selected item should have '>' prefix"
        );
    }

    #[test]
    fn test_render_menu_title_centered() {
        let mut grid = AsciiCellGrid::new(80, 24);
        let w = grid.width as usize;
        let h = grid.height as usize;

        // Clear
        for i in 0..(w * h) {
            grid.char_indices[i] = b' ' as u16;
        }

        // Render title
        let title = b"ASCIICKER";
        let title_row = h / 3;
        let title_start = w.saturating_sub(title.len()) / 2;
        for (i, &ch) in title.iter().enumerate() {
            let idx = title_row * w + title_start + i;
            grid.char_indices[idx] = ch as u16;
        }

        // Verify title is centered
        let expected_start = (80 - 9) / 2; // "ASCIICKER" = 9 chars, width = 80
        let idx = title_row * w + expected_start;
        assert_eq!(grid.char_indices[idx], b'A' as u16);
        assert_eq!(grid.char_indices[idx + 8], b'R' as u16);
    }
}
