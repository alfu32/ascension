//! The match report of a network match (task 117): as JSON for `report.json` and `/status`,
//! and read back from a server's state folder for `rom report`.

use std::{path::Path, sync::Arc};

use rom_match::{Aspect, MatchReport, Standing};
use rom_rules::Rules;
use rom_save::{Controller, MatchInfo, Seat};
use rom_sim::{EndReason, PlayerId};
use serde_json::{Value, json};

use crate::state::{SeatRow, StateDb};

/// The end reason's key: `no_human_left` or `last_standing`.
pub fn end_key(reason: EndReason) -> &'static str {
    match reason {
        EndReason::NoHumanLeft => "no_human_left",
        EndReason::LastStanding { .. } => "last_standing",
    }
}

/// One line per player for `/status`: id, name, human, standing, score and overall rank.
pub fn ranking(report: &MatchReport) -> Vec<Value> {
    report
        .players
        .iter()
        .map(|p| {
            json!({
                "player": p.id.0,
                "name": p.name,
                "nation": p.nation,
                "human": p.human,
                "standing": report.standing_text(p),
                "score": p.score,
                "rank": p.ranks[0],
            })
        })
        .collect()
}

/// The whole report, the same data as the report screen.
pub fn report_json(report: &MatchReport) -> Value {
    let players: Vec<Value> = report
        .players
        .iter()
        .map(|p| {
            let (standing, defeated_tick, defeated_by) = match &p.standing {
                Standing::Won => ("won", None, None),
                Standing::Defeated { tick, by } => ("defeated", Some(*tick), by.map(|b| b.0)),
                Standing::Ascended => ("ascended", None, None),
                Standing::Playing => ("playing", None, None),
            };
            let ranks: serde_json::Map<String, Value> = Aspect::ALL
                .iter()
                .zip(&p.ranks)
                .map(|(aspect, rank)| (aspect.label().to_lowercase(), json!(rank)))
                .collect();
            json!({
                "player": p.id.0,
                "name": p.name,
                "nation": p.nation,
                "race": p.race,
                "human": p.human,
                "team": p.team,
                "standing": standing,
                "defeated_tick": defeated_tick,
                "defeated_by": defeated_by,
                "score": p.score,
                "kills": p.kills,
                "losses": p.losses,
                "gathered": p.gathered,
                "spent": p.spent,
                "units_trained": p.units_trained,
                "buildings": p.buildings,
                "cities_founded": p.cities_founded,
                "cities_captured": p.cities_captured,
                "cities_lost": p.cities_lost,
                "peak_land": p.peak_land,
                "peak_army": p.peak_army,
                "techs": p.techs,
                "age": p.age.0,
                "age_tick": p.age.1,
                "ascensions": p.ascensions,
                "ranks": ranks,
            })
        })
        .collect();
    let charts: Vec<Value> = report
        .charts
        .iter()
        .map(|chart| {
            json!({
                "title": chart.title,
                "series": chart.series.iter().map(|s| json!({
                    "player": s.player.0,
                    "points": s.points,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "tick": report.tick,
        "ticks_per_second": report.ticks_per_second,
        "map_style": report.map_style,
        "seed": report.seed,
        "length_ticks": report.length(),
        "end": report.end.map(|end| json!({
            "tick": end.tick,
            "reason": end_key(end.reason),
            "team": match end.reason {
                EndReason::LastStanding { team } => Some(team),
                EndReason::NoHumanLeft => None,
            },
        })),
        "first_victory": report.victory.as_ref().map(|(team, rule, tick)| json!({
            "team": team,
            "rule": rule,
            "tick": tick,
        })),
        "players": players,
        "charts": charts,
    })
}

/// The seats of a network match as a save names them: people by their names.
pub fn seats_of(rows: &[SeatRow]) -> Vec<Seat> {
    rows.iter()
        .map(|row| Seat {
            player: PlayerId(row.seat + 1),
            slot: row.seat,
            name: if row.name.is_empty() {
                format!("Computer ({})", row.spec)
            } else {
                row.name.clone()
            },
            controller: if row.human {
                Controller::Remote
            } else {
                Controller::Ai
            },
            ai_profile: String::new(),
            color: row.seat + 1,
        })
        .collect()
}

/// The report of the match in a server's state folder (`server.db` or the folder itself): its
/// latest save, with the seats' names.
pub fn state_report(path: &Path, rules: &Arc<Rules>) -> Result<MatchReport, String> {
    let dir = if path.is_dir() {
        path
    } else {
        path.parent().ok_or("no state folder")?
    };
    let db = StateDb::open(dir).map_err(|e| e.to_string())?;
    let row = db
        .match_row()
        .map_err(|e| e.to_string())?
        .ok_or("the state folder has no match")?;
    let save = row.save.ok_or("the match has not been saved yet")?;
    let (world, info, _) =
        rom_match::load_match(&db.saves().join(save), rules).map_err(|e| e.to_string())?;
    let seats = seats_of(&db.seats().map_err(|e| e.to_string())?);
    Ok(MatchReport::of(&world, &MatchInfo { seats, ..info }))
}
