//! Formations from the command card (task 143): the formation a selection's group orders name,
//! its page on the Order tab and its hotkeys (Alt+1 to Alt+9). The simulation lays the group out
//! (`rom-sim`, `formation.rs`); here the person picks the shape.

use std::collections::BTreeMap;

use rom_rules::{DefId, FormationShape, Rules};
use rom_sim::{EntityId, Pos, World};

use super::{Controller, OrderSink};

/// The formations the card offers, in its order: the first row Square, Compact, Circle, Line and
/// Double line (Q W E R T), then the others. A shape without a `formation` row is left out.
const SHAPES: [FormationShape; 14] = [
    FormationShape::Square,
    FormationShape::Compact,
    FormationShape::Circle,
    FormationShape::Line,
    FormationShape::DoubleLine,
    FormationShape::Column,
    FormationShape::Wedge,
    FormationShape::EchelonLeft,
    FormationShape::EchelonRight,
    FormationShape::Sparse,
    FormationShape::Envelop,
    FormationShape::Refused,
    FormationShape::Mob,
    FormationShape::Sphere,
];

/// The shape a group takes until the person picks another (owner, task 143: a big fleet no
/// longer stands in a circle).
const DEFAULT_SHAPE: FormationShape = FormationShape::Square;

/// The formations offered, in card order: the first `formation` row of each offered shape.
pub(super) fn offered(rules: &Rules) -> Vec<DefId> {
    SHAPES
        .iter()
        .filter_map(|shape| {
            rules
                .formations
                .values()
                .find(|f| f.shape == *shape)
                .map(|f| f.id)
        })
        .collect()
}

/// The icon of a formation's button.
pub(super) fn icon(rules: &Rules, formation: DefId) -> &'static str {
    match rules.formations.get(&formation).map(|f| f.shape) {
        Some(FormationShape::Square) => "icons/form_square",
        Some(FormationShape::Compact) => "icons/form_compact",
        Some(FormationShape::Circle | FormationShape::Sphere) => "icons/form_circle",
        Some(FormationShape::Line) => "icons/form_line",
        Some(FormationShape::DoubleLine) => "icons/form_double_line",
        Some(FormationShape::Column) => "icons/form_column",
        Some(FormationShape::Wedge) => "icons/form_wedge",
        Some(FormationShape::EchelonLeft) => "icons/form_echelon_left",
        Some(FormationShape::EchelonRight) => "icons/form_echelon_right",
        Some(FormationShape::Sparse) => "icons/form_sparse",
        Some(FormationShape::Envelop) => "icons/form_envelop",
        Some(FormationShape::Refused) => "icons/form_refused",
        Some(FormationShape::Mob) => "icons/form_mob",
        None => "icons/cmd_move",
    }
}

/// The button text of a formation: its name, its hotkey when it has one.
pub(super) fn label(rules: &Rules, formation: DefId) -> String {
    let name = &rules.def(formation).name;
    match offered(rules).iter().position(|f| *f == formation) {
        Some(n) if n < 9 => format!("Formation: {name} (Alt+{})", n + 1),
        _ => format!("Formation: {name}"),
    }
}

/// The point among `points` nearest to all the others: the middle of a group, on its surface.
fn middle(world: &World, points: &[Pos]) -> Option<Pos> {
    points
        .iter()
        .min_by_key(|a| {
            points
                .iter()
                .map(|b| u128::from(world.distance(**a, *b).unwrap_or(u64::MAX / 4)))
                .sum::<u128>()
        })
        .copied()
}

impl Controller {
    /// The formation the selection's group orders name: the one picked last, Square until then.
    pub(super) fn formation(&self, world: &World) -> Option<DefId> {
        let rules = world.rules();
        self.formation
            .filter(|f| rules.formations.contains_key(f))
            .or_else(|| {
                rules
                    .formations
                    .values()
                    .find(|f| f.shape == DEFAULT_SHAPE)
                    .or_else(|| rules.formations.values().next())
                    .map(|f| f.id)
            })
    }

    /// The own selected units that can move.
    pub(super) fn movers(&self, world: &World) -> Vec<EntityId> {
        let rules = world.rules();
        self.own_selected(world)
            .iter()
            .filter(|e| rules.entity_types[&e.type_id].movement.is_some())
            .map(|e| e.id)
            .collect()
    }

    /// The formation the orders given to the selection now name: one of at least two movers.
    pub(super) fn group_formation(&self, world: &World) -> Option<DefId> {
        (self.movers(world).len() >= 2)
            .then(|| self.formation(world))
            .flatten()
    }

    /// The formation of an Alt+digit hotkey (1 is the first on the card).
    pub(super) fn formation_hotkey(&self, world: &World, digit: u32) -> Option<DefId> {
        let n = usize::try_from(digit).ok()?.checked_sub(1)?;
        offered(world.rules()).get(n).copied()
    }

    /// Picks a formation for this and later group orders, and puts the selection into it: units
    /// standing or walking form up about the middle of where they are bound (an attack move
    /// stays one), ships holding a body beside it take their places there again. Units at work
    /// or in a fight keep at it and take the formation with their next order.
    pub(super) fn pick_formation(&mut self, world: &mut OrderSink<'_>, formation: DefId) {
        self.formation = Some(formation);
        let rules = world.rules().clone();
        let mut walkers: Vec<(EntityId, Pos, bool)> = Vec::new();
        let mut holders: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        for id in self.movers(world) {
            let Some(e) = world.entity(id) else {
                continue;
            };
            if world.is_landed(id) || e.container_id.is_some() {
                continue;
            }
            let orbit = world.orbit(id).map(|o| o.pos);
            let armed = !rules.entity_types[&e.type_id].weapons.is_empty();
            match e.orders().front() {
                None => walkers.push((id, orbit.unwrap_or(e.pos), false)),
                Some(o)
                    if matches!(o.kind.as_str(), "move" | "attack_move")
                        && o.target_entity.is_none() =>
                {
                    if let Some(to) = o.target_pos {
                        walkers.push((id, to, armed && o.kind == "attack_move"));
                    }
                }
                Some(o) if o.kind == "orbit" => match (orbit, o.target_entity) {
                    (Some(at), _) => walkers.push((id, at, false)),
                    (None, Some(body)) => holders.entry(body).or_default().push(id),
                    (None, None) => {}
                },
                Some(_) => {}
            }
        }
        if let Some(body) = walkers.first().map(|(_, at, _)| at.body) {
            walkers.retain(|(_, at, _)| at.body == body);
        }
        let points: Vec<Pos> = walkers.iter().map(|(_, at, _)| *at).collect();
        if walkers.len() >= 2
            && let Some(goal) = middle(world, &points)
        {
            for (id, _, attack) in walkers {
                let kind = if attack { "attack_move" } else { "move" };
                self.submit(world, id, kind, Some(formation), None, Some(goal), false);
            }
        }
        for (body, ships) in holders {
            if ships.len() >= 2 {
                for id in ships {
                    self.submit(world, id, "orbit", Some(formation), Some(body), None, false);
                }
            }
        }
    }
}
