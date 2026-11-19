//! The market manager (task 107): a computer player with a market sells what it holds far
//! more of than it needs and buys what its plans lacked, through the same `buy` and `sell`
//! orders and at the same prices as anyone. The thresholds are profile weights (`market.*`).

use crate::{
    Weights,
    plan::Plan,
    view::{View, operational},
};
use rom_rules::DefId;
use rom_sim::EntityId;

/// The own market that trades this thought: the lowest id complete one.
fn market(view: &View<'_>) -> Option<(EntityId, DefId)> {
    view.own
        .iter()
        .filter(|e| operational(e))
        .find_map(|e| view.world.market_currency(e.id).map(|c| (e.id, c)))
}

/// Sells surplus, then buys toward the largest shortfalls of this thought, at most
/// `market.lots_per_thought` lots and one per resource.
pub(crate) fn trade(plan: &mut Plan<'_>, weights: &Weights) {
    let view = &plan.view;
    if weights.market_lots <= 0 || !view.world.can_trade(view.me) {
        return;
    }
    let Some((market, currency)) = market(view) else {
        return;
    };
    let me = view.me;
    let resources = view.world.tradeable_resources(me);
    let mut lots = weights.market_lots;
    let mut traded: Vec<DefId> = Vec::new();
    // Selling first: the money is then there for what is short.
    for resource in &resources {
        if lots <= 0 {
            return;
        }
        let Some(quote) = plan.view.world.market_quote(me, *resource) else {
            continue;
        };
        let stock = plan.view.purse.get(resource).copied().unwrap_or(0);
        let short = plan.shortfall.get(resource).copied().unwrap_or(0);
        if short == 0
            && stock - quote.lot >= weights.market_sell_above
            && quote.sell * 100 >= quote.base * weights.market_sell_min
            && plan.order(market, "sell", Some(*resource), None, None)
        {
            *plan.view.purse.entry(*resource).or_default() -= quote.lot;
            *plan.view.purse.entry(currency).or_default() += quote.sell;
            traded.push(*resource);
            lots -= 1;
        }
    }
    // Then the largest shortfall first (ties by resource id).
    let mut wanted: Vec<(i64, DefId)> = plan
        .shortfall
        .iter()
        .filter(|(r, missing)| **missing > 0 && resources.contains(r) && !traded.contains(r))
        .map(|(r, missing)| (*missing, *r))
        .collect();
    wanted.sort_by_key(|(missing, r)| (std::cmp::Reverse(*missing), *r));
    let keep = weights.market_reserve + plan.shortfall.get(&currency).copied().unwrap_or(0);
    for (_, resource) in wanted {
        if lots <= 0 {
            return;
        }
        let Some(quote) = plan.view.world.market_quote(me, resource) else {
            continue;
        };
        let money = plan.view.purse.get(&currency).copied().unwrap_or(0);
        if quote.buy * 100 <= quote.base * weights.market_buy_max
            && money - quote.buy >= keep
            && plan.order(market, "buy", Some(resource), None, None)
        {
            *plan.view.purse.entry(currency).or_default() -= quote.buy;
            *plan.view.purse.entry(resource).or_default() += quote.lot;
            lots -= 1;
        }
    }
}
