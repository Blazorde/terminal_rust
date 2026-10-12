use std::error::Error;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Focus {
    #[default]
    Terminal,
    Project,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::Terminal => Self::Project,
            Self::Project => Self::Terminal,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

impl SplitDirection {
    fn toggle(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    fn ratatui_direction(self) -> Direction {
        match self {
            Self::Horizontal => Direction::Horizontal,
            Self::Vertical => Direction::Vertical,
        }
    }
}

#[derive(Debug, Default)]
struct App {
    focus: Focus,
    should_quit: bool,
    split: SplitDirection,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> AppResult<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event(event::read()?);
        }
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else {
            return;
        };
        if key.kind != KeyEventKind::Press {
            return;
        }
        self.handle_key(key);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('q') | KeyCode::Esc, _) => self.should_quit = true,
            (KeyCode::Tab, _) => self.focus = self.focus.next(),
            (KeyCode::BackTab, _) => self.focus = self.focus.next(),
            (KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down, _) => {
                self.focus = self.focus.next();
            }
            (KeyCode::Char('s'), _) => self.split = self.split.toggle(),
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let [body, footer] = vertical_areas(frame.area());
        let [terminal, project] = pane_areas(body, self.split);

        frame.render_widget(
            pane(
                " Terminal ",
                self.focus == Focus::Terminal,
                vec![
                    Line::from(vec![
                        Span::styled("terminal-rust", Style::default().fg(Color::Cyan)),
                        Span::raw("  develop"),
                    ]),
                    Line::raw("$ cargo test --all-targets --all-features"),
                    Line::styled("125 tests passed", Style::default().fg(Color::Green)),
                    Line::raw("$ _"),
                ],
            ),
            terminal,
        );
        frame.render_widget(
            pane(
                " Project & logs ",
                self.focus == Focus::Project,
                vec![
                    Line::styled("FILES", Style::default().add_modifier(Modifier::BOLD)),
                    Line::raw("  src/"),
                    Line::raw("  desktop/"),
                    Line::raw("  docs/"),
                    Line::raw(""),
                    Line::styled("LOGS", Style::default().add_modifier(Modifier::BOLD)),
                    Line::raw("  Git repository detected"),
                    Line::raw("  Working tree clean"),
                ],
            ),
            project,
        );

        let help = format!(
            " Tab/Shift+Tab/arrows: focus  •  s: split {:?}  •  q/Esc/Ctrl+C: quit ",
            self.split
        );
        frame.render_widget(
            Paragraph::new(help)
                .style(Style::default().fg(Color::DarkGray))
                .centered(),
            footer,
        );
    }
}

fn pane<'a>(title: &'a str, focused: bool, lines: Vec<Line<'a>>) -> Paragraph<'a> {
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .wrap(Wrap { trim: true })
}

fn vertical_areas(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area)
}

fn pane_areas(area: Rect, split: SplitDirection) -> [Rect; 2] {
    Layout::default()
        .direction(split.ratatui_direction())
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .areas(area)
}

fn main() -> AppResult<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_cycles_between_both_panes() {
        assert_eq!(Focus::Terminal.next(), Focus::Project);
        assert_eq!(Focus::Project.next(), Focus::Terminal);
    }

    #[test]
    fn split_direction_changes_layout_axis() {
        let area = Rect::new(0, 0, 100, 40);
        let horizontal = pane_areas(area, SplitDirection::Horizontal);
        let vertical = pane_areas(area, SplitDirection::Vertical);

        assert_eq!(horizontal[0].height, horizontal[1].height);
        assert!(horizontal[0].width > horizontal[1].width);
        assert_eq!(vertical[0].width, vertical[1].width);
        assert!(vertical[0].height > vertical[1].height);
    }

    #[test]
    fn keyboard_events_update_state() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, Focus::Project);

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.split, SplitDirection::Vertical);

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.should_quit);
    }
}
