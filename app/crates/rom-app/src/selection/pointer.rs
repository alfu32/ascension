//! What the pointer rests on (task 151): the side of the hovered entity, its line of help, the
//! order a right click on it would give, and the flash on the target of an order just given.
//! The right click and the badge share one decision ([`Controller::right_order`]), so the badge
//! never promises an order the click does not give.

use rom_render::{HudFlash, HudPointer, OrderBadge, Side};
use rom_rules::{DefId, EntityClass};
use rom_sim::{Entity, EntityId, PlayerId, Pos, World};

use super::{Controller, present};

/// What a right click names on the planet: the ground, or an entity and what it is to the
/// selection.
pub(super) enum RightTarget {
    /// Open ground, or an entity nothing special is done with.
    Ground,
    /// An own carrier that units can board.
    Board(EntityId),
    /// An own unit that can go aboard a selected sea transport (task 167).
    Rider(EntityId),
    /// An enemy.
    Enemy(EntityId),
    /// An own construction site.
    Site(EntityId),
    /// An own gather site and the resource gathered there.
    Gather(EntityId, DefId),
    /// A relic to carry, and the tag of the units that may.
    Carry(EntityId, u32),
    /// A city a trader can run a route to.
    Trade(EntityId),
    /// An own airfield, where aircraft land (task 123).
    Airfield(Pos),
}

/// One executor's order from a right click.
pub(super) struct RightOrder {
    /// The order kind.
    pub(super) kind: &'static str,
    /// Its subject (the formation of a group move).
    pub(super) subject: Option<DefId>,
    /// The entity it is given against.
    pub(super) target_entity: Option<EntityId>,
    /// The point it is given at.
    pub(super) target_pos: Option<Pos>,
    /// Whether it queues behind the executor's orders.
    pub(super) queue: bool,
    /// What the pointer's badge shows for it; `None` for an order on the ground.
    pub(super) badge: Option<OrderBadge>,
    /// The unit that takes the order instead of the executor, against the executor: a unit
    /// right-clicked with a sea transport selected boards it (task 167).
    pub(super) rider: Option<EntityId>,
}

impl RightOrder {
    fn at(kind: &'static str, subject: Option<DefId>, at: Pos, queue: bool) -> Self {
        Self {
            kind,
            subject,
            target_entity: None,
            target_pos: Some(at),
            queue,
            badge: None,
            rider: None,
        }
    }

    fn on(kind: &'static str, target: EntityId, queue: bool, badge: OrderBadge) -> Self {
        Self {
            kind,
            subject: None,
            target_entity: Some(target),
            target_pos: None,
            queue,
            badge: Some(badge),
            rider: None,
        }
    }
}

/// Whose `entity` is, as `me` sees it: own, a teammate's, an enemy's or nobody's.
pub(super) fn side_of(world: &World, me: PlayerId, entity: &Entity) -> Side {
    if entity.owner == me {
        Side::Own
    } else if entity.owner == PlayerId(0) {
        Side::Neutral
    } else if Controller::is_enemy(world, me, entity) {
        Side::Enemy
    } else {
        Side::Ally
    }
}

/// The help line for `entity`: "Enemy · Corvette (Ashscale Brood) · 80/100", with a city's
/// name after its kind ("Enemy · City (Ashscale Brood) · Ostia · 900/900").
pub(super) fn describe(world: &World, me: PlayerId, entity: &Entity) -> String {
    let rules = world.rules();
    let side = side_of(world, me, entity);
    let faction = world
        .players()
        .find(|p| p.id == entity.owner)
        .map(|p| p.faction);
    let name = match faction {
        Some(faction) => rules.display_name(entity.type_id, faction),
        None => rules.def(entity.type_id).name.as_str(),
    };
    let mut text = format!("{} · {name}", side.word());
    if let (Side::Ally | Side::Enemy, Some(faction)) = (side, faction) {
        text.push_str(&format!(" ({})", rules.def(faction).name));
    }
    if let Some(city) = world.city_name(entity.id) {
        text.push_str(&format!(" · {city}"));
    }
    match entity.build_progress {
        Some(progress) => text.push_str(&format!(" · being built, {progress} %")),
        None if entity.max_hp > 0 => {
            text.push_str(&format!(" · {}/{}", entity.hp.max(0), entity.max_hp));
        }
        None => {}
    }
    text
}

