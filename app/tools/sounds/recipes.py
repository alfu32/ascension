"""Recipes for each unit's and building's own sounds (see assign.py and plan.json).

A recipe takes a seeded random generator `r` and the parameters assign.py derived from the
type's data (`p`: age, size, pitch, power, reach, speed, figures, active …) and returns mono
samples at 44.1 kHz. Loops ("move.*", "active.*") are periodic by construction.
"""
import numpy as np

import synth as s
from synth import SR, n_of, place, place_wrap, cnoise, wander, qf, phase_of, env_ad, burst, modal

LOOP_MOVE = 4.0
LOOP_ACTIVE = 8.0


def seed_of(r):
    return int(r.integers(1_000_000))


def bright_of(p):
    """Older weapons and machines sound duller; later ones brighter."""
    return min(1.0, 0.5 + 0.04 * p.get("age", 0))


def voiced(x, pitch=1.0, bright=1.0, loop=False):
    """Give a recipe's result the type's own pitch and brightness."""
    if abs(pitch - 1.0) > 1e-3:
        n = len(x)
        m = max(2, int(round(n / pitch)))
        pos = np.arange(m) * (n / m)
        x = np.interp(pos, np.arange(n), x, period=n) if loop else np.interp(pos, np.arange(n), x)
    if bright < 0.999:
        cutoff = 2500 + 15000 * bright ** 2
        x = s.cfilter(x, hi=cutoff) if loop else s.lowpass(x, cutoff)
    return x


def gentle(x):
    """Not too violent (owner's review): no harsh highs, a 3 ms softened attack."""
    x = s.lowpass(x, 7000)
    n = min(len(x), n_of(0.003))
    x[:n] *= np.linspace(0, 1, n)
    return x


# ------------------------------------------------------------------ firing

def fire(name):
    def make(r, p):
        x = FIRE[name](r, p)
        x = voiced(x, p["pitch"] * (1.08 - 0.16 * p.get("power", 0.5)), bright_of(p))
        return gentle(x)
    return make


def _rifle(r, p):
    n = n_of(0.9 + 0.6 * p.get("reach", 0.3))
    crack = s.highpass(r.standard_normal(n), 1500) * env_ad(n, 0.0003, 0.006) * (1.2 + 0.5 * p["power"])
    body = s.lowpass(r.standard_normal(n), 700 + 50 * p["age"]) * env_ad(n, 0.001, 0.04 + 0.03 * p["power"]) * 1.2
    tail = s.lowpass(r.standard_normal(n), 1800) * env_ad(n, 0.03, 0.2 + 0.3 * p.get("reach", 0.3)) * 0.12
    return crack + body + tail


def _machine_gun(r, p):
    x = s.machine_gun(r)
    return x


def _cannon(r, p):
    return s.boom(r, 2.5 + p["power"], 1.0, 1500 + 400 * r.uniform(), 0.1 + 0.05 * p["power"],
                  40 + 20 * r.uniform(), 0.7 + 0.6 * p["power"], 0.5 + 0.3 * p.get("reach", 0.5))


def _artillery(r, p):
    return s.artillery(r) if p["power"] > 0.6 else s.boom(r, 3.0, 1.0, 1200, 0.12, 45, 1.0, 0.8)


def _missile(r, p):
    return s.missile_launch(seed_of(r))


def _catapult(r, p):
    return s.catapult(seed_of(r))


def _drone_strike(r, p):
    """A kamikaze drone (task 120): the whine of a small rotor diving, falling in pitch as it
    nears, then a small, dull blast; short and not violent."""
    dive = 0.45
    n = n_of(dive + 0.9)
    t = np.arange(n) / SR
    f = np.where(t < dive, 980 - 420 * (t / dive) ** 1.5, 560.0)
    rise = np.clip(t / dive, 0, 1) ** 2 * (t < dive)
    whine = (np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.3 * np.sin(4 * np.pi * np.cumsum(f) / SR)) * rise * 0.18
    air = s.bandpass(r.standard_normal(n), 600, 3000) * rise * 0.12
    blast = np.zeros(n)
    place(blast, s.boom(r, 0.85, 0.35, 900, 0.06, 70, 0.25, 0.2) * 0.8, dive)
    return whine + air + blast


