//! Fortification lines and minefields by dragging (task 147b). With a segment type armed
//! (a building with an `et_fortification` row that is no gate) a press on the ground starts a
//! line and the drag draws it, site by site, along the ground; G or a right click while
//! dragging drops a gate (three tiles along the line) at the pointer; the release builds it.
//! A press on an own wall continues the line from there. With a minefield type armed the drag
//! is a rectangle of mines. The selected builders take the sites in line order, one stretch
//! each; the tool stays armed for the next line until Escape or a right click.

use rom_render::{HudLineSite, MarkerKind};
use rom_rules::{DefId, ProductionMode};
use rom_sim::{Command, EntityId, LineSite, Pos, SimError, World};

use super::{Controller, OrderSink, distance};

/// What an armed type lays when dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LineKind {
    /// A chain of segments with gates (`World::line_sites`).
    Segments,
    /// A rectangle of mines (`World::minefield_sites`).
    Mines,
}

/// The line or minefield being dragged.
#[derive(Clone, Debug)]
pub(super) struct LineDrag {
    def: DefId,
    kind: LineKind,
    start: Pos,
    end: Pos,
    gates: Vec<Pos>,
    /// The press was on an own wall: the sites it already covers are left out.
    continues: bool,
    sites: Vec<LineSite>,
}

/// What dragging `def` lays, if anything: from its components, never its key.
pub(super) fn line_kind(world: &World, def: DefId) -> Option<LineKind> {
    let et = world.rules().entity_type(def)?;
    match (&et.fortification, &et.minefield) {
        (Some(f), _) if !f.gate => Some(LineKind::Segments),
        (_, Some(_)) => Some(LineKind::Mines),
        _ => None,
    }
}

impl Controller {
    /// Whether the armed placement is a line or minefield type (the left drag draws it).
    pub(super) fn line_armed(&self, world: &World) -> Option<LineKind> {
        self.placement
            .as_ref()
            .and_then(|p| line_kind(world, p.def))
    }

    /// The left button went down (`start`) or moved on while held, at `at`.
    pub(super) fn line_stroke(&mut self, world: &World, at: Pos, start: bool) {
        let Some(def) = self.placement.as_ref().map(|p| p.def) else {
            return;
        };
        let Some(kind) = line_kind(world, def) else {
            return;
        };
        if start || self.line.is_none() {
            let (start, continues) = match kind {
                LineKind::Segments => self.wall_end(world, at).map_or((at, false), |p| (p, true)),
                LineKind::Mines => (at, false),
            };
            self.line = Some(LineDrag {
                def,
                kind,
                start,
                end: at,
                gates: Vec::new(),
                continues,
                sites: Vec::new(),
            });
            self.refresh_line(world);
            return;
        }
        let drag = self.line.as_mut().expect("a line is dragged");
        let tile = |p: Pos| (p.body, p.face, p.x.div_euclid(1000), p.y.div_euclid(1000));
        if tile(drag.end) != tile(at) {
            drag.end = at;
            self.refresh_line(world);
        }
    }

    /// Drops a gate into the line being dragged at `at`, or at its end (where the pointer is)
    /// (G or a right click); false when no segment line is dragged.
    pub(super) fn drop_gate(&mut self, world: &World, at: Option<Pos>) -> bool {
        let Some(drag) = self.line.as_mut().filter(|d| d.kind == LineKind::Segments) else {
            return false;
        };
        let at = at.unwrap_or(drag.end);
        drag.gates.push(at);
        self.refresh_line(world);
        true
    }

    /// The centre of the own fortification under `at`, if any: a line pressed there continues
    /// from it.
    fn wall_end(&self, world: &World, at: Pos) -> Option<Pos> {
        let me = self.me();
        let rules = world.rules();
        world
            .entities()
            .filter(|e| e.owner == me && e.pos.body == at.body && e.pos.face == at.face)
            .filter(|e| {
                rules
                    .entity_type(e.type_id)
                    .is_some_and(|t| t.fortification.is_some())
            })
            .find(|e| {
                let et = &rules.entity_types[&e.type_id];
                let (w, h) = rom_sim::turned_footprint(et.footprint_w, et.footprint_h, e.facing);
                (i64::from(at.x) - i64::from(e.pos.x)).abs() <= w * 500
                    && (i64::from(at.y) - i64::from(e.pos.y)).abs() <= h * 500
            })
            .map(|e| e.pos)
    }

    fn refresh_line(&mut self, world: &World) {
        let me = self.me();
        let Some(drag) = &mut self.line else {
            return;
        };
        let sites = match drag.kind {
            LineKind::Segments => {
                world.line_sites(me, drag.def, &[drag.start, drag.end], &drag.gates)
            }
            LineKind::Mines => world.minefield_sites(me, drag.def, drag.start, drag.end),
        };
        drag.sites = match sites {
            Ok(mut sites) => {
                if drag.continues {
                    // The wall pressed on stands already: its tiles are not laid again.
                    let skip = sites
                        .iter()
                        .take_while(|s| s.check == Err(SimError::PlacementOverlap))
                        .count();
                    sites.drain(..skip);
                }
                sites
            }
            // Too long or across an edge: what was drawn last stays.
            Err(_) => std::mem::take(&mut drag.sites),
        };
    }

