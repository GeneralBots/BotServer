use super::ActivePanel;
use super::XtreeUI;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// The colors every console panel renders with.
///
/// The set travels as one value so a panel signature receives the palette
/// instead of seven separate colors.
#[derive(Clone, Copy)]
pub(super) struct ConsolePalette {
    pub(super) bg: Color,
    pub(super) text: Color,
    pub(super) border_focused: Color,
    pub(super) border_dim: Color,
    pub(super) highlight: Color,
    pub(super) header_bg_color: Color,
    pub(super) header_text_color: Color,
}

impl XtreeUI {
    pub(super) fn render_file_tree(&self, f: &mut Frame, area: Rect, palette: ConsolePalette) {
        let ConsolePalette {
            bg,
            text,
            border_focused,
            border_dim,
            highlight,
            header_bg_color,
            header_text_color,
        } = palette;
        if let Some(file_tree) = &self.file_tree {
            let items = file_tree.render_items();
            let selected = file_tree.selected_index();

            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(idx, (display, _))| {
                    let style = if idx == selected {
                        Style::default()
                            .bg(highlight)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(text)
                    };
                    ListItem::new(Line::from(Span::styled(display.clone(), style)))
                })
                .collect();
            let is_active = self.active_panel == ActivePanel::FileTree;
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
            let block = Block::default()
                .title(Span::styled(" FILE EXPLORER ", title_style))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg));
            let list = List::new(list_items).block(block);
            f.render_widget(list, area);
        }
    }

    pub(super) fn render_status(&mut self, f: &mut Frame, area: Rect, palette: ConsolePalette) {
        let ConsolePalette {
            bg,
            text,
            border_focused,
            border_dim,
            header_bg_color,
            header_text_color,
            ..
        } = palette;
        let selected_bot_opt = self
            .file_tree
            .as_ref()
            .and_then(|ft| ft.get_selected_bot());
        let status_text = if let Some(status_panel) = &mut self.status_panel {
            match selected_bot_opt {
                Some(bot) => status_panel.render(Some(bot)),
                None => status_panel.render(None),
            }
        } else {
            "Waiting for initialization...".to_string()
        };
        let is_active = self.active_panel == ActivePanel::Status;
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
        let block = Block::default()
            .title(Span::styled(" SYSTEM STATUS ", title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg));
        let paragraph = Paragraph::new(status_text)
            .block(block)
            .style(Style::default().fg(text))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }

    pub(super) fn render_chat(&self, f: &mut Frame, area: Rect, palette: ConsolePalette) {
        let ConsolePalette {
            bg,
            text,
            border_focused,
            border_dim,
            header_bg_color,
            header_text_color,
            ..
        } = palette;
        if let Some(chat_panel) = &self.chat_panel {
            let is_active = self.active_panel == ActivePanel::Chat;
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
            let selected_bot = if let Some(file_tree) = &self.file_tree {
                file_tree
                    .get_selected_bot()
                    .unwrap_or_else(|| "No bot selected".to_string())
            } else {
                "No bot selected".to_string()
            };
            let title_text = format!(" CHAT: {selected_bot} ");
            let block = Block::default()
                .title(Span::styled(title_text, title_style))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg));
            let content = chat_panel.render();
            let paragraph = Paragraph::new(content)
                .block(block)
                .style(Style::default().fg(text))
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
        }
    }

    pub(super) fn render_logs(&self, f: &mut Frame, area: Rect, palette: ConsolePalette) {
        let ConsolePalette {
            bg,
            text,
            border_focused,
            border_dim,
            header_bg_color,
            header_text_color,
            ..
        } = palette;
        let visible_lines = area.height.saturating_sub(2) as usize;

        let (log_lines, can_scroll_up, can_scroll_down, logs_count, auto_scroll) = {
            let log_panel = self.log_panel.try_lock();
            if let Ok(panel) = log_panel {
                let (content, up, down) = panel.render_with_scroll_indicator(visible_lines);
                (
                    content,
                    up,
                    down,
                    panel.logs_count(),
                    panel.is_auto_scroll(),
                )
            } else {
                ("Loading logs...".to_string(), false, false, 0, true)
            }
        };

        let is_active = self.active_panel == ActivePanel::Logs;
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

        let scroll_indicator = if can_scroll_up && can_scroll_down {
            " [^v] "
        } else if can_scroll_up {
            " [^] "
        } else if can_scroll_down {
            " [v] "
        } else {
            ""
        };

        let auto_indicator = if auto_scroll { "" } else { " [SCROLL] " };
        let title_text = format!(" SYSTEM LOGS ({logs_count}) {scroll_indicator}{auto_indicator}");

        let block = Block::default()
            .title(Span::styled(title_text, title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg));
        let paragraph = Paragraph::new(log_lines)
            .block(block)
            .style(Style::default().fg(text))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    }
}