FIRE = {
    "bow": lambda r, p: s.bow(r), "crossbow": lambda r, p: s.crossbow(r), "sling": lambda r, p: s.sling(r),
    "catapult": _catapult, "musket": lambda r, p: s.musket(r), "rifle": _rifle, "machine_gun": _machine_gun,
    "autocannon": lambda r, p: s.autocannon(r), "cannon": _cannon, "tank_gun": lambda r, p: s.tank_gun(r),
    "artillery": _artillery, "flak": lambda r, p: s.flak(r), "bomb_drop": lambda r, p: s.bomb_drop(r),
    "rocket": lambda r, p: s.rocket(r), "missile": _missile, "laser": lambda r, p: s.laser(r),
    "plasma": lambda r, p: s.plasma(r), "railgun": lambda r, p: s.railgun(r),
    "drone_strike": _drone_strike,
}

FIGHT = {
    "sword_clash": s.sword_clash, "spear_thrust": s.spear_thrust, "club_blow": s.club_blow,
    "cavalry_charge": s.cavalry_charge, "bayonet": s.bayonet, "energy_blade": s.energy_blade,
    "power_fist": s.power_fist,
}


def fight(name):
    def make(r, p):
        return gentle(voiced(FIGHT[name](r), p["pitch"], 1.0))
    return make


# ------------------------------------------------------------------ moving (loops)

def footsteps(r, p):
    """Soft steps on grass: a light, muffled tread with a little rustle, never quite regular."""
    L = 2.0
    out = np.zeros(n_of(L))
    cadence = 1.6 + 1.4 * p["speed"]
    steps = max(2, round(cadence * L))
    for f in range(min(4, max(1, p.get("figures", 1)))):
        offset = r.uniform(0, L / steps)
        gain = 1.0 if f == 0 else r.uniform(0.4, 0.7)
        for k in range(steps):
            place_wrap(out, soft_step(r) * gain * r.uniform(0.7, 1.0), offset + k * L / steps + r.uniform(-0.04, 0.04))
            if p["age"] >= 3 and r.uniform() < 0.3:  # kit and equipment, faintly
                place_wrap(out, burst(r, 0.06, 2000, 6000, 0.005, 0.015, 0.03 * gain), offset + k * L / steps + 0.03)
    if p.get("swarm"):
        out += swarm_hum(r, L)
    return out


def swarm_hum(r, seconds):
    """A drone cloud overhead (task 120): a faint, many-voiced whir of small rotors, each a
    little apart in pitch, slowly beating; periodic over `seconds`."""
    n = n_of(seconds)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for k in range(6):
        f = qf(r.uniform(150, 230), seconds)
        out += (np.sin(2 * np.pi * f * t + r.uniform(0, 6)) + 0.3 * np.sin(4 * np.pi * f * t)) * r.uniform(0.6, 1.0)
    out *= 0.012 * (0.85 + 0.15 * wander(r, n, 4))
    return out + cnoise(r, n, 400, 2500, color=1) * 0.012


def soft_step(r):
    """One footfall on grass: a muffled heel and a little rustle."""
    m = n_of(0.25)
    heel = s.lowpass(r.standard_normal(m), 160) * env_ad(m, 0.012, 0.035) * 1.2
    grass = s.bandpass(r.standard_normal(m), 900, 4500) * env_ad(m, 0.02, 0.06) * r.uniform(0.08, 0.16)
    return heel + grass


def hoofbeats(r, p):
    strides = 3 + round(3 * p["speed"])
    return s.hoofbeats(seed_of(r), strides * 0.6) if strides else s.hoofbeats(seed_of(r))


def wheels(r, p):
    return s.wheels_earth(seed_of(r), LOOP_MOVE)


def engine_wheeled(r, p):
    return s.engine(seed_of(r), LOOP_MOVE, 35 + 35 * p["speed"] + 10 * r.uniform(), 0.2, whine=0.05, road=0.2)


def engine_tracked(r, p):
    sd = seed_of(r)
    return s.engine(sd, LOOP_MOVE, 22 + 20 * p["speed"], 0.35, road=0.25, sub=0.5) \
        + s.tracks(sd + 1, LOOP_MOVE, round(8 + 10 * p["speed"])) * 0.35


def electric_drive(r, p):
    """A calm electric drive: a soft low hum and a quiet rolling sound, no whine."""
    n = n_of(LOOP_MOVE)
    f = qf(110 + 50 * p["speed"] + 20 * r.uniform(), LOOP_MOVE)
    t = np.arange(n) / SR
    out = (np.sin(2 * np.pi * f * t) + 0.25 * np.sin(4 * np.pi * f * t)) * 0.12 * (0.9 + 0.1 * wander(r, n, 4))
    return out + cnoise(r, n, 60, 600) * 0.2 * (0.85 + 0.15 * wander(r, n, 6))


