//! The opening book: the profile's ordered steps, followed until they run out or a city
//! is attacked.
//!
//! A step asks to *have* a number of something, counting what is queued. A blocking step
//! waits for resources; a step that cannot be done for another reason (no producer yet, a
//! missing prerequisite, no site) is waited for a short while and then skipped.

use crate::{
    memory::Memory,
    plan::{Act, Outcome, Wish},
    view::View,
};
use rom_rules::{AiOpeningStep, OpeningAction};

/// Ticks an opening step may wait for something other than resources.
pub(crate) const PATIENCE_TICKS: u64 = 1200;
/// Score of the current opening step: above every planner wish.
const SCORE: i64 = 2000;

/// Whether a step already holds, counting queued work.
fn satisfied(view: &View<'_>, step: &AiOpeningStep) -> bool {
    let Some(subject) = step.subject_def_id else {
        // A step without a subject can only be the founding of cities.
        return step.action != OpeningAction::FoundCity || view.cities.len() as i64 >= step.count;
    };
    match step.action {
        OpeningAction::Train => view.count(subject) >= step.count,
        OpeningAction::Construct => view.line_count(subject) >= step.count,
        OpeningAction::Research => view.researched(subject) || view.researching.contains(&subject),
        OpeningAction::FoundCity => view.cities.len() as i64 >= step.count,
        // Recorded as a floor for the economy manager, then done.
        OpeningAction::AssignWorkers => true,
        // Exploring belongs to the military part of the computer player.
        OpeningAction::Scout => true,
    }
}

/// Advances past every step that already holds and returns the wish of the current one.
pub(crate) fn current(
    view: &View<'_>,
    steps: &[AiOpeningStep],
    memory: &mut Memory,
) -> Option<Wish> {
    while !memory.opening_done {
        let Some(step) = steps.get(memory.opening_seq as usize) else {
            memory.opening_done = true;
            break;
        };
        if step.action == OpeningAction::AssignWorkers
            && let Some(resource) = step.subject_def_id
        {
            memory.floors.insert(resource.0, step.count);
        }
        if satisfied(view, step) {
            memory.opening_seq += 1;
            memory.opening_since = view.tick;
            continue;
        }
        let act = match (step.action, step.subject_def_id) {
            (OpeningAction::Train, Some(subject)) => Act::Train(subject),
            (OpeningAction::Construct, Some(subject)) => Act::Construct(subject),
            (OpeningAction::Research, Some(subject)) => Act::Research(subject),
            (OpeningAction::FoundCity, subject) => {
                match subject.or_else(|| view.world.city_type(view.me)) {
                    Some(city) => Act::Construct(city),
                    None => {
                        memory.opening_seq += 1;
                        continue;
                    }
                }
            }
            _ => {
                memory.opening_seq += 1;
                continue;
            }
        };
        return Some(Wish {
            score: SCORE,
            act,
            reserve: step.blocking,
            opening: true,
        });
    }
    None
}

/// Moves on from a step that could not be done: at once when it was only skippable and
/// too dear, after a while when something other than resources stands in the way.
pub(crate) fn settle(steps: &[AiOpeningStep], memory: &mut Memory, outcome: Outcome, tick: u64) {
    let Some(step) = steps.get(memory.opening_seq as usize) else {
        return;
    };
    let skip = match outcome {
        Outcome::Ordered => false,
        Outcome::Unaffordable => !step.blocking,
        Outcome::Impossible => tick.saturating_sub(memory.opening_since) >= PATIENCE_TICKS,
    };
    if skip {
        memory.opening_seq += 1;
    }
    if skip || outcome == Outcome::Ordered {
        memory.opening_since = tick;
    }
}
