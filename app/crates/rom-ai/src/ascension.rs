//! The ascension (task 76). A computer player starts the ascension project once everything at
//! its library is researched and it either faces defeat (the military manager's own measure)
//! or has nothing left to gain (no material rival is left); ascended, it descends again after
//! `ascension.ai_descend_ticks`, on open land of the habitable planet farthest from its
//! enemies' cities.

use crate::{memory::Memory, plan::Plan, view::operational};
use rom_rules::ProductionMode;
use rom_sim::{BodyId, Command, EntityId, PlayerId, PlayerStatus, Pos, Topology, World};

/// How good a place to descend is, greatest best: squared star-map distance to the nearest
/// enemy city's system, ground distance to the nearest enemy city on the planet, lower body id.
type Rank = (i64, u64, std::cmp::Reverse<u32>);

/// Orders the ascension at a free library when it is time (see the module). Returns whether
/// it did.
pub(crate) fn ascend(plan: &mut Plan<'_>, memory: &Memory) -> bool {
    let view = &plan.view;
    let world = view.world;
    let me = view.me;
    if !world.ascension_ready(me) || world.ascension_projects().iter().any(|p| p.player == me) {
        return false;
    }
    let team = world.players().find(|p| p.id == me).map(|p| p.team);
    let alone = team.is_some() && world.last_standing() == team;
    if !alone && !crate::army::facing_defeat(view, memory) {
        return false;
    }
    let found = view.own.iter().filter(|e| operational(e)).find_map(|e| {
        let tech = view
            .rules
            .production
            .get(&e.type_id)?
            .iter()
            .find(|p| p.mode == ProductionMode::Research && view.rules.ascends(p.product_def_id))?
            .product_def_id;
        Some((e.id, tech))
    });
    let Some((library, tech)) = found else {
        return false;
    };
    let affordable = view
        .price(tech)
        .iter()
        .all(|(r, amount)| view.purse.get(r).copied().unwrap_or(0) >= *amount);
    affordable && plan.order(library, "research", Some(tech), None, None)
}

/// The descent of an ascended computer player, once it has been ascended for
/// `ascension.ai_descend_ticks`: the open land, on a planet with a surface, farthest from the
/// cities of material enemies (the farthest star system first, then the farthest ground).
pub fn descend(world: &World, me: PlayerId) -> Option<Command> {
    let since = world.ascended_since(me)?;
    let wait = world
        .rules()
        .constant("ascension.ai_descend_ticks")
        .unwrap_or(0)
        .max(0) as u64;
    if world.tick() < since.saturating_add(wait) {
        return None;
    }
    let enemies: Vec<Pos> = world
        .players()
        .filter(|p| {
            p.id != PlayerId(0)
                && p.id != me
                && world.enemies(me, p.id)
                && matches!(
                    world.player_status(p.id),
                    Some(PlayerStatus::Playing | PlayerStatus::Won)
                )
        })
        .flat_map(|p| world.cities(p.id))
        .filter_map(|id| world.entity(id).map(|e| e.pos))
        .collect();
    let system_of = |body: BodyId| world.body(body).map(|b| b.system);
    let place_of = |system: Option<u32>| {
        system
            .and_then(|s| world.systems().iter().find(|i| i.id == s))
            .map(|s| (i64::from(s.x), i64::from(s.y), i64::from(s.z)))
    };
    // Squared star-map distance from a body's system to the nearest enemy city's system.
    let far_in_space = |body: BodyId| -> i64 {
        let here = place_of(system_of(body));
        enemies
            .iter()
            .map(|e| match (here, place_of(system_of(e.body))) {
                (Some(a), Some(b)) => (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) + (a.2 - b.2).pow(2),
                _ if e.body == body => 0,
                _ => i64::MAX / 4,
            })
            .min()
            .unwrap_or(i64::MAX / 4)
    };
    let mut best: Option<(Rank, Pos)> = None;
    for surface in world.surfaces() {
        let body = surface.body();
        if world
            .body(body)
            .is_some_and(|b| b.kind != rom_rules::BodyKind::Habitable)
        {
            continue;
        }
        let space = far_in_space(body);
        for at in candidates(body, surface.topology()) {
            let Some(site) = world.find_city_site(me, body, at) else {
                continue;
            };
            if world.descent_site(me, site).is_err() {
                continue;
            }
            let ground = enemies
                .iter()
                .filter(|e| e.body == body)
                .map(|e| world.distance(*e, site).unwrap_or(u64::MAX / 4))
                .min()
                .unwrap_or(u64::MAX / 4);
            let key = (space, ground, std::cmp::Reverse(body.0));
            if best.as_ref().is_none_or(|(k, _)| key > *k) {
                best = Some((key, site));
            }
        }
    }
    let (_, site) = best?;
    Some(Command {
        player: me,
        executor: EntityId(0),
        kind: "descend".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(site),
        queue: false,
    })
}

/// Places to look for a site on a surface: the middle and the quarters of every face of a
/// planet, a grid over a flat map.
fn candidates(body: BodyId, topology: Topology) -> Vec<Pos> {
    match topology {
        Topology::Cube(cube) => {
            let n = cube.n as i32 * 1000;
            (0..6u8)
                .flat_map(|face| {
                    [(2, 2), (1, 1), (3, 1), (1, 3), (3, 3)]
                        .map(|(x, y)| Pos::on(body, face, n * x / 4, n * y / 4))
                })
                .collect()
        }
        Topology::Flat { width, height } => {
            let (w, h) = (width as i32 * 1000, height as i32 * 1000);
            (1..=5)
                .flat_map(|x| (1..=5).map(move |y| Pos::on(body, 0, w * x / 6, h * y / 6)))
                .collect()
        }
    }
}

/// What an ascended computer player is answering in the celestial home's channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTopic {
    /// It has just ascended.
    Arrived,
    /// Another player has just ascended.
    Welcome,
    /// It is about to descend.
    Farewell,
    /// Someone spoke to the channel.
    Reply,
}

/// The short lines of an ascended computer player, by topic.
const LINES: [(ChatTopic, &[&str]); 4] = [
    (
        ChatTopic::Arrived,
        &[
            "Free of it all at last.",
            "So this is what lies past the stars.",
            "Greetings. The war below is far away now.",
        ],
    ),
    (
        ChatTopic::Welcome,
        &[
            "Welcome to the galactic centre.",
            "Another one joins us. Welcome.",
            "Welcome. Rest a while before you go down again.",
        ],
    ),
    (
        ChatTopic::Farewell,
        &[
            "I am going down again. Farewell.",
            "Time to start over. See you below.",
            "A new city calls. Farewell for now.",
        ],
    ),
    (
        ChatTopic::Reply,
        &[
            "Indeed.",
            "We have time enough to talk.",
            "The view from here never tires.",
            "Some day I will go back down.",
        ],
    ),
];

/// A short line `player` says on `topic` at `tick`; the choice depends only on its inputs.
pub fn chat_line(player: PlayerId, topic: ChatTopic, tick: u64) -> &'static str {
    let lines = LINES
        .iter()
        .find(|(t, _)| *t == topic)
        .map_or(&[][..], |(_, lines)| *lines);
    let pick = (u64::from(player.0).wrapping_mul(0x9e37_79b9) ^ tick) as usize;
    lines.get(pick % lines.len().max(1)).copied().unwrap_or("")
}
