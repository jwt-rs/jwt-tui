use ratatui::{
  layout::{Constraint, Rect},
  widgets::{Row, Table},
  Frame,
};

use super::{
  utils::{
    layout_block_with_line, style_highlight, style_primary, style_secondary, title_with_dual_style,
    vertical_chunks,
  },
  HIGHLIGHT,
};
use crate::app::App;

pub fn draw_help(f: &mut Frame<'_>, app: &mut App, area: Rect) {
  let chunks = vertical_chunks(vec![Constraint::Percentage(100)], area);

  // Create a one-column table to avoid flickering due to non-determinism when
  // resolving constraints on widths of table columns.
  let format_row =
    |r: &Vec<String>| -> Vec<String> { vec![format!("{:50}{:50}{:20}", r[0], r[1], r[2])] };

  let header = ["Key", "Action", "Context"];
  let header = format_row(&header.iter().map(|s| s.to_string()).collect());

  let help_docs = app
    .help_docs
    .items
    .iter()
    .map(format_row)
    .collect::<Vec<Vec<String>>>();
  let help_docs = &help_docs[0_usize..];

  let rows = help_docs
    .iter()
    .map(|item| Row::new(item.clone()).style(style_primary(app.light_theme)));

  let title = title_with_dual_style(" Help ".into(), "| close <esc> ".into());

  let help_menu = Table::new(rows, [Constraint::Percentage(100)])
    .header(
      Row::new(header)
        .style(style_secondary(app.light_theme))
        .bottom_margin(0),
    )
    .block(layout_block_with_line(title, app.light_theme, true))
    .row_highlight_style(style_highlight())
    .highlight_symbol(HIGHLIGHT);
  f.render_stateful_widget(help_menu, chunks[0], &mut app.help_docs.state);
}

#[cfg(test)]
mod tests {
  use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Position,
    style::{Modifier, Style},
    Terminal,
  };

  use super::*;
  use crate::ui::utils::{COLOR_CYAN, COLOR_WHITE, COLOR_YELLOW};

  #[test]
  fn test_draw_help() {
    let backend = TestBackend::new(110, 7);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
      .draw(|f| {
        let size = f.area();
        let mut app = App::default();
        draw_help(f, &mut app, size);
      })
      .unwrap();

    let mut expected = Buffer::with_lines(vec![
        "┌ Help | close <esc> ────────────────────────────────────────────────────────────────────────────────────────┐",
        "│   Key                                               Action                                            Conte│",
        "│=> <Ctrl+c> | <q>                                    Quit                                              Gener│",
        "│   <Esc>                                             Close child page/Go back/Stop editing             Gener│",
        "│   <?>                                               Help page                                         Gener│",
        "│   <Ctrl+r>                                          Refresh UI                                        Gener│",
        "└────────────────────────────────────────────────────────────────────────────────────────────────────────────┘",
      ]);
    // set row styles
    // First row heading style
    for col in 0..=109 {
      match col {
        0 | 7..=109 => {
          expected
            .cell_mut(Position::new(col, 0))
            .unwrap()
            .set_style(Style::default().fg(COLOR_YELLOW));
        }
        1..=6 => {
          expected.cell_mut(Position::new(col, 0)).unwrap().set_style(
            Style::default()
              .fg(COLOR_YELLOW)
              .add_modifier(Modifier::BOLD),
          );
        }
        _ => {
          expected.cell_mut(Position::new(col, 0)).unwrap().set_style(
            Style::default()
              .fg(COLOR_WHITE)
              .add_modifier(Modifier::BOLD),
          );
        }
      }
    }

    // second row table headings
    for col in 0..=109 {
      expected
        .cell_mut(Position::new(col, 1))
        .unwrap()
        .set_style(Style::default().fg(COLOR_YELLOW));
    }

    // first table data row style
    for col in 0..=109 {
      match col {
        1..=108 => {
          expected.cell_mut(Position::new(col, 2)).unwrap().set_style(
            Style::default()
              .fg(COLOR_CYAN)
              .add_modifier(Modifier::REVERSED),
          );
        }
        _ => {
          expected
            .cell_mut(Position::new(col, 2))
            .unwrap()
            .set_style(Style::default().fg(COLOR_YELLOW));
        }
      }
    }

    // rows
    for row in 3..=5 {
      for col in 0..=109 {
        match col {
          1..=108 => {
            expected
              .cell_mut(Position::new(col, row))
              .unwrap()
              .set_style(Style::default().fg(COLOR_CYAN));
          }
          _ => {
            expected
              .cell_mut(Position::new(col, row))
              .unwrap()
              .set_style(Style::default().fg(COLOR_YELLOW));
          }
        }
      }
    }

    // last row
    for col in 0..=109 {
      expected
        .cell_mut(Position::new(col, 6))
        .unwrap()
        .set_style(Style::default().fg(COLOR_YELLOW));
    }

    terminal.backend().assert_buffer(&expected);
  }
}
