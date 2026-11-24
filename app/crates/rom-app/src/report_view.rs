//! The match report (task 117), drawn by GPUI: the facts of the match, the players ranked in
//! every aspect, and a line chart per aspect over game time. The in-match menu shows it from
//! the end dialog; the launcher shows it for a saved game.

use gpui::{
    AnyElement, IntoElement, ParentElement, PathBuilder, SharedString, Styled, canvas, div, point,
    px, rgb,
};
use rom_match::{Aspect, Chart, MatchReport, report::ordinal};
use rom_sim::PlayerId;

const GOLD: u32 = 0xe8c98a;
const MUTED: u32 = 0x9fb0c3;
const TEXT: u32 = 0xf2f2f2;
const ROW: u32 = 0x0a0e14;
const ME: u32 = 0x1d2a38;
const GRID: u32 = 0x2b3a4a;

/// A player's line colour, as the world draws its owner.
fn colour(id: PlayerId) -> u32 {
    let [r, g, b, _] = crate::ascension_view::colour_of(id);
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// The title line of the end dialog for `me`: why the match ended.
pub fn end_lines(report: &MatchReport, me: Option<PlayerId>) -> Vec<String> {
    let mut lines: Vec<String> = report.end_line(me).into_iter().collect();
    lines.extend(report.winner_line());
    lines
}

/// The whole report as one element, `width` pixels wide.
pub fn report_element(report: &MatchReport, me: Option<PlayerId>, width: f32) -> AnyElement {
    let facts = [
        ("Map style", report.map_style_name()),
        ("Seed", report.seed.to_string()),
        ("Length", report.clock(report.length())),
        ("End", report.end_text()),
        (
            "First victory",
            report.victory.as_ref().map_or_else(
                || "none".to_owned(),
                |(team, rule, tick)| {
                    format!(
                        "team {team} by {} at {}",
                        rule.replace('_', " "),
                        report.clock(*tick)
                    )
                },
            ),
        ),
    ];
    let mut facts_row = div().flex().flex_row().flex_wrap().gap_4();
    for (label, value) in facts {
        facts_row = facts_row.child(
            div()
                .flex()
                .gap_1()
                .child(div().text_color(rgb(MUTED)).child(label))
                .child(div().text_color(rgb(TEXT)).child(value)),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_3()
        .w(px(width))
        .child(facts_row)
        .child(ranking(report, me))
        .child(cities(report))
        .child(charts(report, width))
        .into_any_element()
}

/// The cities each player holds, by name, its capital first (task 144).
fn cities(report: &MatchReport) -> AnyElement {
    let mut list = div()
        .flex()
        .flex_col()
        .gap_1()
        .px_2()
        .child(div().text_xs().text_color(rgb(GOLD)).child("Cities"));
    for p in &report.players {
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .text_sm()
                .child(div().size(px(10.)).rounded(px(2.)).bg(rgb(colour(p.id))))
                .child(div().flex_none().text_color(rgb(TEXT)).child(p.label()))
                .child(div().text_color(rgb(MUTED)).child(report.cities_text(p))),
        );
    }
    list.into_any_element()
}

/// The ranking table: one row per player, a column per aspect with its rank.
fn ranking(report: &MatchReport, me: Option<PlayerId>) -> AnyElement {
    let widths = [170., 90., 76., 116.];
    let aspect_width = 118.;
    let cell = |text: String, w: f32, colour: u32, bold: bool| {
        let d = div()
            .w(px(w))
            .flex_none()
            .overflow_hidden()
            .text_color(rgb(colour))
            .child(SharedString::from(text));
        if bold {
            d.font_weight(gpui::FontWeight::BOLD)
        } else {
            d
        }
    };
    let mut head = div().flex().flex_row().gap_2().px_2().text_xs();
    for (title, w) in ["Player", "Race", "Played by", "Standing"]
        .iter()
        .zip(widths)
    {
        head = head.child(cell((*title).to_owned(), w, MUTED, false));
    }
    for aspect in Aspect::ALL {
        head = head.child(cell(aspect.label().to_owned(), aspect_width, GOLD, true));
    }
    let mut table = div().flex().flex_col().gap_1().child(head);
    for p in &report.players {
        let mut row = div()
            .flex()
            .flex_row()
            .items_start()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(rgb(if Some(p.id) == me { ME } else { ROW }))
            .text_sm()
            .child(
                div()
                    .w(px(widths[0]))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(10.)).rounded(px(2.)).bg(rgb(colour(p.id))))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_color(rgb(TEXT)).child(p.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(p.nation.clone()),
                            ),
                    ),
            )
            .child(cell(p.race.clone(), widths[1], TEXT, false))
            .child(cell(
                if p.human { "human" } else { "computer" }.to_owned(),
                widths[2],
                TEXT,
                false,
            ))
            .child(cell(report.standing_text(p), widths[3], TEXT, false));
        for (index, aspect) in Aspect::ALL.iter().enumerate() {
            let rank = p.ranks[index];
            let leader = report.leads(p, index);
            row = row.child(
                div()
                    .w(px(aspect_width))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_color(rgb(if leader { GOLD } else { TEXT }))
                            .font_weight(if leader {
                                gpui::FontWeight::BOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .child(if leader {
                                format!("{} · leader", ordinal(rank))
                            } else {
                                ordinal(rank)
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(report.cell(p, *aspect)),
                    ),
            );
        }
        table = table.child(row);
    }
    table.into_any_element()
}

/// The charts, two to a row, with one legend.
fn charts(report: &MatchReport, width: f32) -> AnyElement {
    let chart_width = (width - 16.) / 2.;
    let mut grid = div().flex().flex_row().flex_wrap().gap_4();
    for chart in &report.charts {
        grid = grid.child(line_chart(report, chart, chart_width, 150.));
    }
    let mut legend = div().flex().flex_row().flex_wrap().gap_3().text_xs();
    for p in &report.players {
        legend = legend.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().w(px(14.)).h(px(3.)).bg(rgb(colour(p.id))))
                .child(div().text_color(rgb(TEXT)).child(p.name.clone())),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(legend)
        .child(grid)
        .into_any_element()
}

/// One chart: a line per player over game time, from zero to the largest value.
fn line_chart(report: &MatchReport, chart: &Chart, width: f32, height: f32) -> AnyElement {
    let end = report
        .charts
        .iter()
        .flat_map(|c| c.series.iter())
        .flat_map(|s| s.points.last().map(|p| p.0))
        .max()
        .unwrap_or(0)
        .max(1);
    let top = chart
        .series
        .iter()
        .flat_map(|s| s.points.iter().map(|p| p.1))
        .max()
        .unwrap_or(0)
        .max(1);
    let lines: Vec<(u32, Vec<(f32, f32)>)> = chart
        .series
        .iter()
        .map(|s| {
            let points = s
                .points
                .iter()
                .map(|(tick, value)| {
                    (
                        *tick as f32 / end as f32,
                        (*value).max(0) as f32 / top as f32,
                    )
                })
                .collect();
            (colour(s.player), points)
        })
        .collect();
    let empty = lines.iter().all(|(_, points)| points.is_empty());
    let plot = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            for step in 0..=4 {
                let y = y0 + h * step as f32 / 4.;
                let mut grid = PathBuilder::stroke(px(1.));
                grid.move_to(point(px(x0), px(y)));
                grid.line_to(point(px(x0 + w), px(y)));
                if let Ok(path) = grid.build() {
                    window.paint_path(path, rgb(GRID));
                }
            }
            for (colour, points) in &lines {
                let mut line = PathBuilder::stroke(px(2.));
                // Every curve starts at the beginning of the match.
                line.move_to(point(px(x0), px(y0 + h)));
                for (fx, fy) in points {
                    line.line_to(point(px(x0 + fx * w), px(y0 + h - fy * h)));
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, rgb(*colour));
                }
            }
        },
    )
    .w(px(width))
    .h(px(height));
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w(px(width))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(rgb(GOLD)).child(chart.title))
                .child(div().text_color(rgb(MUTED)).child(format!("up to {top}"))),
        )
        .child(
            div()
                .bg(rgb(ROW))
                .rounded(px(4.))
                .p_1()
                .child(plot)
                .children(empty.then(|| {
                    div()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child("No samples yet: one is taken every game minute.")
                })),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .text_color(rgb(MUTED))
                .child("00:00")
                .child(report.clock(end)),
        )
        .into_any_element()
}
