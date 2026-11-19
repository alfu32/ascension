//! Market trading (task 107): a player with a completed market buys and sells lots of the
//! tradeable resources for the market's currency. Prices are per player: each lot bought
//! raises the price of its resource, each lot sold lowers it, and the price drifts back to
//! rest over time. Buying costs half the spread above the price and selling pays half the
//! spread below it, so selling back what was just bought always loses.
//!
//! The state is one integer per player and resource, its `pressure`, in hundredths of a lot:
//! the price is `base × (10 000 + step_pct × pressure) / 10 000`.

use crate::{Command, Order, PlayerId, SimError, World};
use rom_rules::DefId;

/// The `tech_effect.rule_key` that lets a player trade at its markets.
pub const RULE_MARKET_TRADING: &str = "market_trading";

/// Pressure of one lot, in hundredths of a lot.
const LOT: i64 = 100;

/// An amount of one resource.
type Amount = (DefId, i64);

/// The market constants (`market.*`), read once from the rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MarketRules {
    /// Units in one lot.
    pub lot: i64,
    /// Price change per lot, in percent of the base price.
    pub step_pct: i64,
    /// Spread before `market_spread` modifiers, percent.
    pub spread_base: i64,
    /// Least spread, percent.
    pub spread_min: i64,
    /// Pressure bounds, hundredths of a lot, from the lowest and highest price.
    pub floor: i64,
    pub ceiling: i64,
    /// Ticks between drifts.
    pub drift_period: u64,
    /// Pressure taken back toward 0 each drift, hundredths of a lot.
    pub drift: i64,
}

impl MarketRules {
    /// The constants, or `None` when one is missing or out of range (then nothing trades).
    pub(crate) fn of(rules: &rom_rules::Rules) -> Option<Self> {
        let get = |key| rules.constant(key);
        let (lot, step_pct) = (get("market.lot_size")?, get("market.price_step_pct")?);
        let (spread_base, spread_min) = (
            get("market.spread_pct.base")?,
            get("market.spread_pct.min")?,
        );
        let (min_pct, max_pct) = (get("market.price_min_pct")?, get("market.price_max_pct")?);
        let (period, drift) = (
            get("market.drift_period_ticks")?,
            get("market.drift_hundredths")?,
        );
        let valid = lot > 0
            && step_pct >= 0
            && (0..200).contains(&spread_min)
            && spread_base >= 0
            && (1..=100).contains(&min_pct)
            && max_pct >= 100
            && period > 0
            && drift >= 0;
        if !valid {
            return None;
        }
        // The pressure at which the price reaches its bounds; none without a step.
        let bound = |pct: i64| {
            if step_pct == 0 {
                0
            } else {
                (pct - 100) * LOT / step_pct
            }
        };
        Some(Self {
            lot,
            step_pct,
            spread_base,
            spread_min,
            floor: bound(min_pct),
            ceiling: bound(max_pct),
            drift_period: period as u64,
            drift,
        })
    }

    /// The price of a lot at `pressure`, before the spread.
    fn price(&self, base: i64, pressure: i64) -> i64 {
        let pressure = pressure.clamp(self.floor, self.ceiling);
        base * (100 * LOT + self.step_pct * pressure) / (100 * LOT)
    }
}

/// What one lot of a resource costs and pays at a player's markets now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketQuote {
    /// The resource traded.
    pub resource: DefId,
    /// Units in one lot.
    pub lot: i64,
    /// The price of a lot before the spread.
    pub price: i64,
    /// The price at rest (`resource.market_base_price`).
    pub base: i64,
    /// What buying one lot costs, in the market's currency.
    pub buy: i64,
    /// What selling one lot pays, in the market's currency.
    pub sell: i64,
    /// The spread in percent: buying costs half of it above the price, selling pays half of
    /// it below.
    pub spread_pct: i64,
}

