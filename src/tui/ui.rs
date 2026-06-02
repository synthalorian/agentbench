use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::app::App;
use super::theme::SynthwaveTheme;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    draw_header(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, chunks[2]);
}

fn draw_header(f: &mut Frame, area: ratatui::layout::Rect) {
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled(
                "AgentBench",
                Style::default()
                    .fg(SynthwaveTheme::NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — "),
            Span::styled(
                "Agent Benchmark Runner",
                Style::default().fg(SynthwaveTheme::HOT_PINK),
            ),
        ]),
        Line::from(vec![Span::styled(
            "v0.1.0",
            Style::default().fg(SynthwaveTheme::MUTED),
        )]),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SynthwaveTheme::ELECTRIC_PURPLE))
            .title(Span::styled(
                " 🎹🦈 ",
                Style::default().fg(SynthwaveTheme::CYAN),
            )),
    );

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_run_list(f, app, chunks[0]);
    draw_detail_pane(f, app, chunks[1]);
}

fn draw_run_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let header_cells = ["Run ID", "Harness", "Benchmark", "Score", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(SynthwaveTheme::NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells)
        .style(Style::default().bg(SynthwaveTheme::DEEP_PURPLE))
        .height(1);

    let rows: Vec<Row> = app
        .runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let style = if i == app.selected {
                Style::default()
                    .bg(SynthwaveTheme::ELECTRIC_PURPLE)
                    .fg(SynthwaveTheme::TEXT)
            } else {
                Style::default().fg(SynthwaveTheme::TEXT)
            };

            let score = run
                .aggregate_score
                .map(|s| format!("{:.1}%", s * 100.0))
                .unwrap_or_else(|| "—".to_string());

            Row::new(vec![
                Cell::from(run.id.chars().take(8).collect::<String>()),
                Cell::from(run.harness_name.clone()),
                Cell::from(run.benchmark_name.clone()),
                Cell::from(score),
                Cell::from(run.status.clone()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Benchmark Runs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(SynthwaveTheme::HOT_PINK)),
    );

    f.render_widget(table, area);
}

fn draw_detail_pane(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let text = Text::from(vec![
        Line::from(Span::styled(
            "Select a run to view details",
            Style::default().fg(SynthwaveTheme::MUTED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "r",
                Style::default()
                    .fg(SynthwaveTheme::NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to refresh, "),
            Span::styled(
                "q",
                Style::default()
                    .fg(SynthwaveTheme::NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to quit"),
        ]),
    ]);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SynthwaveTheme::CYAN)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, area: ratatui::layout::Rect) {
    let footer = Paragraph::new(Text::from(vec![Line::from(vec![
        Span::styled("[q]uit", Style::default().fg(SynthwaveTheme::MUTED)),
        Span::raw(" | "),
        Span::styled("[r]efresh", Style::default().fg(SynthwaveTheme::MUTED)),
        Span::raw(" | "),
        Span::styled("↑↓", Style::default().fg(SynthwaveTheme::MUTED)),
        Span::raw(" navigate"),
    ])]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(SynthwaveTheme::ELECTRIC_PURPLE)),
    );

    f.render_widget(footer, area);
}
