use crate::control::ControlWorker;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Gauge, Paragraph},
    Terminal,
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

pub fn run(controls: Arc<Mutex<Vec<Arc<ControlWorker>>>>) -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, controls);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    controls_mutex: Arc<Mutex<Vec<Arc<ControlWorker>>>>,
) -> io::Result<()> {
    let mut selected_idx = 0;
    let mut dragging: Option<usize> = None;
    let labels: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();

    loop {
        let controls_snapshot = {
            let guard = controls_mutex.lock().unwrap();
            guard.clone()
        };

        if !controls_snapshot.is_empty() && selected_idx >= controls_snapshot.len() {
            selected_idx = controls_snapshot.len() - 1;
        }

        terminal.draw(|f| {
            let size = f.size();

            let pad_top: u16 = 1;
            let pad_bottom: u16 = 1;

            let border_rect = Rect::new(
                size.x,
                size.y + pad_top,
                size.width,
                size.height.saturating_sub(pad_top + pad_bottom),
            );

            let block = Block::default()
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(Color::Cyan))
                .borders(ratatui::widgets::Borders::ALL);
            f.render_widget(&block, border_rect);

            let inner = block.inner(border_rect);

            if inner.height < 6 || inner.width < 40 {
                let p = Paragraph::new("too small :(");
                f.render_widget(p, size);
                return;
            }

            let title_text = " H E L I O R I C ";
            let title_y = border_rect.y;
            let inner_w = inner.width as usize;
            let inner_x = inner.x as usize;
            let title_x = inner_x + (inner_w.saturating_sub(title_text.len())) / 2;

            f.render_widget(
                Paragraph::new(Span::styled(
                    title_text,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Rect::new(title_x as u16, title_y, title_text.len() as u16, 1),
            );

            if controls_snapshot.is_empty() {
                let msg = "Scanning for monitors...";
                let msg_x = inner_x + (inner_w.saturating_sub(msg.len())) / 2;
                f.render_widget(
                    Paragraph::new(Span::styled(msg, Style::default().fg(Color::DarkGray))),
                    Rect::new(
                        msg_x as u16,
                        inner.y + inner.height / 2,
                        msg.len() as u16,
                        1,
                    ),
                );
            } else {
                let list_height = controls_snapshot.len() * 3;
                let inner_h = inner.height as usize;
                let inner_y = inner.y as usize;
                let start_y = std::cmp::max(
                    title_y as usize + 1,
                    (inner_y + inner_h / 2).saturating_sub(list_height / 2),
                ) as u16;

                for (i, ctrl) in controls_snapshot.iter().enumerate() {
                    let active = i == selected_idx;
                    let draw_y = start_y + (i * 3) as u16;
                    let label = if controls_snapshot.len() == 1 {
                        None
                    } else {
                        labels.get(i).copied()
                    };

                    if draw_y < inner.y + inner.height - 2 {
                        draw_bar(f, draw_y, inner.width, ctrl, active, label);
                    }
                }
            }

            let help_txt = " [ABC] select   [123] set   [esc] quit ";
            let help_x = inner_x + (inner_w.saturating_sub(help_txt.len())) / 2;
            if inner.height > 2 {
                f.render_widget(
                    Paragraph::new(Span::styled(help_txt, Style::default().fg(Color::DarkGray))),
                    Rect::new(
                        help_x as u16,
                        border_rect.y + border_rect.height - 1,
                        help_txt.len() as u16,
                        1,
                    ),
                );
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !controls_snapshot.is_empty() {
                            selected_idx = (selected_idx + 1) % controls_snapshot.len();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !controls_snapshot.is_empty() {
                            if selected_idx == 0 {
                                selected_idx = controls_snapshot.len() - 1;
                            } else {
                                selected_idx -= 1;
                            }
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        if !controls_snapshot.is_empty() {
                            let ctrl = &controls_snapshot[selected_idx];
                            ctrl.set_target(ctrl.get_value() + 5);
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if !controls_snapshot.is_empty() {
                            let ctrl = &controls_snapshot[selected_idx];
                            ctrl.set_target(ctrl.get_value() - 5);
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if !controls_snapshot.is_empty() {
                            if let Some(d) = c.to_digit(10) {
                                let mut val = d as i32 * 10;
                                if val == 0 {
                                    val = 100;
                                } else if val == 10 {
                                    val = 0;
                                }

                                controls_snapshot[selected_idx].set_target(val);
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(idx) = labels.iter().position(|&l| l == c.to_ascii_uppercase())
                        {
                            if idx < controls_snapshot.len() {
                                selected_idx = idx;
                            }
                        }
                    }
                    _ => {}
                },

                Event::Mouse(me) => {
                    // mirror the layout math from draw_bar to find which bar was clicked
                    if !controls_snapshot.is_empty() {
                        let size = terminal.size()?;

                        let list_height = controls_snapshot.len() * 3;
                        let start_y = std::cmp::max(
                            4,
                            (size.height as usize / 2).saturating_sub(list_height / 2),
                        );

                        let bar_width = 40usize;
                        let bar_x = (size.width as usize / 2).saturating_sub(bar_width / 2);

                        let col_to_percent = |col: u16| -> i32 {
                            let col_i = col as i32;
                            let rel = col_i - bar_x as i32;
                            let rel = rel.clamp(0, bar_width as i32);
                            let pct = (rel as f64 / bar_width as f64) * 100.0;
                            let p = pct.round() as i32;
                            // snap to nearest 5 to match ControlWorker behavior
                            let snapped = ((p + 2) / 5) * 5;
                            snapped.clamp(0, 100)
                        };

                        match me.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                for (i, _ctrl) in controls_snapshot.iter().enumerate() {
                                    let draw_y = (start_y + (i * 3)) as u16;
                                    if draw_y < size.height - 2 {
                                        if me.row == draw_y {
                                            selected_idx = i;
                                            if (me.column as usize) >= bar_x
                                                && (me.column as usize) < (bar_x + bar_width)
                                            {
                                                dragging = Some(i);
                                                let val = col_to_percent(me.column).clamp(0, 100);
                                                controls_snapshot[i].set_target(val);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            MouseEventKind::Drag(MouseButton::Left) => {
                                if let Some(idx) = dragging {
                                    if idx < controls_snapshot.len() {
                                        let val = col_to_percent(me.column).clamp(0, 100);
                                        controls_snapshot[idx].set_target(val);
                                    }
                                }
                            }

                            MouseEventKind::Up(MouseButton::Left) => {
                                dragging = None;
                            }

                            MouseEventKind::Moved => {
                                if let Some(idx) = dragging {
                                    if idx < controls_snapshot.len() {
                                        let val = col_to_percent(me.column).clamp(0, 100);
                                        controls_snapshot[idx].set_target(val);
                                    }
                                }
                            }

                            _ => {}
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

fn draw_bar(
    f: &mut ratatui::Frame,
    y: u16,
    win_width: u16,
    ctrl: &Arc<ControlWorker>,
    active: bool,
    label: Option<char>,
) {
    let bar_width = 40;
    let bar_x = (win_width as usize / 2).saturating_sub(bar_width / 2);

    if bar_x < 10 {
        return;
    }

    let value = ctrl.get_value();
    let name = &ctrl.name;

    let label_width = 4;
    let max_name_len = 20;

    let label_start_x = bar_x
        .saturating_sub(2)
        .saturating_sub(max_name_len)
        .saturating_sub(label_width);

    if let Some(l) = label {
        let label_str = format!("[{}]", l);
        let style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        f.render_widget(
            Paragraph::new(Span::styled(label_str, style)),
            Rect::new(label_start_x as u16, y, 4, 1),
        );
    }

    let name_style = if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let disp_name = if name.len() > 19 {
        format!("{}…", &name[..19])
    } else {
        name.clone()
    };

    let name_start_x = label_start_x + label_width + 1;

    f.render_widget(
        Paragraph::new(Span::styled(disp_name.clone(), name_style)),
        Rect::new(name_start_x as u16, y, max_name_len as u16, 1),
    );

    if !ctrl.is_ready() {
        let loading_txt = " Discovering... ";
        f.render_widget(
            Paragraph::new(Span::styled(
                loading_txt,
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(bar_x as u16, y, bar_width as u16, 1),
        );
    } else {
        let gauge = Gauge::default()
            .gauge_style(if active {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            })
            .ratio(value as f64 / 100.0)
            .label(Span::raw(""))
            .use_unicode(true);

        f.render_widget(gauge, Rect::new(bar_x as u16, y, bar_width as u16, 1));

        let val_str = format!("[{}%]", value);
        let val_style = if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM)
        };

        let val_area_width = 8;
        let val_start_x = bar_x + bar_width as usize + 2 + (val_area_width - val_str.len());

        f.render_widget(
            Paragraph::new(Span::styled(val_str.clone(), val_style)),
            Rect::new(val_start_x as u16, y, val_str.len() as u16, 1),
        );
    }
}