impl World {
    /// Whether `player` may trade at its markets (`market_trading` enabled by a research).
    pub fn can_trade(&self, player: PlayerId) -> bool {
        self.market_rules.is_some() && self.rule_enabled(player, RULE_MARKET_TRADING)
    }

    /// The currency a market entity trades in, if it is a market.
    pub fn market_currency(&self, market: crate::EntityId) -> Option<DefId> {
        let entity = self.entity(market)?;
        self.rules.entity_types[&entity.type_id]
            .market
            .as_ref()
            .map(|m| m.currency_id)
    }

    /// The resources `player` can trade now, in id order: tradeable ones its age has
    /// discovered.
    pub fn tradeable_resources(&self, player: PlayerId) -> Vec<DefId> {
        let Some(age) = self
            .players
            .get(&player)
            .and_then(|p| self.rules.ages.get(&p.age))
            .map(|a| a.ordinal)
        else {
            return Vec::new();
        };
        self.rules
            .resources
            .values()
            .filter(|r| {
                r.tradeable
                    && r.first_age_id
                        .and_then(|a| self.rules.ages.get(&a))
                        .is_none_or(|a| a.ordinal <= age)
            })
            .map(|r| r.id)
            .collect()
    }

    /// A resource's market pressure for `player`, in hundredths of a lot: positive after
    /// buying, negative after selling, 0 at rest.
    pub fn market_pressure(&self, player: PlayerId, resource: DefId) -> i64 {
        self.market.get(&(player, resource)).copied().unwrap_or(0)
    }