    /// The sites of the line being dragged, for the HUD.
    pub(super) fn line_hud(&self, world: &World) -> Vec<HudLineSite> {
        let Some(drag) = &self.line else {
            return Vec::new();
        };
        let rules = world.rules();
        let me = self.me();
        drag.sites
            .iter()
            .map(|s| {
                let et = &rules.entity_types[&s.subject];
                let (w, h) = rom_sim::turned_footprint(et.footprint_w, et.footprint_h, s.facing);
                let gate = et.fortification.as_ref().is_some_and(|f| f.gate);
                HudLineSite {
                    face: s.pos.face,
                    x: s.pos.x,
                    y: s.pos.y,
                    width: w.clamp(1, 255) as u8,
                    height: h.clamp(1, 255) as u8,
                    length: if drag.kind == LineKind::Mines {
                        0
                    } else {
                        et.footprint_w.clamp(1, 255) as u8
                    },
                    facing: s.facing,
                    owner: u8::try_from(me.0).unwrap_or(u8::MAX),
                    gate,
                    valid: s.check.is_ok(),
                }
            })
            .collect()
    }

    /// The line's count and price, and how to finish it, for the help line.
    pub(super) fn line_help(&self, world: &World) -> Option<String> {
        let drag = self.line.as_ref()?;
        let rules = world.rules();
        let me = self.me();
        let valid: Vec<&LineSite> = drag.sites.iter().filter(|s| s.check.is_ok()).collect();
        let gates = valid.iter().filter(|s| s.subject != drag.def).count();
        let mut price: std::collections::BTreeMap<DefId, i64> = Default::default();
        for site in &valid {
            for (resource, amount) in world.price(me, site.subject) {
                *price.entry(resource).or_default() += amount;
            }
        }
        let price = price
            .iter()
            .map(|(r, n)| format!("{n} {}", rules.def(*r).name.to_lowercase()))
            .collect::<Vec<_>>()
            .join(", ");
        let name = &rules.def(drag.def).name;
        let blocked = drag.sites.len() - valid.len();
        let mut text = match drag.kind {
            LineKind::Segments => format!(
                "{name}: {} segments, {gates} {} — {price}",
                valid.len() - gates,
                if gates == 1 { "gate" } else { "gates" }
            ),
            LineKind::Mines => format!("{name}: {} mines — {price}", valid.len()),
        };
        if blocked > 0 {
            text.push_str(&format!(" ({blocked} cannot stand there)"));
        }
        text.push_str(match drag.kind {
            LineKind::Segments => ". Release to build; G or right click drops a gate",
            LineKind::Mines => ". Release to lay them",
        });
        Some(text)
    }

    /// The left button was released: the selected builders take the line's sites in order,
    /// one stretch each.
    pub(super) fn finish_line(&mut self, world: &mut OrderSink<'_>, shift: bool) {
        let Some(drag) = self.line.take() else {
            return;
        };
        let sites: Vec<LineSite> = drag.sites.into_iter().filter(|s| s.check.is_ok()).collect();
        if sites.is_empty() {
            self.say(world, "Nothing of this line can stand there");
            self.cues.push(crate::sound::UiCue::Refused);
            return;
        }
        let rules = world.rules().clone();
        let body = sites[0].pos.body;
        let mut builders: Vec<EntityId> = self
            .own_selected(world)
            .iter()
            .filter(|e| e.build_progress.is_none() && e.pos.body == body)
            .filter(|e| {
                rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|row| {
                        row.mode == ProductionMode::Construct && row.product_def_id == drag.def
                    })
            })
            .map(|e| e.id)
            .collect();
        if builders.is_empty() {
            self.say(world, "No selected builder can build this");
            self.cues.push(crate::sound::UiCue::Refused);
            return;
        }
        let stretch = sites.len().div_ceil(builders.len());
        let mut laid = 0;
        'line: for chunk in sites.chunks(stretch) {
            // The free builder nearest the stretch's first site, the lowest id on a tie.
            let Some((index, _)) = builders
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    world
                        .entity(*id)
                        .map(|e| (i, (distance(world, e.pos, chunk[0].pos), *id)))
                })
                .min_by_key(|(_, key)| *key)
            else {
                break;
            };
            let builder = builders.remove(index);
            for (k, site) in chunk.iter().enumerate() {
                if !self.submit_construct(world, builder, site, shift || k > 0, k == 0) {
                    break 'line;
                }
                laid += 1;
            }
        }
        if laid > 0 {
            self.marker(world, sites[0].pos, MarkerKind::Move);
        }
    }

    /// Orders `builder` to construct one site, with its facing.
    fn submit_construct(
        &mut self,
        world: &mut OrderSink<'_>,
        builder: EntityId,
        site: &LineSite,
        queue: bool,
        cue: bool,
    ) -> bool {
        let command = Command {
            player: self.me(),
            executor: builder,
            kind: "construct".into(),
            subject: Some(site.subject),
            target_entity: None,
            target_pos: Some(site.pos),
            queue,
            text: None,
            facing: Some(site.facing),
        };
        match world.submit(command) {
            Ok(()) => {
                self.ordered = true;
                // One answer a builder, not one a segment.
                if cue {
                    self.cues.push(crate::sound::UiCue::Ordered {
                        executor: builder,
                        kind: "construct".to_owned(),
                    });
                }
                true
            }
            Err(error) => {
                let message =
                    super::rejection_message(world, self.me(), &error, Some(site.subject));
                self.say(world, message);
                self.cues.push(crate::sound::UiCue::Refused);
                false
            }
        }
    }
}
