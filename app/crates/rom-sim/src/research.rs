//! Research exclusion checks and completed technology effects.

use crate::{EntityId, PlayerId, SimError, World};
use rom_rules::{DefId, TechEffectKind};

impl World {
    pub(crate) fn check_research(&self, player: PlayerId, subject: DefId) -> Result<(), SimError> {
        let tech = &self.rules.techs[&subject];
        let researched = &self.players[&player].techs;
        if !tech.repeatable && researched.get(&subject).is_some_and(|times| *times > 0) {
            return Err(SimError::AlreadyResearched);
        }
        let queued: Vec<_> = self
            .production_orders()
            .filter(|o| o.player == player && o.kind == "research")
            .filter_map(|o| o.subject)
            .collect();
        if queued.contains(&subject) {
            return Err(SimError::ResearchQueued);
        }
        // The ascension waits until everything else at the library is researched (task 76).
        if self.rules.ascends(subject) && !self.ascension_ready(player) {
            return Err(SimError::Locked);
        }
        if !tech.exclusive_group.is_empty()
            && researched
                .iter()
                .filter(|(_, times)| **times > 0)
                .map(|(id, _)| id)
                .chain(queued.iter())
                .any(|id| {
                    *id != subject && self.rules.techs[id].exclusive_group == tech.exclusive_group
                })
        {
            return Err(SimError::ExclusiveResearch);
        }
        Ok(())
    }

    /// Test setups (`research-all` in a script): before the first tick, completes for `player`
    /// every technology it could research in its current age, with their effects, until none
    /// is left: each one researched somewhere, available to its faction and unlocked, except
    /// ascensions, handicaps, a second member of an exclusive group, and age advances beyond
    /// its age. The player keeps its age. Returns how many were completed.
    pub fn complete_research_for_setup(&mut self, player: PlayerId) -> Result<usize, SimError> {
        if self.tick != 0 {
            return Err(SimError::Locked);
        }
        let age = self
            .players
            .get(&player)
            .ok_or(SimError::UnknownPlayer(player))?
            .age;
        let rules = self.rules.clone();
        let reached = rules.ages[&age].ordinal;
        let mut done = 0;
        loop {
            let p = &self.players[&player];
            let sources = p.active_sources();
            let owned = self.owned_types(player);
            let groups: std::collections::BTreeSet<&str> = p
                .techs
                .iter()
                .filter(|(_, n)| **n > 0)
                .map(|(id, _)| rules.techs[id].exclusive_group.as_str())
                .filter(|g| !g.is_empty())
                .collect();
            let next = rules.techs.iter().find_map(|(id, tech)| {
                let beyond = rules.tech_effects.get(id).into_iter().flatten().any(|e| {
                    e.effect == TechEffectKind::SetAge
                        && e.target_def_id
                            .is_none_or(|a| rules.ages[&a].ordinal > reached)
                });
                let researched_somewhere = rules
                    .production_by_product
                    .get(id)
                    .into_iter()
                    .flatten()
                    .any(|row| row.mode == rom_rules::ProductionMode::Research);
                (researched_somewhere
                    && !beyond
                    && !rules.ascends(*id)
                    && tech.kind != rom_rules::TechKind::Handicap
                    && p.techs.get(id).is_none_or(|n| *n == 0)
                    && (tech.exclusive_group.is_empty()
                        || !groups.contains(tech.exclusive_group.as_str()))
                    && rules.is_available(*id, p.faction)
                    && rules.is_unlocked(*id, &sources, &owned))
                .then_some(*id)
            });
            let Some(tech) = next else {
                break;
            };
            self.complete_research(player, tech);
            self.players.get_mut(&player).expect("checked player").age = age;
            done += 1;
        }
        Ok(done)
    }