impl Controller {
    /// What a right click on `entity` names, and the damaged own building builders would mend
    /// there.
    pub(super) fn right_target(
        &self,
        world: &World,
        entity: Option<u32>,
    ) -> (RightTarget, Option<EntityId>) {
        let me = self.me();
        let rules = world.rules();
        // An own airfield is a landing place only for aircraft (task 123).
        let flyers = self
            .own_selected(world)
            .iter()
            .any(|e| world.is_aircraft_type(e.type_id));
        let target =
            entity
                .map(EntityId)
                .and_then(|id| world.entity(id))
                .map_or(RightTarget::Ground, |e| {
                    let et = &rules.entity_types[&e.type_id];
                    if let Some(node) = et
                        .node
                        .as_ref()
                        .filter(|n| n.claim_mode == rom_rules::ClaimMode::Carry)
                        && e.container_id.is_none()
                        && let Some(tag) = node.claim_tag_id
                    {
                        RightTarget::Carry(e.id, tag)
                    } else if Self::is_enemy(world, me, e) {
                        RightTarget::Enemy(e.id)
                    } else if e.owner == me && e.build_progress.is_some() {
                        RightTarget::Site(e.id)
                    } else if e.owner == me
                        && world.is_carrier_type(e.type_id)
                        && (world.is_landed(e.id)
                            || world.orbit(e.id).is_some()
                            || world.is_sea_carrier_type(e.type_id))
                    {
                        RightTarget::Board(e.id)
                    } else if e.owner == me
                        && e.container_id.is_none()
                        && self.own_selected(world).iter().any(|ship| {
                            world.is_sea_carrier_type(ship.type_id)
                                && world.may_ride(ship.type_id, e.type_id)
                        })
                    {
                        RightTarget::Rider(e.id)
                    } else if flyers
                        && e.owner == me
                        && !e.neutralised
                        && world.is_airfield_type(e.type_id)
                    {
                        RightTarget::Airfield(e.pos)
                    } else if e.owner != PlayerId(0)
                        && e.build_progress.is_none()
                        && et.territory.as_ref().is_some_and(|t| t.is_city)
                    {
                        RightTarget::Trade(e.id)
                    } else if let (true, Some(site)) = (e.owner == me, et.gather_site.as_ref()) {
                        RightTarget::Gather(e.id, site.resource_id)
                    } else {
                        RightTarget::Ground
                    }
                });
        // A damaged own building (finished, not struck down) that builders can mend.
        let mend = entity
            .map(EntityId)
            .and_then(|id| world.entity(id))
            .filter(|e| {
                e.owner == me
                    && rules.entity_types[&e.type_id].class == EntityClass::Building
                    && e.build_progress.is_none()
                    && !e.neutralised
                    && e.hp < e.max_hp
            })
            .map(|e| e.id);
        (target, mend)
    }

    /// Who a right click orders: the own selection, and every worker inside a selected gather
    /// building.
    pub(super) fn right_executors(&self, world: &World) -> Vec<EntityId> {
        let mut selected: Vec<EntityId> = self.own_selected(world).iter().map(|e| e.id).collect();
        let inside: Vec<EntityId> = selected
            .iter()
            .flat_map(|id| world.occupants(*id))
            .filter(|id| !selected.contains(id))
            .collect();
        selected.extend(inside);
        selected
    }