def servo_walk(r, p):
    """A machine walking: muffled metal footfalls and a faint, low servo hum."""
    L = 2.4
    n = n_of(L)
    t = np.arange(n) / SR
    out = cnoise(r, n, 80, 600) * 0.05 + np.sin(2 * np.pi * qf(140, L) * t) * 0.03
    for k in range(4):
        stomp = s.lowpass(modal(r, [90, 240, 620], [0.06, 0.04, 0.02], [1, 0.5, 0.25], 0.4, 0.6, 1200), 2000)
        place_wrap(out, stomp * 0.7, k * L / 4)
    return out


def quad_buzz(r, p):
    """A small drone: a soft, low whir of its rotors and moving air; steady, no sweep."""
    n = n_of(LOOP_MOVE)
    t = np.arange(n) / SR
    base = qf(r.uniform(90, 120), LOOP_MOVE)
    out = np.zeros(n)
    for k, a in [(1, 1.0), (2, 0.35), (3, 0.12)]:
        out += a * np.sin(2 * np.pi * base * k * t + r.uniform(0, 6))
    out *= 0.15 * (0.92 + 0.08 * wander(r, n, 4))
    return out + cnoise(r, n, 100, 1200, color=1) * 0.25


def propeller(r, p):
    return s.propeller(seed_of(r))


def rotor(r, p):
    """A rotorcraft under way: blade slap, turbine whine and tail rotor (the shared `act.rotor`
    with its own seed); a ducted-fan craft (`ducted`) keeps a little of it under the whir of
    its fans."""
    blades = s.rotor(seed_of(r))
    if p.get("ducted"):
        return 0.4 * blades + quad_buzz(r, p)
    return blades


def jet(r, p):
    return s.jet(seed_of(r))


def orbital_craft(r, p):
    return s.jet(seed_of(r)) * 0.6 + s.starship_hum(seed_of(r), LOOP_MOVE) * 0.6


def water_bed(r, n, amp=0.25):
    return cnoise(r, n, 150, 2500) * amp * (0.5 + 0.5 * wander(r, n, 6))


def ship_oars(r, p):
    L = LOOP_MOVE
    out = water_bed(r, n_of(L))
    for k in range(3):
        at = k * L / 3
        place_wrap(out, modal(r, [260, 610], [0.04, 0.02], [1, 0.4], 0.2, 0.3, 2000) * 0.4, at)  # oars in locks
        place_wrap(out, burst(r, 0.5, 300, 3500, 0.08, 0.15, 0.6), at + 0.1)  # blades in the water
    place_wrap(out, s.stick_slip(r, 0.6, 60, 110) * 0.25, L * 0.45)
    return out


def ship_sail(r, p):
    L = LOOP_MOVE
    n = n_of(L)
    out = water_bed(r, n, 0.35)
    flap = cnoise(r, n, 60, 400) * np.clip(wander(r, n, 10, -1, 1), 0, 1) ** 2 * 0.6
    out += flap
    place_wrap(out, s.stick_slip(r, 0.8, 40, 90) * 0.3, L * 0.3)
    return out


def ship_engine(r, p):
    n = n_of(LOOP_MOVE)
    return s.engine(seed_of(r), LOOP_MOVE, 14 + 8 * r.uniform(), 0.15, sub=0.6) * 0.8 + water_bed(r, n, 0.4)


def starship(r, p):
    return s.starship_hum(seed_of(r))


def hover(r, p):
    return s.hover_hum(seed_of(r))


MOVE = {
    "footsteps": footsteps, "hoofbeats": hoofbeats, "wheels_earth": wheels, "engine_wheeled": engine_wheeled,
    "engine_tracked": engine_tracked, "electric_drive": electric_drive, "servo_walk": servo_walk,
    "quad_buzz": quad_buzz, "propeller": propeller, "rotor": rotor, "jet": jet, "orbital_craft": orbital_craft,
    "ship_oars": ship_oars, "ship_sail": ship_sail, "ship_engine": ship_engine, "starship_hum": starship,
    "hover_hum": hover,
}


def move(name):
    def make(r, p):
        return voiced(MOVE[name](r, p), p["pitch"], 1.0, loop=True)
    return make


# ------------------------------------------------------------------ buildings at work (8 s loops)

