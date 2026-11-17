//! Automated colonies (task 73): cities outside the city limit whose governor (in `rom-ai`)
//! keeps sites standing in their area through ordinary orders; the switch that pauses a
//! governor; the wealth a colony adds.

use crate::{EntityId, Order, PlayerId, World};
use rom_rules::DefId;

impl World {
    /// Whether a type is an automated colony (`et_colony`).
    pub fn is_colony_type(&self, type_id: DefId) -> bool {
        self.rules
            .entity_types
            .get(&type_id)
            .is_some_and(|et| et.colony.is_some())
    }

    /// Whether a type is a city that counts towards the city limit: every city type but an
    /// automated colony.
    pub fn counts_as_city(&self, type_id: DefId) -> bool {
        self.is_city_type(type_id) && !self.is_colony_type(type_id)
    }

    /// The first type of a building or unit's upgrade line.
    pub fn line_root(&self, mut type_id: DefId) -> DefId {
        for _ in 0..self.rules.entity_types.len() {
            let Some(previous) = self
                .rules
                .entity_types
                .get(&type_id)
                .and_then(|et| et.upgrades_from_id)
            else {
                break;
            };
            type_id = previous;
        }
        type_id
    }

    /// The most buildings of `subject`'s upgrade line a colony of `colony_type` keeps in its
    /// area (`et_colony_site.cap`), or None when its governor does not keep that line.
    pub fn colony_cap(&self, colony_type: DefId, subject: DefId) -> Option<i64> {
        let colony = self.rules.entity_types.get(&colony_type)?.colony.as_ref()?;
        let root = self.line_root(subject);
        colony
            .sites
            .iter()
            .find(|site| self.line_root(site.site_type_id) == root)
            .map(|site| site.cap)
    }

    /// Whether the owner has paused a colony's governor.
    pub fn governor_paused(&self, colony: EntityId) -> bool {
        self.governors_paused.contains(&colony)
    }

    /// The own buildings whose city is `colony` (sites included), in id order.
    pub fn colony_buildings(&self, colony: EntityId) -> Vec<EntityId> {
        let Some(owner) = self.entity(colony).map(|e| e.owner) else {
            return Vec::new();
        };
        self.entities()
            .filter(|e| e.city_id == Some(colony) && e.owner == owner)
            .map(|e| e.id)
            .collect()
    }

    /// Applies `pause_governor` or `resume_governor` at once; the colony's queue stays.
    pub(crate) fn apply_governor(&mut self, order: &Order) {
        if order.kind == "pause_governor" {
            self.governors_paused.insert(order.executor);
        } else {
            self.governors_paused.remove(&order.executor);
        }
        self.stats.orders_done += 1;
    }

    /// The wealth term a complete site in a colony's area adds: the colony's share of the
    /// site's own income, while the colony stands complete and active and has the same owner.
    pub(crate) fn colony_share(&self, site: EntityId) -> Option<((PlayerId, DefId), i64)> {
        let entity = self.entity(site)?;
        let own = self.rules.entity_types[&entity.type_id]
            .gather_site
            .as_ref()?
            .self_income;
        let city = self.entity(entity.city_id?)?;
        let colony = self.rules.entity_types[&city.type_id].colony.as_ref()?;
        let share = (i128::from(own) * i128::from(colony.site_share_pct) / 100) as i64;
        (share != 0
            && city.owner == entity.owner
            && city.build_progress.is_none()
            && !city.neutralised
            && city.assimilation_until.is_none())
        .then_some(((entity.owner, colony.resource_id), share))
    }

    /// Refreshes the income of every building of a colony's area, whose share follows the
    /// colony's state.
    pub(crate) fn refresh_colony_sites(&mut self, colony: EntityId) {
        if !self
            .entity(colony)
            .is_some_and(|e| self.is_colony_type(e.type_id))
        {
            return;
        }
        let sites: Vec<EntityId> = self
            .entities()
            .filter(|e| e.city_id == Some(colony))
            .map(|e| e.id)
            .collect();
        for site in sites {
            self.refresh_income(site);
        }
    }
}