    /// The order a right click gives executor `id` against `target` at `at`, if any.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn right_order(
        &self,
        world: &World,
        target: &RightTarget,
        mend: Option<EntityId>,
        id: EntityId,
        at: Pos,
        formation: Option<DefId>,
        queue: bool,
    ) -> Option<RightOrder> {
        let rules = world.rules();
        let e = world.entity(id)?;
        let et = &rules.entity_types[&e.type_id];
        let can_move = et.movement.is_some();
        let armed = !et.weapons.is_empty();
        let orbiting_here = world.orbit(id).is_some_and(|o| o.body == at.body);
        let walk = || {
            let kind = if self.patrol {
                "patrol"
            } else if self.attack_move && armed {
                "attack_move"
            } else {
                "move"
            };
            RightOrder::at(kind, formation, at, queue)
        };
        Some(match target {
            RightTarget::Board(ship)
                if *ship != id
                    && et.class == EntityClass::Unit
                    && !world.is_spacecraft_type(et.id)
                    && world
                        .entity(*ship)
                        .is_some_and(|s| world.may_ride(s.type_id, et.id)) =>
            {
                RightOrder::on("enter", *ship, queue, OrderBadge::Board)
            }
            // A sea transport selected: the unit clicked goes aboard (task 167).
            RightTarget::Rider(unit)
                if world.is_sea_carrier_type(et.id)
                    && world
                        .entity(*unit)
                        .is_some_and(|u| world.may_ride(et.id, u.type_id)) =>
            {
                RightOrder {
                    rider: Some(*unit),
                    ..RightOrder::on("enter", id, queue, OrderBadge::Board)
                }
            }
            // A sea transport with units aboard sent onto land sails there and sets them
            // ashore; on the water it moves (task 167).
            RightTarget::Ground
                if world.is_sea_carrier_type(et.id)
                    && !world.passengers(id).is_empty()
                    && world
                        .terrain_at(at)
                        .is_some_and(|t| rules.terrains[&t].layer == rom_rules::Layer::Ground) =>
            {
                RightOrder::at("unload", None, at, queue)
            }
            // A ship above this planet moves within its orbit to above the clicked point; it
            // comes down only with the Land button.
            RightTarget::Ground if orbiting_here => walk(),
            // A landed carrier sends its passengers to the clicked point.
            RightTarget::Ground if world.is_landed(id) => {
                if world.passengers(id).is_empty() {
                    return None;
                }
                RightOrder::at("unload", None, at, queue)
            }
            // Aircraft sent to an own airfield land on it (task 123).
            RightTarget::Airfield(field)
                if world.is_aircraft_type(et.id) && !self.patrol && !self.attack_move =>
            {
                RightOrder {
                    badge: Some(OrderBadge::Land),
                    ..RightOrder::at("land", None, *field, queue)
                }
            }
            RightTarget::Carry(node, tag)
                if et.class == EntityClass::Unit && rules.has_tag(et.id, *tag) =>
            {
                RightOrder::on("carry", *node, queue, OrderBadge::Carry)
            }
            RightTarget::Trade(city) if et.trader.as_ref().is_some_and(|t| t.runs_routes) => {
                RightOrder::on("trade", *city, queue, OrderBadge::Trade)
            }
            RightTarget::Enemy(foe) if armed && can_move => {
                RightOrder::on("attack", *foe, queue, OrderBadge::Attack)
            }
            RightTarget::Site(site) if et.builder.is_some() => {
                RightOrder::on("repair", *site, queue, OrderBadge::Build)
            }
            // Builders mend a damaged own building for free, a gather building too: they go
            // back to work there by themselves once it is whole (task 140).
            _ if et.builder.is_some()
                && et.movement.is_some()
                && let Some(building) = mend =>
            {
                RightOrder::on("repair", building, queue, OrderBadge::Repair)
            }
            RightTarget::Gather(site, resource)
                if et.harvester.iter().any(|h| h.resource_id == *resource) =>
            {
                RightOrder::on("harvest", *site, queue, OrderBadge::Gather)
            }
            _ if can_move => walk(),
            // A building in the air flies to the clicked point and hovers there (task 170).
            _ if world.airborne(id) => RightOrder::at("fly", None, at, false),
            _ => {
                if !et.producer.as_ref().is_some_and(|p| p.has_rally) {
                    return None;
                }
                // A gather site becomes the rally target: new workers harvest there.
                match target {
                    RightTarget::Gather(site, _) if *site != id => {
                        RightOrder::on("set_rally", *site, false, OrderBadge::Rally)
                    }
                    _ => RightOrder::at("set_rally", None, at, false),
                }
            }
        })
    }

    /// The order a right click on `entity` would give the selection, for the badge beside the
    /// pointer: the first in [`OrderBadge`]'s order that any selected executor would take.
    pub(super) fn right_badge(&self, world: &World, entity: &Entity) -> Option<OrderBadge> {
        if self.placement.is_some() || self.aiming(world) {
            return None;
        }
        let me = self.me();
        if self.view.in_space {
            // In space a right click on a ship attacks it with the armed ships selected.
            let armed = self.selected_ships(world).iter().any(|id| {
                world
                    .entity(*id)
                    .is_some_and(|e| !world.rules().entity_types[&e.type_id].weapons.is_empty())
            });
            return (armed && Self::is_enemy(world, me, entity)).then_some(OrderBadge::Attack);
        }
        let (target, mend) = self.right_target(world, Some(entity.id.0));
        let formation = self.group_formation(world);
        self.right_executors(world)
            .into_iter()
            .filter_map(|id| {
                self.right_order(world, &target, mend, id, entity.pos, formation, false)
            })
            .filter_map(|order| order.badge)
            .min()
    }

    /// The pointer's entry for the HUD: the hovered entity, its side, its line of help and the
    /// order a right click on it would give.
    pub(super) fn pointer_info(&self, world: &World) -> Option<HudPointer> {
        let me = self.me();
        let entity = self.hovered.and_then(|id| present(world, id))?;
        let badge = self.right_badge(world, entity);
        // What a carry or a trade route would bring says more than its verb.
        let hint = badge
            .and_then(|_| self.wealth_help(world))
            .or_else(|| badge.map(|b| format!("Right-click: {}", b.verb())))
            .unwrap_or_default();
        Some(HudPointer {
            entity: entity.id.0,
            side: side_of(world, me, entity),
            badge,
            text: describe(world, me, entity),
            hint,
        })
    }

    /// Every player's side as the person sees them, indexed by player id, for the selection
    /// rings (task 151).
    pub(super) fn sides(&self, world: &World) -> Vec<Side> {
        let me = self.me();
        let team_of = |id: PlayerId| world.players().find(|p| p.id == id).map(|p| p.team);
        let last = world.players().map(|p| p.id.0).max().unwrap_or(0);
        (0..=last)
            .map(|n| {
                let id = PlayerId(n);
                if id == me {
                    Side::Own
                } else if n == 0 || team_of(id).is_none() {
                    Side::Neutral
                } else if team_of(id) == team_of(me) {
                    Side::Ally
                } else {
                    Side::Enemy
                }
            })
            .collect()
    }

    /// Remembers that a right click gave an order against `target`: its ring flashes, red for an
    /// attack, green for a friendly order.
    pub(super) fn flash(&mut self, world: &World, target: EntityId, hostile: bool) {
        let tick = world.tick();
        self.flashes.retain(|f| {
            f.0 != target && tick.saturating_sub(f.2) < u64::from(rom_render::pointer::FLASH_TICKS)
        });
        self.flashes.push((target, hostile, tick));
    }

    /// The flashes still under way, for the HUD.
    pub(super) fn hud_flashes(&self, world: &World) -> Vec<HudFlash> {
        let tick = world.tick();
        self.flashes
            .iter()
            .filter_map(|&(entity, hostile, at)| {
                let age_ticks = u32::try_from(tick.saturating_sub(at)).ok()?;
                (age_ticks < rom_render::pointer::FLASH_TICKS).then_some(HudFlash {
                    entity: entity.0,
                    hostile,
                    age_ticks,
                })
            })
            .collect()
    }
}
