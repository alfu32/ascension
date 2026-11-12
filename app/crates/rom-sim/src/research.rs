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
            .all_orders()
            .filter(|o| o.player == player && o.kind == "research")
            .filter_map(|o| o.subject)
            .collect();
        if queued.contains(&subject) {
            return Err(SimError::ResearchQueued);
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
                        let old_hp = self.rules.entity_types[&entity.type_id].hp.max(1);
                        let new_hp = self.rules.entity_types[&target].hp;
                        entity.hp = (i128::from(entity.hp) * i128::from(new_hp)
                            / i128::from(old_hp)) as i64;
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
    }

    pub(crate) fn production_duration(
        &self,
        executor: EntityId,
        subject: DefId,
        research: bool,
    ) -> u64 {
        let sources =
            self.players[&self.entity(executor).expect("executor").owner].active_sources();
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
            assert_eq!(upgraded.hp, 27); // 10/40 of 110, rounded down.
            assert_eq!(upgraded.id, unit);
            assert_eq!(w.entity(foreign).unwrap().type_id, citizen);
            assert!(w.entity(EntityId(999)).is_none());
            w.hash()
        };
        assert_eq!(run(), run());
    }
}