    /// Every market pressure that is not 0, by player and resource.
    pub fn market_pressures(&self) -> impl Iterator<Item = ((PlayerId, DefId), i64)> + '_ {
        self.market.iter().map(|(key, value)| (*key, *value))
    }

    /// The spread `player` trades at, percent: `market.spread_pct.base` after its
    /// `market_spread` modifiers, at least `market.spread_pct.min`.
    pub fn market_spread(&self, player: PlayerId) -> Option<i64> {
        let rules = self.market_rules?;
        let sources = self.active_sources(player);
        let spread = self
            .rules
            .player_stat("market_spread", rules.spread_base, &sources);
        Some(spread.clamp(rules.spread_min, 198))
    }

    /// What a lot of `resource` costs and pays for `player` now; `None` for a resource that
    /// is not tradeable or rules without a market.
    pub fn market_quote(&self, player: PlayerId, resource: DefId) -> Option<MarketQuote> {
        let rules = self.market_rules?;
        let row = self
            .rules
            .resources
            .get(&resource)
            .filter(|r| r.tradeable)?;
        let spread = self.market_spread(player)?;
        let base = row.market_base_price.max(1);
        let pressure = self.market_pressure(player, resource);
        let price = rules.price(base, pressure);
        // Buying pays at the pressure before the lot, selling at the pressure after it: a lot
        // sold straight back meets the same price, less the whole spread.
        let after_sale = (pressure - LOT).max(rules.floor);
        let buy = (price * (200 + spread) + 199) / 200;
        let sell = rules.price(base, after_sale) * (200 - spread) / 200;
        Some(MarketQuote {
            resource,
            lot: rules.lot,
            price,
            base,
            buy,
            sell: sell.max(0),
            spread_pct: spread,
        })
    }

    /// Checks `buy` and `sell` at submission: a completed own market, trading enabled, a
    /// tradeable resource the player's age has discovered, and the payment or the goods.
    pub(crate) fn validate_trade_order(&self, command: &Command) -> Result<(), SimError> {
        let buying = match command.kind.as_str() {
            "buy" => true,
            "sell" => false,
            _ => return Ok(()),
        };
        let resource = command.subject.ok_or(SimError::WrongTargetShape)?;
        self.trade_terms(command.player, command.executor, resource, buying)
            .map(|_| ())
    }

    /// The resource and amount paid out and taken in by one lot, or why it cannot trade.
    fn trade_terms(
        &self,
        player: PlayerId,
        market: crate::EntityId,
        resource: DefId,
        buying: bool,
    ) -> Result<(Amount, Amount), SimError> {
        let entity = self
            .entity(market)
            .filter(|e| e.owner == player)
            .ok_or(SimError::UnknownExecutor(market))?;
        if entity.build_progress.is_some() {
            return Err(SimError::IncompleteExecutor);
        }
        if entity.neutralised || entity.assimilation_until.is_some() {
            return Err(SimError::InactiveExecutor);
        }
        let currency = self
            .market_currency(market)
            .ok_or_else(|| SimError::MissingComponent("market".into()))?;
        if !self.can_trade(player) {
            return Err(SimError::NeedsMarketTrading);
        }
        if resource == currency || !self.tradeable_resources(player).contains(&resource) {
            return Err(SimError::NotTradeable);
        }
        let quote = self
            .market_quote(player, resource)
            .ok_or(SimError::NotTradeable)?;
        let stock = |id| {
            self.players[&player]
                .resources
                .get(&id)
                .copied()
                .unwrap_or(0)
        };
        let (paid, got) = if buying {
            ((currency, quote.buy), (resource, quote.lot))
        } else {
            ((resource, quote.lot), (currency, quote.sell))
        };
        if stock(paid.0) < paid.1 {
            return Err(SimError::InsufficientResources);
        }
        Ok((paid, got))
    }

    /// Makes one trade as `buy` or `sell` leaves `pending`, at the prices of that moment; it
    /// fails, reported, when the market or the money is gone by then.
    pub(crate) fn apply_trade(&mut self, order: &Order) {
        let buying = order.kind == "buy";
        let terms = order
            .subject
            .ok_or(SimError::WrongTargetShape)
            .and_then(|r| {
                self.trade_terms(order.player, order.executor, r, buying)
                    .map(|t| (r, t))
            });
        let (resource, (paid, got)) = match terms {
            Ok(terms) => terms,
            Err(error) => {
                let reason = match error {
                    SimError::InsufficientResources => "cannot_afford",
                    _ => "cannot_trade",
                };
                self.record_failure(order, reason);
                self.stats.orders_failed += 1;
                return;
            }
        };
        let player = self.players.get_mut(&order.player).expect("checked owner");
        *player.resources.entry(paid.0).or_default() -= paid.1;
        *player.resources.entry(got.0).or_default() += got.1;
        let rules = self.market_rules.expect("checked market rules");
        let step = if buying { LOT } else { -LOT };
        let key = (order.player, resource);
        let pressure =
            (self.market_pressure(order.player, resource) + step).clamp(rules.floor, rules.ceiling);
        if pressure == 0 {
            self.market.remove(&key);
        } else {
            self.market.insert(key, pressure);
        }
        self.stats.orders_done += 1;
    }

    /// Once every `market.drift_period_ticks` (before that tick's trades), each price away
    /// from rest drifts back by `market.drift_hundredths` of a lot.
    pub(crate) fn drift_market(&mut self) {
        let Some(rules) = self.market_rules else {
            return;
        };
        if self.market.is_empty() || !self.tick.is_multiple_of(rules.drift_period) {
            return;
        }
        self.market.retain(|_, pressure| {
            *pressure -= pressure.signum() * rules.drift.min(pressure.abs());
            *pressure != 0
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> MarketRules {
        MarketRules {
            lot: 100,
            step_pct: 4,
            spread_base: 30,
            spread_min: 2,
            floor: -1750,
            ceiling: 5000,
            drift_period: 20,
            drift: 2,
        }
    }

    #[test]
    fn price_steps_with_pressure_and_stays_within_bounds() {
        let r = rules();
        assert_eq!(r.price(100, 0), 100);
        assert_eq!(r.price(100, 100), 104);
        assert_eq!(r.price(100, -100), 96);
        assert_eq!(r.price(130, 250), 143);
        assert_eq!(r.price(100, 1_000_000), 300);
        assert_eq!(r.price(100, -1_000_000), 30);
    }
}
