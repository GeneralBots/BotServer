use super::ui_panels::ConsolePalette;
use super::XtreeUI;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

impl XtreeUI {
    pub(super) fn render(&mut self, f: &mut Frame, cursor_blink: bool) {
        let palette = ConsolePalette {
            bg: Color::Rgb(0, 30, 100),
            text: Color::Rgb(255, 255, 255),
            border_focused: Color::Rgb(85, 255, 255),
            border_dim: Color::Rgb(170, 170, 170),
            highlight: Color::Rgb(0, 170, 170),
            header_bg_color: Color::Rgb(170, 170, 170),
            header_text_color: Color::Rgb(0, 0, 0),
        };
        let ConsolePalette {
            bg,
            border_focused,
            border_dim,
            text,
            header_bg_color,
            header_text_color,
            ..
        } = palette;
        if self.app_state.is_none() {
            self.render_loading(
                f,
                bg,
                text,
                border_focused,
                header_bg_color,
                header_text_color,
            );
            return;
        }
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(12),
            ])
            .split(f.area());
        self.render_header(f, main_chunks[0], bg, header_bg_color, header_text_color);

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(40),
                Constraint::Percentage(35),
            ])
            .split(main_chunks[1]);

        self.render_file_tree(f, content_chunks[0], palette);

        if self.editor.is_some() {
            if let Some(editor) = &mut self.editor {
                let area = content_chunks[1];
                editor.set_visible_lines(area.height.saturating_sub(4) as usize);
                let is_active = self.active_panel == super::ActivePanel::Editor;
                let border_color = if is_active {
                    border_focused
                } else {
                    border_dim
                };
                let title_style = if is_active {
                    Style::default()
                        .fg(header_text_color)
                        .bg(header_bg_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(header_text_color).bg(header_bg_color)
                };
                let title_text = format!(" EDITOR: {} ", editor.file_path());
                let block = Block::default()
                    .title(Span::styled(title_text, title_style))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(bg));
                let content = editor.render(cursor_blink);
                let paragraph = Paragraph::new(content)
                    .block(block)
                    .style(Style::default().fg(text))
                    .wrap(Wrap { trim: false });
                f.render_widget(paragraph, area);
            }
            self.render_chat(f, content_chunks[2], palette);
        } else {
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(content_chunks[1]);
            self.render_status(f, right_chunks[0], palette);
            self.render_status(f, content_chunks[2], palette);
        }
        self.render_logs(f, main_chunks[2], palette);
    }

    fn render_header(
        &self,
        f: &mut Frame,
        area: Rect,
        _bg: Color,
        header_bg_color: Color,
        header_text_color: Color,
    ) {
        let block = Block::default().style(Style::default().bg(header_bg_color));
        f.render_widget(block, area);
        let title = if self.app_state.is_some() {
            let components = [
                ("Tables", "postgres", "5432"),
                ("Cache", "valkey-server", "6379"),
                ("Drive", "minio", "9000"),
                ("LLM", "llama-server", "8081"),
            ];
            let statuses: Vec<String> = components
                .iter()
                .map(|(comp_name, process, _port)| {
                    let running = super::status_panel::StatusPanel::check_component_running(process);
                    let icon = if running { "●" } else { "○" };
                    format!("{icon} {comp_name}")
                })
                .collect();
            format!(" GENERAL BOTS ┃ {} ", statuses.join(" ┃ "))
        } else {
            " GENERAL BOTS ".to_string()
        };
        let title_len = title.len() as u16;
        let centered_x = (area.width.saturating_sub(title_len)) / 2;
        let centered_y = area.y + 1;
        let x = area.x + centered_x;
        let max_width = area.width.saturating_sub(x - area.x);
        let width = title_len.min(max_width);
        let title_span = Span::styled(
            title,
            Style::default()
                .fg(header_text_color)
                .bg(header_bg_color)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(
            Paragraph::new(Line::from(title_span)),
            Rect {
                x,
                y: centered_y,
                width,
                height: 1,
            },
        );
    }

    fn render_loading(
        &self,
        f: &mut Frame,
        bg: Color,
        text: Color,
        border: Color,
        header_bg_color: Color,
        header_text_color: Color,
    ) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(12),
            ])
            .split(f.area());

        let header_block = Block::default().style(Style::default().bg(header_bg_color));
        f.render_widget(header_block, main_chunks[0]);

        let title = " GENERAL BOTS ";
        let title_len = title.len() as u16;
        let centered_x = (main_chunks[0].width.saturating_sub(title_len)) / 2;
        let title_span = Span::styled(
            title,
            Style::default()
                .fg(header_text_color)
                .bg(header_bg_color)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(
            Paragraph::new(Line::from(title_span)),
            Rect {
                x: main_chunks[0].x + centered_x,
                y: main_chunks[0].y + 1,
                width: title_len,
                height: 1,
            },
        );

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(40),
                Constraint::Percentage(35),
            ])
            .split(main_chunks[1]);

        let file_block = Block::default()
            .title(Span::styled(
                " FILE EXPLORER ",
                Style::default().fg(header_text_color).bg(header_bg_color),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(bg));
        let file_text = Paragraph::new("\n\n     Loading files...")
            .block(file_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(file_text, content_chunks[0]);

        let middle_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(content_chunks[1]);

        let status_block = Block::default()
            .title(Span::styled(
                " STATUS ",
                Style::default().fg(header_text_color).bg(header_bg_color),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(bg));

        let status_text = format!(
            "\n   {}\n\n  Components:\n    ○ Vault\n    ○ Database\n    ○ Drive\n    ○ Cache\n    ○ LLM",
            self.bootstrap_status
        );
        let status_para = Paragraph::new(status_text)
            .block(status_block)
            .style(Style::default().fg(text));
        f.render_widget(status_para, middle_chunks[0]);

        let empty_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(bg));
        f.render_widget(empty_block, middle_chunks[1]);

        let chat_block = Block::default()
            .title(Span::styled(
                " CHAT ",
                Style::default().fg(header_text_color).bg(header_bg_color),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(bg));
        let chat_text = Paragraph::new("\n\n     Connecting...")
            .block(chat_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(chat_text, content_chunks[2]);

        let logs_block = Block::default()
            .title(Span::styled(
                " SYSTEM LOGS ",
                Style::default().fg(header_text_color).bg(header_bg_color),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(bg));

        let logs_visible_lines = main_chunks[2].height.saturating_sub(2) as usize;
        let logs_content = {
            if let Ok(panel) = self.log_panel.lock() {
                panel.render(logs_visible_lines)
            } else {
                String::from("  Waiting for logs...")
            }
        };

        let logs_para = Paragraph::new(logs_content)
            .block(logs_block)
            .style(Style::default().fg(text))
            .wrap(Wrap { trim: false });
        f.render_widget(logs_para, main_chunks[2]);
    }
}