    pub(crate) fn complete_research(&mut self, player: PlayerId, subject: DefId) {
        *self
            .players
            .get_mut(&player)
            .expect("checked player")
            .techs
            .entry(subject)
            .or_default() += 1;
        let rules = self.rules.clone();
        for effect in rules.tech_effects.get(&subject).into_iter().flatten() {
            if effect.effect == TechEffectKind::Ascend {
                self.ascension.completing.push(player);
                continue;
            }
            let Some(target) = effect.target_def_id else {
                continue;
            };
            match effect.effect {
                TechEffectKind::SetAge => {
                    self.players.get_mut(&player).expect("owner").age = target
                }
                TechEffectKind::ChangeFaction => {
                    self.players.get_mut(&player).expect("owner").faction = target
                }
                TechEffectKind::UpgradeType => {
                    let mut changed = Vec::new();
                    for entity in self
                        .entities
                        .iter_mut()
                        .flatten()
                        .filter(|e| e.owner == player && Some(e.type_id) == effect.from_def_id)
                    {
                        changed.push((entity.id, entity.type_id));
                        entity.type_id = target;
                    }
                    for (id, old) in changed {
                        self.retyped(id, old);
                    }
                }
                _ => {}
            }
        }
        let ids: Vec<_> = self
            .entities()
            .filter(|e| e.owner == player)
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.refresh_entity_stats(id);
        }
    }

    pub(crate) fn production_duration(
        &self,
        executor: EntityId,
        subject: DefId,
        research: bool,
    ) -> u64 {
        let sources = self.active_sources(self.entity(executor).expect("executor").owner);
        let owner = self.entity(executor).expect("executor").owner;
        if research && self.rules.ascends(subject) {
            return self.ascension_duration(owner, subject);
        }
        let ticks = if research {
            self.rules.player_stat(
                "research_time",
                self.rules.techs[&subject].research_ticks,
                &sources,
            )
        } else {
            self.rules.build_ticks(subject, &sources)
        };
        ticks.max(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use crate::{Command, EntityId, PlayerId, Pos, World};
    use rom_rules::{Rules, build_rules_db};
    use std::sync::Arc;

    #[test]
    fn upgrade_type_converts_existing_units_keeping_hp_pct() {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        conn.execute_batch("INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id)
            SELECT t.id,'upgrade_type',f.id,x.id FROM def t,def f,def x
            WHERE t.key='tech.age.classical' AND f.key='unit.citizen' AND x.key='unit.spearman';
            UPDATE tech SET research_ticks=1 WHERE id=(SELECT id FROM def WHERE key='tech.age.classical');").unwrap();
        conn.execute_batch("DELETE FROM prerequisite WHERE def_id=(SELECT id FROM def WHERE key='tech.age.classical');").unwrap();
        let rules = Arc::new(Rules::load(&conn).unwrap());
        let run = || {
            let mut w = World::movement_scenario(rules.clone(), 7, 0).unwrap();
            let citizen = rules.id_of("unit.citizen").unwrap();
            let spear = rules.id_of("unit.spearman").unwrap();
            let library = w
                .spawn(
                    rules.id_of("bld.library").unwrap(),
                    PlayerId(1),
                    Pos {
                        x: 10_000,
                        y: 10_000,
                        face: 0,
                        ..Pos::default()
                    },
                )
                .unwrap();
            let unit = w.spawn(citizen, PlayerId(1), Pos::default()).unwrap();
            w.entities[unit.0 as usize].as_mut().unwrap().hp = 10;
            let mut enemy = w.players[&PlayerId(1)].clone();
            enemy.id = PlayerId(2);
            w.players.insert(enemy.id, enemy);
            let foreign = w.spawn(citizen, PlayerId(2), Pos::default()).unwrap();
            w.grant(PlayerId(1), rules.id_of("res.food").unwrap(), 250)
                .unwrap();
            w.submit(Command {
                player: PlayerId(1),
                executor: library,
                kind: "research".into(),
                subject: rules.id_of("tech.age.classical"),
                target_entity: None,
                target_pos: None,
                queue: true,
            })
            .unwrap();
            w.step();
            let upgraded = w.entity(unit).unwrap();
            assert_eq!(upgraded.type_id, spear);
            assert_eq!(upgraded.hp, 25); // 10/40 of 100.
            assert_eq!(upgraded.id, unit);
            assert_eq!(w.entity(foreign).unwrap().type_id, citizen);
            assert!(w.entity(EntityId(999)).is_none());
            w.hash()
        };
        assert_eq!(run(), run());
    }

    /// `research-all` (task 84): everything up to the player's age is researched, nothing
    /// beyond it, the age is kept, and the same setup gives the same state twice.
    #[test]
    fn research_for_setup_completes_everything_up_to_the_age_and_no_further() {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let rules = Arc::new(Rules::load(&conn).unwrap());
        let industrial = rules.id_of("age.industrial").unwrap();
        let reached = rules.ages[&industrial].ordinal;
        let run = || {
            let mut w = World::movement_scenario(rules.clone(), 7, 0).unwrap();
            w.players.get_mut(&PlayerId(1)).unwrap().age = industrial;
            let done = w.complete_research_for_setup(PlayerId(1)).unwrap();
            assert!(done > 0);
            assert_eq!(w.complete_research_for_setup(PlayerId(1)).unwrap(), 0);
            let p = &w.players[&PlayerId(1)];
            assert_eq!(p.age, industrial);
            let sources = p.active_sources();
            for (id, tech) in &rules.techs {
                let age_of = |a: &crate::DefId| rules.ages.get(a).map(|a| a.ordinal);
                let needs_later_age = rules
                    .prerequisites
                    .get(id)
                    .into_iter()
                    .flatten()
                    .filter_map(|r| age_of(&r.requires_def_id))
                    .any(|o| o > reached);
                let advance_beyond = rules.tech_effects.get(id).into_iter().flatten().any(|e| {
                    e.effect == rom_rules::TechEffectKind::SetAge
                        && e.target_def_id.and_then(|a| age_of(&a)) > Some(reached)
                });
                let has = p.techs.get(id).is_some_and(|n| *n > 0);
                if needs_later_age || advance_beyond || rules.ascends(*id) {
                    assert!(!has, "{} is beyond the industrial age", id.0);
                } else if tech.kind != rom_rules::TechKind::Handicap
                    && rules.is_unlocked(*id, &sources, &Default::default())
                {
                    assert!(has, "{} was left out", id.0);
                }
            }
            w.hash()
        };
        assert_eq!(run(), run());
        let mut w = World::movement_scenario(rules.clone(), 7, 0).unwrap();
        w.step();
        assert!(w.complete_research_for_setup(PlayerId(1)).is_err());
    }
}