def babble(r, seconds, talkers, amp=1.0):
    """Crowd murmur: syllables of pulsed voices through random vowel formants, periodic."""
    n = n_of(seconds)
    out = np.zeros(n)
    vowels = [(730, 1090), (270, 2290), (530, 1840), (570, 840), (440, 1020), (300, 870), (660, 1720)]
    for _ in range(talkers):
        f0 = r.uniform(95, 230)
        at = r.uniform(0, seconds)
        end = at + seconds
        while at < end:
            d = r.uniform(0.09, 0.25)
            m = n_of(d)
            t = np.arange(m) / SR
            f = f0 * (1 + 0.15 * r.uniform(-1, 1) * t / d)
            pulses = (np.sin(2 * np.pi * np.cumsum(f) / SR) > 0.95).astype(float)
            f1, f2 = vowels[int(r.integers(len(vowels)))]
            syl = s.bandpass(pulses, f1 * 0.8, f1 * 1.2) + 0.5 * s.bandpass(pulses, f2 * 0.85, f2 * 1.15)
            place_wrap(out, syl * np.sin(np.pi * t / d) * r.uniform(0.3, 1.0), at)
            at += d + (r.exponential(0.5) if r.uniform() < 0.25 else r.uniform(0.01, 0.06))
    return s.cfilter(out, 100, 3500) * amp


def norm(x):
    return x / (np.sqrt(np.mean(x**2)) or 1)


def a_workshop(r, p):
    return s.workshop(seed_of(r))


def a_smithy(r, p):
    return s.smithy(seed_of(r))


def a_mill(r, p):
    return s.mill(seed_of(r))


def a_factory(r, p):
    return s.factory(seed_of(r))


def a_refinery(r, p):
    return s.refinery(seed_of(r))


def a_power(r, p):
    return s.power_plant(seed_of(r))


def a_shipyard(r, p):
    return s.shipyard(seed_of(r))


def a_spaceport(r, p):
    return s.spaceport(seed_of(r))


def events(r, out, maker, count, gain=(0.5, 1.0)):
    for _ in range(count):
        place_wrap(out, maker(r) * r.uniform(*gain), r.uniform(0, len(out) / SR))
    return out


def chirps(r, out, count, far=0.5):
    for _ in range(count):
        c = s.lowpass(s.chirp(r, int(r.integers(0, 2))), 9000 - 5000 * far) * r.uniform(0.05, 0.2)
        place_wrap(out, c, r.uniform(0, len(out) / SR))
    return out


def a_farm(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 1800, 9000) * 0.05 * (0.3 + wander(r, n, 6) ** 2)
    for k in range(6):
        place_wrap(out, s.hoe_soil(r) * r.uniform(0.5, 0.9), 0.4 + k * 1.3 + r.uniform(-0.1, 0.1))
    return chirps(r, out, 6)


def a_farm_auto(r, p):
    n = n_of(LOOP_ACTIVE)
    out = s.engine(seed_of(r), LOOP_ACTIVE, 24, 0.25, sub=0.4) * 0.5
    t = np.arange(n) / SR
    out += cnoise(r, n, 2000, 9000) * 0.15 * (0.5 + 0.5 * np.sin(2 * np.pi * qf(0.5, LOOP_ACTIVE) * t) ** 2)
    return out


def a_woodcut(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 1500, 8000) * 0.04 * wander(r, n, 6, 0.3, 1)
    at = 0.3
    while at < LOOP_ACTIVE - 0.5:
        place_wrap(out, s.axe_wood(r) * r.uniform(0.6, 1.0), at)
        at += r.uniform(0.7, 1.1)
    place_wrap(out, s.stick_slip(r, 0.9, 30, 70, 150, 700) * 0.4, r.uniform(0, LOOP_ACTIVE))
    return chirps(r, out, 4, 0.8)


