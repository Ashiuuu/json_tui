mod node;

use crate::node::Tree;

use color_eyre::{Result, eyre::eyre};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use serde_json::Value;

fn retrieve_content() -> Result<(String, String)> {
    let args: Vec<String> = std::env::args().collect();

    let (title, data) = match args.len() {
        1 => {
            let data = std::io::read_to_string(std::io::stdin())?;
            ("stdin".to_string(), data)
        }
        2 => {
            let data = std::fs::read_to_string(&args[1])?;
            (args[1].to_string(), data)
        }
        _ => {
            println!("More than 1 arg not supported");
            return Err(eyre!("More than 1 arg not supported"));
        }
    };

    Ok((title, data))
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let (title, content) = retrieve_content()?;

    let terminal = ratatui::init();
    let result = run(terminal, title, content);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, title: String, content: String) -> Result<()> {
    let content: Value = serde_json::from_str(&content)?;
    let mut tree = Tree::from_value(content);

    let scroll_x = 0;
    let mut scroll_y: u16 = 0;

    let mut up_clamp = 0;
    let mut bot_clamp = 0;
    let mut scroll_y_max = 0;
    let mut total_height = 0;

    loop {
        let current_line = tree.find_current_line();

        let number_of_lines = {
            let content_text = tree.to_text();
            content_text.lines.len()
        };

        terminal.draw(|frame| {
            let (title_area, layout) = calculate_layout(frame.area());

            (up_clamp, bot_clamp) = {
                total_height = (layout.height as usize) - 2; // account for borders
                let first_third = total_height / 3;
                let second_third = first_third * 2;
                let scroll_y = scroll_y as usize;
                (first_third + scroll_y, second_third + scroll_y)
            };

            scroll_y_max = number_of_lines.saturating_sub(total_height) as u16;

            if scroll_y > scroll_y_max {
                scroll_y = scroll_y_max;
            }

            render_title(frame, title_area, &title);

            let text_content = tree.to_text();

            let paragraph = Paragraph::new(text_content.clone())
                .wrap(Wrap { trim: false })
                .scroll((scroll_y, scroll_x))
                .block(Block::new().borders(Borders::ALL));

            frame.render_widget(paragraph, layout);
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => {
                    break Ok(());
                }
                KeyCode::Char('h') => {
                    tree.toggle_current_node_highlight();
                }
                KeyCode::Up => {
                    tree.next_node_up();

                    if current_line < up_clamp {
                        let diff = up_clamp.saturating_sub(current_line) as u16;

                        scroll_y = scroll_y.saturating_sub(diff);
                    }
                }
                KeyCode::Down => {
                    tree.next_node_down();

                    if current_line > bot_clamp {
                        let diff = current_line.saturating_sub(bot_clamp) as u16;
                        scroll_y += diff;

                        if scroll_y > scroll_y_max {
                            scroll_y = scroll_y_max;
                        }
                    }
                }
                KeyCode::Enter => {
                    tree.toggle_current_node_visibility();
                }
                _ => (),
            }
        }
    }
}

fn calculate_layout(area: Rect) -> (Rect, Rect) {
    let main_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]);
    let [title_area, main_area] = main_layout.areas(area);
    (title_area, main_area)
}

fn render_title(frame: &mut Frame, area: Rect, title: &str) {
    frame.render_widget(
        Paragraph::new(title)
            .dark_gray()
            .alignment(Alignment::Center),
        area,
    );
}