def a_woodcut_auto(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    rev = 0.5 + 0.5 * np.clip(np.sin(2 * np.pi * qf(0.25, LOOP_ACTIVE) * t + r.uniform(0, 6)), 0, 1)
    ph = phase_of(120 + 110 * rev)
    saw_tooth = sum(np.sin(k * ph) / k for k in range(1, 20))
    return s.cfilter(saw_tooth, hi=4000) * 0.4 * rev + cnoise(r, n, 2000, 8000) * 0.2 * rev ** 2


def a_sawmill(r, p):
    out = s.saw(seed_of(r), LOOP_ACTIVE) * 0.7
    for k in range(3):
        place_wrap(out, s.hammer_wood(r) * 0.5, 1.0 + k * 2.6)
    return out


def a_mine(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 40, 300, color=1) * 0.15
    at = 0.2
    while at < LOOP_ACTIVE - 0.3:
        x = s.pickaxe_stone(r) * r.uniform(0.5, 1.0)
        place_wrap(out, x, at)
        place_wrap(out, s.lowpass(x, 3000) * 0.25, at + 0.12)  # echo in the gallery
        at += r.uniform(0.5, 0.9)
    return out


def a_mine_auto(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    drill = np.sin(phase_of(np.full(n, 310.0) * (1 + 0.02 * wander(r, n, 8, -1, 1))))
    out = s.cfilter(np.sign(drill), hi=2500) * 0.15 * (0.6 + 0.4 * wander(r, n, 4))
    out += cnoise(r, n, 30, 400, color=1) * 0.4
    out += cnoise(r, n, 1000, 5000) * 0.1 * (np.sin(2 * np.pi * qf(3, LOOP_ACTIVE) * t) > 0.7)
    return out


def a_oil_pump(r, p):
    n = n_of(LOOP_ACTIVE)
    out = s.engine(seed_of(r), LOOP_ACTIVE, 9, 0.2, sub=0.3) * 0.4
    for k in range(4):
        place_wrap(out, s.stick_slip(r, 0.7, 50, 90, 200, 900) * 0.4, k * 2.0)
        place_wrap(out, s.lowpass(r.standard_normal(n_of(0.3)), 200) * env_ad(n_of(0.3), 0.01, 0.06), k * 2.0 + 1.0)
    return out


def a_oil_pump_auto(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    hum = sum(a * np.sin(2 * np.pi * f * t) for f, a in [(60, 1), (120, 0.6), (180, 0.3)]) * 0.15
    flow = cnoise(r, n, 200, 3000) * 0.2 * (0.7 + 0.3 * np.sin(2 * np.pi * qf(0.75, LOOP_ACTIVE) * t))
    return hum + flow


def a_study(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 80, 1200, color=1) * 0.03 + babble(r, LOOP_ACTIVE, 2, 0.08)
    for _ in range(3):  # pages turning
        place_wrap(out, burst(r, 0.35, 1500, 8000, 0.1, 0.1, 0.35), r.uniform(0, LOOP_ACTIVE))
    for k in range(8):  # a clock in the next room
        place_wrap(out, modal(r, [2500, 3800], [0.01, 0.006], [1, 0.5], 0.05, 0.2) * 0.04, k * 1.0)
    for _ in range(5):  # a pen scratching
        m = n_of(r.uniform(0.3, 0.7))
        grain = cnoise(r, m, 2500, 9000) * (np.abs(np.sin(2 * np.pi * r.uniform(5, 9) * np.arange(m) / SR)) ** 3)
        place_wrap(out, grain * 0.12, r.uniform(0, LOOP_ACTIVE))
    return out


def a_study_auto(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    out = cnoise(r, n, 200, 3000) * 0.08 + np.sin(2 * np.pi * 120 * t) * 0.04
    for _ in range(8):  # instruments beeping
        f = r.choice([880.0, 1046.5, 1318.5, 1760.0])
        m = n_of(0.08)
        place_wrap(out, np.sin(2 * np.pi * f * np.arange(m) / SR) * env_ad(m, 0.003, 0.03) * 0.12, r.uniform(0, LOOP_ACTIVE))
    for _ in range(12):  # keys
        place_wrap(out, modal(r, [3000, 4500], [0.004, 0.003], [1, 0.5], 0.03, 0.3) * 0.05, r.uniform(0, LOOP_ACTIVE))
    return out


def a_town(r, p):
    n = n_of(LOOP_ACTIVE)
    out = babble(r, LOOP_ACTIVE, 10, 1.0)
    out = norm(out) * 0.25 + cnoise(r, n, 100, 2000) * 0.05
    for k in range(3):
        place_wrap(out, s.hammer_wood(r) * 0.25, 1.0 + k * 0.35)
    place_wrap(out, s.wheels_earth(seed_of(r), 4.0)[: n_of(3.0)] * np.sin(np.pi * np.arange(n_of(3.0)) / n_of(3.0)) * 0.2, 4.0)
    return chirps(r, out, 3, 0.7)


def a_colony(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    out = norm(babble(r, LOOP_ACTIVE, 6)) * 0.12 + sum(np.sin(2 * np.pi * f * t) for f in (55, 110)) * 0.06
    out += cnoise(r, n, 50, 500, color=1) * 0.12
    for k, f in enumerate((784.0, 1046.5, 1318.5)):
        m = n_of(0.5)
        place_wrap(out, np.sin(2 * np.pi * f * np.arange(m) / SR) * env_ad(m, 0.005, 0.15) * 0.08, 2.0 + k * 0.15)
    return out


def a_market(r, p):
    out = norm(babble(r, LOOP_ACTIVE, 16)) * 0.3
    for _ in range(10):  # coins
        coin = modal(r, [3800, 5900, 8200], [0.08, 0.05, 0.03], [1, 0.6, 0.4], 0.3, 0.2, 10000, 0.08)
        place_wrap(out, coin * r.uniform(0.05, 0.15), r.uniform(0, LOOP_ACTIVE))
    return out


def chant(r, seconds, root):
    """A quiet sung drone: a few voices on a vowel, a fifth and an octave apart, slowly changing
    vowel; periodic over `seconds`. Synthesised, no melody."""
    n = n_of(seconds)
    t = np.arange(n) / SR
    vowels = [(700, 1150), (450, 800), (350, 900), (500, 1400)]  # ah, oh, oo, eh-ish
    blend = wander(r, n, 4)
    out = np.zeros(n)
    for ratio, amp in [(1.0, 1.0), (1.0, 0.8), (1.5, 0.6), (2.0, 0.5)]:
        f = qf(root * ratio * r.uniform(0.997, 1.003), seconds)
        vib = 1 + 0.004 * np.sin(2 * np.pi * qf(r.uniform(4.5, 5.5), seconds) * t + r.uniform(0, 6))
        ph = phase_of(f * vib)
        pulses = sum(np.sin(k * ph) / k for k in range(1, 30))
        a, b = vowels[int(r.integers(len(vowels)))], vowels[int(r.integers(len(vowels)))]
        v1 = s.cfilter(pulses, a[0] * 0.8, a[0] * 1.25) + 0.5 * s.cfilter(pulses, a[1] * 0.85, a[1] * 1.2)
        v2 = s.cfilter(pulses, b[0] * 0.8, b[0] * 1.25) + 0.5 * s.cfilter(pulses, b[1] * 0.85, b[1] * 1.2)
        out += amp * (v1 * blend + v2 * (1 - blend))
    return out


def a_temple(r, p):
    """A church: a quiet chant hum and a distant bell now and then."""
    root = r.choice([98.0, 110.0, 130.81])
    out = norm(chant(r, LOOP_ACTIVE, root)) * 0.08
    toll = s.lowpass(s.bell(root * 4, 5.0), 3500)
    place_wrap(out, toll * 0.3, 1.0)
    return out


def a_drill_yard(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 100, 1500) * 0.03
    beats = 16
    for k in range(beats):  # a squad marching in step
        for f in range(4):
            m = n_of(0.2)
            step = s.lowpass(r.standard_normal(m), 300) * env_ad(m, 0.004, 0.03) * 2
            step += s.bandpass(r.standard_normal(m), 1500, 6000) * env_ad(m, 0.01, 0.04) * 0.3
            place_wrap(out, step * r.uniform(0.5, 0.9), k * LOOP_ACTIVE / beats + r.uniform(0, 0.03))
        if p["age"] >= 2 and k % 4 == 0:
            place_wrap(out, burst(r, 0.1, 2500, 8000, 0.005, 0.03, 0.15), k * LOOP_ACTIVE / beats + 0.02)
    for _ in range(3):
        place_wrap(out, s.sword_clash(r)[: n_of(0.6)] * 0.25, r.uniform(0, LOOP_ACTIVE))
    return out


def a_stable(r, p):
    n = n_of(LOOP_ACTIVE)
    out = cnoise(r, n, 1500, 7000) * 0.03 * wander(r, n, 8)  # straw
    for _ in range(6):  # hooves shifting
        hoof = s.lowpass(r.standard_normal(n_of(0.15)), 500) * env_ad(n_of(0.15), 0.002, 0.03)
        place_wrap(out, hoof * r.uniform(0.3, 0.7), r.uniform(0, LOOP_ACTIVE))
    for _ in range(2):  # a horse snorting: a noisy, nasal burst
        m = n_of(0.5)
        snort = s.bandpass(r.standard_normal(m), 300, 1200) * (np.sin(2 * np.pi * 25 * np.arange(m) / SR) > 0) * env_ad(m, 0.05, 0.12)
        place_wrap(out, snort * 0.5, r.uniform(0, LOOP_ACTIVE))
    return out


def a_siege_yard(r, p):
    out = s.workshop(seed_of(r))
    place_wrap(out, s.stick_slip(r, 1.0, 40, 80, 150, 700) * 0.4, r.uniform(0, LOOP_ACTIVE))
    return out


def a_airfield(r, p):
    if p["age"] <= 6:
        x = s.propeller(seed_of(r))
    else:
        x = s.jet(seed_of(r))
    out = np.tile(x, 2)[: n_of(LOOP_ACTIVE)] * 0.4
    for k in range(2):
        m = n_of(0.12)
        place_wrap(out, np.sin(2 * np.pi * 1000 * np.arange(m) / SR) * env_ad(m, 0.003, 0.05) * 0.1, 3.0 + k * 0.2)
    return out


def a_post(r, p):
    n = n_of(LOOP_ACTIVE)
    gust = wander(r, n, 6, 0.2, 1.0) ** 1.5
    out = cnoise(r, n, 150, 1500, color=1) * 0.4 * gust
    out += cnoise(r, n, 60, 300) * 0.3 * np.clip(wander(r, n, 24, -1, 1), 0, 1) * gust  # flag flapping
    place_wrap(out, s.stick_slip(r, 0.7, 40, 80) * 0.2, r.uniform(0, LOOP_ACTIVE))
    return out


def a_post_modern(r, p):
    n = n_of(LOOP_ACTIVE)
    out = s.engine(seed_of(r), LOOP_ACTIVE, 25, 0.2, sub=0.3) * 0.3
    out += cnoise(r, n, 800, 4000) * 0.05 * (wander(r, n, 16) > 0.7)  # radio static
    return out + a_post(r, p) * 0.4


def a_post_future(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    out = sum(np.sin(2 * np.pi * f * t) for f in (60, 120, 180)) * 0.05
    sweep = 0.5 + 0.5 * np.cos(2 * np.pi * qf(0.25, LOOP_ACTIVE) * t)
    out += np.sin(phase_of(500 + 300 * sweep)) * 0.05
    for k in range(4):
        m = n_of(0.1)
        place_wrap(out, np.sin(2 * np.pi * 1600 * np.arange(m) / SR) * env_ad(m, 0.002, 0.03) * 0.1, k * 2.0 + 0.3)
    return out


def a_sensor(r, p):
    n = n_of(LOOP_ACTIVE)
    t = np.arange(n) / SR
    out = cnoise(r, n, 40, 400, color=1) * 0.15 + np.sin(2 * np.pi * 50 * t) * 0.05
    for k in range(2):  # a sweep and its ping
        place_wrap(out, burst(r, 1.5, 500, 3000, 0.7, 0.3, 0.3), k * 4.0)
        m = n_of(1.0)
        place_wrap(out, np.sin(2 * np.pi * 1200 * np.arange(m) / SR) * env_ad(m, 0.005, 0.2) * 0.15, k * 4.0 + 1.2)
    return out


ACTIVE = {
    "workshop": a_workshop, "smithy": a_smithy, "mill": a_mill, "factory": a_factory, "refinery": a_refinery,
    "power": a_power, "power_auto": a_power, "shipyard": a_shipyard, "spaceport": a_spaceport,
    "farm": a_farm, "farm_auto": a_farm_auto, "woodcut": a_woodcut, "woodcut_auto": a_woodcut_auto,
    "sawmill": a_sawmill, "mine": a_mine, "mine_auto": a_mine_auto, "oil_pump": a_oil_pump,
    "oil_pump_auto": a_oil_pump_auto, "study": a_study, "study_auto": a_study_auto, "town": a_town,
    "colony": a_colony, "market": a_market, "temple": a_temple, "drill_yard": a_drill_yard, "stable": a_stable,
    "siege_yard": a_siege_yard, "airfield": a_airfield, "post": a_post, "post_modern": a_post_modern,
    "post_future": a_post_future, "sensor": a_sensor, "workshop_auto": a_factory,
}


def active(name):
    def make(r, p):
        return voiced(ACTIVE[name](r, p), p["pitch"], 1.0, loop=True)
    return make


# ------------------------------------------------------------------ building cues

SCALE = [1.0, 9 / 8, 5 / 4, 3 / 2, 5 / 3, 2.0]  # a pentatonic set


def motif(r, base, count):
    steps = sorted(r.choice(len(SCALE), size=count, replace=False))
    return [base * SCALE[i] for i in steps]


def instrument(p):
    return "history" if p["age"] < 5 else ("modern" if p["age"] < 8 else "future")


def play_notes(freqs, gap, era, length):
    parts = []
    for k, f in enumerate(freqs):
        if era == "history":
            x = s.bell(f, 1.5) if f > 600 else s.marimba(f, 0.6)
        elif era == "modern":
            x = s.note(f, 0.8, 0.25, ((1, 1.0), (3, 0.15), (4, 0.1)))
        else:
            n = n_of(1.0)
            t = np.arange(n) / SR
            x = np.sin(2 * np.pi * f * t + 1.2 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / 0.15)) * env_ad(n, 0.004, 0.3)
        parts.append((k * gap, x))
    return s.sequence(parts, length)


def b_select(r, p):
    """Clicking a building: a second of its work, faded in and out."""
    loop = ACTIVE[p["active"]](r, p)
    win = n_of(1.0)
    energy = np.convolve(loop**2, np.ones(n_of(0.05)), "same")
    start = int(np.argmax(energy[: len(loop) - win] if len(loop) > win else energy))
    start = max(0, min(start - n_of(0.3), len(loop) - win))
    x = loop[start:start + win].copy()
    x *= np.minimum(1, np.arange(win) / n_of(0.04)) * np.minimum(1, (win - np.arange(win)) / n_of(0.3))
    return voiced(x, p["pitch"], 1.0)


def b_build_done(r, p):
    era = instrument(p)
    base = 330 * p["pitch"] * (1.2 - 0.4 * p["size"])
    out = np.zeros(n_of(2.2))
    if era == "history":
        for k in range(3):
            place(out, s.hammer_wood(r) * 0.8, k * 0.18)
    elif era == "modern":
        for k in range(5):
            place(out, modal(r, [1200, 2900], [0.01, 0.006], [1, 0.5], 0.06, 0.6) * 0.5, k * 0.06)
        place(out, s.anvil(r)[: n_of(0.6)] * 0.4, 0.35)
    else:
        m = n_of(0.4)
        t = np.arange(m) / SR
        place(out, np.sin(2 * np.pi * np.cumsum(400 + 600 * t / t[-1]) / SR) * np.sin(np.pi * t / t[-1]) * 0.3, 0)
    place(out, play_notes(motif(r, base, 2), 0.14, era, 1.6), 0.55)
    return out


def b_build_done_bells(r, p):
    """A church finished: a short peal of bells, high to low."""
    base = 523.25 * p["pitch"]
    bells = [base * k for k in (2.0, 5 / 3, 3 / 2, 4 / 3, 9 / 8, 1.0)]
    order = list(range(len(bells))) * 2
    return s.sequence([(k * 0.28, s.bell(bells[i], 2.2) * (0.6 + 0.4 * r.uniform())) for k, i in enumerate(order)], 6.0)


def b_train_done(r, p):
    era = instrument(p)
    base = 260 * p["pitch"]
    if era == "history":
        notes = motif(r, base, 2)
        horn = [(k * 0.3, s.brass([f], 0.35 if k == 0 else 0.7, 0.04, 0.2)) for k, f in enumerate(notes)]
        return s.sequence(horn, 1.3)
    if era == "modern":
        m = n_of(0.1)
        beep = np.sin(2 * np.pi * 1200 * np.arange(m) / SR) * env_ad(m, 0.003, 0.04) * 0.4
        return s.sequence([(0, beep), (0.15, play_notes(motif(r, base * 2, 2), 0.12, era, 1.0))], 1.3)
    return play_notes(motif(r, base * 2, 3), 0.1, era, 1.4)


def b_research_done(r, p):
    era = instrument(p)
    base = 392 * p["pitch"]
    return play_notes(motif(r, base, 4), 0.13, era, 2.4)


def recipe(name):
    """The function that makes a plan entry's sound."""
    group, _, what = name.partition(".")
    if group == "fire":
        return fire(what)
    if group == "fight":
        return fight(what)
    if group == "move":
        return move(what)
    if group == "active":
        return active(what)
    return {"bld.select": b_select, "bld.build_done": b_build_done, "bld.build_done_bells": b_build_done_bells,
            "bld.train_done": b_train_done,
            "bld.research_done": b_research_done}[name]
