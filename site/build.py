#!/usr/bin/env python3
"""Build the public site of Ascension into site/dist/ (tasks 135 and 136).

    python3 site/build.py [--binary app/target/release/rise-of-man] [--cache site/cache]

Pages: index.html (README.md), privacy.html (the Store's privacy policy) and catalogue.html: the
model review page's presentation without its review elements, one chapter per age with the
age's story and main theme above its models, each model's card with its sheet and its sounds,
and the shared cues. story.html, units.html and sounds.html (task 135) are plain links to it.
The model data, names and sound sprites come from the model review page's own scripts
(app/docs/art/review-page/build.py and sprites.py) run on the game's `--gallery-manifest`; the
pictures from `rise-of-man --model <id> --screenshot`; the ages from app/assets/text/ages.toml.
Pictures, sprites and converted images are kept in the cache folder between runs.

Needs: Python 3 with `markdown` (see site/README.md), ffmpeg, an X11 display for the game's
screenshots, and a release build of rom-app.
"""
import argparse
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path
from string import Template

import markdown

SITE = Path(__file__).resolve().parent
REPO = SITE.parent
APP = REPO / "app"
REVIEW = APP / "docs" / "art" / "review-page"
GITHUB = "https://github.com/alfu32/ascension"
SHEET_SIZE = "1600x900"
SHEET_WIDTHS = (640, 1280)
README_WIDTH = 1600

# Age keys in the game's order, as the review page sorts them; "any" holds models of every age.
AGES = ["ancient", "classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern",
        "information", "automation", "fusion", "starfaring", "expansion", "abundance", "galactic",
        "federation", "any"]
KINDS = {"unit": "Units", "bld": "Buildings", "node": "Sites"}
KIND_ONE = {"unit": "unit", "bld": "building", "node": "site"}
EVENTS = [("select", "Selected"), ("move", "Move"), ("attack", "Attack"), ("build", "Build"),
          ("fire", "Firing"), ("fight", "Fighting"), ("moving", "Moving"), ("active", "Working"),
          ("train_done", "Unit ready"), ("build_done", "Build finished"),
          ("research_done", "Research finished")]
NAV = [("index.html", "Home"), ("catalogue.html", "Catalogue"), ("privacy.html", "Privacy")]
# What the review page carries and the public page must not: its review elements and the
# authoring words around them, model ids as text, and anything private.
REVIEW_WORDS = re.compile(r"\b(review|round|fingerprint|verdict|approved|draft|needs work|"
                          r"directions?|status|owner|sketch(es)?|copy id)\b", re.I)
MODEL_ID = re.compile(r"\b(unit|bld|node)\.[a-z0-9_]+(@[a-z~0-9_]+)?")
PRIVATE = re.compile(r"claude\.ai|/home/|/var/|/etc/|ubuntu@|\b\d{1,3}(\.\d{1,3}){3}\b|"
                     r"oraci|ed25519", re.I)


def log(*args):
    print(*args, file=sys.stderr, flush=True)


def run(cmd, **kw):
    subprocess.run(cmd, check=True, **kw)


def digest(data: bytes, n=10) -> str:
    return hashlib.sha256(data).hexdigest()[:n]


class Dist:
    """The output folder; assets get a content hash in their name so they can be cached forever."""

    def __init__(self, root: Path):
        self.root = root
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        self.bytes = 0

    def asset(self, src: Path, folder: str, stem: str) -> str:
        data = src.read_bytes()
        name = f"{folder}/{stem}.{digest(data)}{src.suffix}"
        dst = self.root / name
        if not dst.exists():
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dst)
        return name

    def page(self, name: str, text: str):
        (self.root / name).write_text(text)


# ---------------------------------------------------------------- inputs from the game


def game_env():
    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)  # X11 is the validated path
    return env


def manifest(binary: Path, cache: Path) -> Path:
    out = cache / "models.json"
    log("gallery manifest ...")
    run([str(binary), "--gallery-manifest", str(out)], env=game_env(), cwd=APP,
        stdout=subprocess.DEVNULL)
    return out


def sound_sprites(cache: Path) -> Path:
    """The review page's sprites (sprites.py), re-made only when a sound or the assignment changes."""
    src = APP / "assets" / "sounds"
    assignment = APP / "tools" / "sounds" / "assignment.json"
    h = hashlib.sha256((REVIEW / "sprites.py").read_bytes())
    for f in sorted(src.glob("*.ogg")) + [assignment]:
        h.update(f.name.encode())
        h.update(f.read_bytes())
    stamp = h.hexdigest()
    out = cache / "sounds"
    if (out / "stamp").exists() and (out / "stamp").read_text().strip() == stamp \
            and (out / "sound-index.json").exists():
        return out
    log("sound sprites (sprites.py, about ten minutes) ...")
    staging = cache / "sound-src"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)
    for f in src.glob("*.ogg"):
        (staging / f.name).symlink_to(f)
    shutil.copy(assignment, staging / "assignment.json")
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    run([sys.executable, "-I", str(REVIEW / "sprites.py"), str(staging), str(out)])
    (out / "stamp").write_text(stamp + "\n")
    return out


def review_data(models_json: Path, sounds: Path) -> dict:
    """Run the review page's builder on the manifest and read back the data it puts in the page."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        shutil.copy(models_json, tmp / "models.json")
        shutil.copy(REVIEW / "page.html", tmp / "page.html")
        (tmp / "img").mkdir()
        run([sys.executable, "-I", str(REVIEW / "build.py"), str(tmp), "0", "", str(sounds)],
            stdout=subprocess.DEVNULL)
        page = (tmp / "model-review.html").read_text()
    data = {}
    for name in ("MODELS", "SOUNDS", "LIBRARY", "INDEX"):
        m = re.search(rf"^const {name} = (.*);$", page, re.M)
        if not m:
            sys.exit(f"the review page no longer carries {name}")
        data[name] = json.loads(m.group(1))
    return data


def sheet(binary: Path, cache: Path, model: dict, fingerprint: str, refresh: bool) -> dict:
    """The model sheet from the game, as web pictures: {width: path}."""
    shots = cache / "shots"
    web = cache / "web"
    shots.mkdir(parents=True, exist_ok=True)
    web.mkdir(parents=True, exist_ok=True)
    base = f"{model['img']}.{fingerprint}"
    png = shots / f"{base}.png"
    if refresh or not png.exists():
        log("  sheet", model["id"])
        tmp = png.with_suffix(".tmp.png")
        run([str(binary), "--model", model["id"], "--screenshot", str(tmp), "--size", SHEET_SIZE],
            env=game_env(), cwd=APP, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        tmp.replace(png)
    out = {}
    for w in SHEET_WIDTHS:
        dst = web / f"{base}.{w}.webp"
        if refresh or not dst.exists() or dst.stat().st_mtime < png.stat().st_mtime:
            to_webp(png, dst, w, 78)
        out[w] = dst
    return out


def to_webp(src: Path, dst: Path, width: int, quality: int):
    run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(src), "-vf",
         f"scale='min({width},iw)':-2:flags=lanczos", "-c:v", "libwebp", "-quality", str(quality),
         "-compression_level", "6", str(dst)])


def to_png(src: Path, dst: Path, size: int):
    run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(src), "-vf",
         f"scale={size}:{size}:flags=lanczos", str(dst)])


def image_size(path: Path):
    out = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
                          "stream=width,height", "-of", "csv=p=0", str(path)],
                         check=True, capture_output=True, text=True).stdout.strip()
    w, h = out.split(",")[:2]
    return int(w), int(h)


# ---------------------------------------------------------------- page helpers


def page(dist: Dist, ctx: dict, name: str, title: str, description: str, content: str,
         main_class="", scripts=""):
    nav = "\n      ".join(
        f'<a href="{href}"{" aria-current=\"page\"" if href == name else ""}>{label}</a>'
        for href, label in NAV if not (href == "index.html" == name))
    text = Template((SITE / "templates" / "page.html").read_text()).substitute(
        title=html.escape(title), description=html.escape(description), favicon=ctx["favicon"],
        css=ctx["css"], emblem=ctx["emblem"], nav=nav, page=name.removesuffix(".html"),
        main_class=main_class, content=content, version=html.escape(ctx["version"]),
        scripts=scripts)
    dist.page(name, text)


def md(text: str) -> str:
    return markdown.markdown(text, extensions=["tables", "fenced_code", "sane_lists"],
                             output_format="html")


def wrap_tables(body: str) -> str:
    return re.sub(r"<table>(.*?)</table>", r'<div class="table-scroll"><table>\1</table></div>',
                  body, flags=re.S)


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


# ---------------------------------------------------------------- pages


def home(dist: Dist, ctx: dict, cache: Path):
    text = (REPO / "README.md").read_text()
    # The owner (2026-11-23): no download link or button on the site, only the GitHub link at
    # the end of every page. The README's centred download line is left out here.
    text, n = re.subn(r'<p align="center">\s*<a href="[^"]*/releases/latest">.*?</p>\s*', "", text,
                      count=1, flags=re.S)
    if n != 1:
        sys.exit("README.md: the download line to leave out was not found")
    body = md(text)
    web = cache / "readme"
    web.mkdir(parents=True, exist_ok=True)

    def link(m):
        attr, url = m.group(1), m.group(2)
        if re.match(r"^(https?:|mailto:|#)", url):
            return m.group(0)
        path = (REPO / url).resolve()
        if REPO not in path.parents or not path.exists():
            sys.exit(f"README links to {url}, which is not in the repository")
        rel = path.relative_to(REPO).as_posix()
        if attr == "src":
            if path == REPO / "packaging" / "ascension.png":
                return f'src="{ctx["emblem_large"]}"'
            dst = web / (slug(rel) + ".webp")
            if not dst.exists() or dst.stat().st_mtime < path.stat().st_mtime:
                to_webp(path, dst, README_WIDTH, 80)
            return f'src="{dist.asset(dst, "img/readme", path.stem)}"'
        kind = "tree" if path.is_dir() else "blob"
        return f'href="{GITHUB}/{kind}/main/{rel}"'

    body = re.sub(r'\b(src|href)="([^"]+)"', link, body)
    body = re.sub(r"<img ", '<img loading="lazy" decoding="async" ', body)
    body = wrap_tables(body)
    page(dist, ctx, "index.html", "Ascension — Rise of Man",
         "A real-time strategy game from the first villages to an age of galactic empires.",
         f'<article class="md">\n{body}\n</article>', "prose")


def privacy(dist: Dist, ctx: dict):
    text = (REPO / "packaging" / "microsoft-store-kit" / "documentation" / "PRIVACY.md").read_text()
    body = md(text)
    body = re.sub(r"(?<![\"=>])(https://[^\s<]+?)([.,;]?)(?=[\s<])",
                  r'<a href="\1">\1</a>\2', body)
    page(dist, ctx, "privacy.html", "Privacy — Ascension",
         "The privacy policy of Ascension — Rise of Man: the game collects nothing.",
         f'<article class="md">\n{body}\n</article>', "prose")


def load_ages():
    """The fifteen ages as the game's new-age dialog tells them (app/assets/text/ages.toml, kept
    in step with app/docs/ages-story.md): key, number from one, name, era and story."""
    rows = tomllib.loads((APP / "assets" / "text" / "ages.toml").read_text())["age"]
    ages = []
    for row in sorted(rows, key=lambda r: r["ordinal"]):
        # The theme names the age's music; an age may share another's (age 14 plays age 13's
        # until it has its own, task 164), so the key comes from the order, not the theme.
        if row["ordinal"] >= len(AGES) - 1:
            sys.exit(f"ages.toml: age {row['ordinal']} is beyond the site's list of ages")
        key = AGES[row["ordinal"]]
        theme = row["theme"].split(".", 1)[0]
        if theme not in AGES:
            sys.exit(f"ages.toml: age {row['ordinal']} plays {theme!r}, not an age's theme")
        ages.append({"key": key, "theme": theme, "n": row["ordinal"] + 1, "name": row["name"],
                     "era": row["subtitle"], "story": " ".join(row["story"].split())})
    if len(ages) != len(AGES) - 1:
        sys.exit(f"ages.toml: expected {len(AGES) - 1} ages, found {len(ages)}")
    return ages


def sound_label(key: str) -> str:
    parts = key.split(".")[1:]
    if parts and parts[-1].isdigit():
        parts[-1] = f"({parts[-1]})"
    text = " · ".join(p.replace("_", " ") for p in parts)
    return text[:1].upper() + text[1:]


def event_label(ev: str) -> str:
    for k, label in EVENTS:
        if k == ev:
            return label
    slot = re.match(r"^fire_slot(\d+)$", ev)
    if slot:
        return f"Firing, weapon {int(slot.group(1)) + 1}"
    return ev.replace("_", " ").capitalize()


def play_button(label: str, keys: list) -> str:
    x = f'<span class="x">×{len(keys)}</span>' if len(keys) > 1 else ""
    return (f'<button class="play" type="button" aria-pressed="false" '
            f'data-keys="{html.escape(" ".join(keys))}">{html.escape(label)}{x}</button>')


def sound_box(events: dict) -> str:
    """A type's own sounds as the review page's card shows them: one button per event."""
    order = [k for k, _ in EVENTS]
    buttons = "".join(play_button(event_label(ev), events[ev]) for ev in sorted(
        events, key=lambda e: ((order.index(e) if e in order else 98), e)))
    return (f'<div class="snd-box"><span class="label">Sounds</span>'
            f'<div class="plays">{buttons}</div></div>')


def model_card(dist: Dist, binary: Path, cache: Path, m: dict, age_name: str, fingerprint: str,
               refresh: bool, sounds: dict) -> str:
    """One model as the review page's card shows it, with no review element: the sheet on the
    left; the name, the facts and the type's own sounds on the right."""
    kind = KIND_ONE.get(m["entity"].split(".", 1)[0], "model")
    pics = sheet(binary, cache, m, fingerprint, refresh)
    urls = {w: dist.asset(p, "img/models", m["img"]) for w, p in pics.items()}
    w, h = image_size(pics[SHEET_WIDTHS[-1]])
    srcset = ", ".join(f"{u} {wd}w" for wd, u in urls.items())
    alt = f"{m['name']}, {kind} of the {age_name} age: game camera, front, side and close views"
    size = m["size"]
    facts = [("Kind", kind.capitalize()), ("Age", age_name),
             ("Triangles", f"{m['near']:,} near · {m['far']:,} far"),
             ("Size (tiles)", f"{size[0]:g} × {size[2]:g} × {size[1]:g} high"),
             ("Parts", str(len(m["parts"])))]
    dl = "".join(f"<dt>{k}</dt><dd>{html.escape(v)}</dd>" for k, v in facts)
    snd = sound_box(sounds[m["entity"]]) if m["entity"] in sounds else ""
    big, small = urls[SHEET_WIDTHS[-1]], urls[SHEET_WIDTHS[0]]
    return f'''<article class="model" id="m-{slug(m["id"])}">
  <a class="pic" href="{big}"><img src="{small}" srcset="{srcset}" sizes="(max-width: 860px) 100vw, 700px" width="{w}" height="{h}" loading="lazy" decoding="async" alt="{html.escape(alt)}"></a>
  <div class="side"><h4>{html.escape(m["name"])}</h4><dl>{dl}</dl>{snd}</div>
</article>'''


def catalogue(dist: Dist, ctx: dict, data: dict, binary: Path, cache: Path, sprites: Path,
              fingerprints: dict, refresh: bool, limit: int | None):
    """The model review page's presentation for everyone: one chapter per age in the game's
    order (its number, name and era, its story, its main theme, then its models by kind, each
    card with its sounds), a chapter for the models of every age, and the shared cues."""
    ages = load_ages()
    music = APP / "assets" / "music"
    title_src = dist.asset(music / "title.ogg", "music", "title")
    sounds, index = data["SOUNDS"], data["INDEX"]
    assigned = set(json.loads((APP / "tools" / "sounds" / "assignment.json").read_text()))
    if assigned - set(sounds):
        sys.exit(f"types with sounds but no model: {sorted(assigned - set(sounds))[:5]}")
    missing = sorted({k for ev in sounds.values() for keys in ev.values() for k in keys}
                     - set(index))
    if missing:
        sys.exit(f"sounds missing from the sprites: {missing[:5]}")
    models = data["MODELS"][:limit] if limit else data["MODELS"]
    by_age: dict[str, dict[str, list]] = {}
    for m in models:
        if m["age"] not in AGES:
            sys.exit(f"{m['id']}: unknown age {m['age']!r}")
        by_age.setdefault(m["age"], {}).setdefault(m["entity"].split(".", 1)[0], []).append(m)
    chapters = ages + [{"key": "any", "n": None, "name": "Every age",
                        "era": "from the first cities to the stars",
                        "story": "The people and places that keep one look from the first age "
                                 "to the last."}]
    jump, blocks = [], []
    for a in chapters:
        groups = by_age.get(a["key"], {})
        count = sum(len(v) for v in groups.values())
        num = f'<span class="num">{a["n"]}</span>' if a["n"] else ""
        jump.append(f'<li><a href="#age-{a["key"]}">{num}{html.escape(a["name"])}'
                    f'<span class="n">{count}</span></a></li>')
        if a["n"]:
            theme = dist.asset(music / f"{a['theme']}.theme.ogg", "music", f"{a['theme']}.theme")
            tale = f'<blockquote class="tale"><p>{html.escape(a["story"])}</p></blockquote>'
            player = f'''<div class="player">
    <span class="label">Main theme of the {html.escape(a["name"])} age</span>
    <audio controls preload="none" src="{theme}"></audio>
  </div>'''
        else:
            tale, player = f'<p class="lead">{html.escape(a["story"])}</p>', ""
        parts = []
        for kind in sorted(groups, key=lambda k: list(KINDS).index(k) if k in KINDS else 9):
            cards = [model_card(dist, binary, cache, m, a["name"], fingerprints[m["id"]], refresh,
                                sounds)
                     for m in sorted(groups[kind], key=lambda m: (m["name"], m["id"]))]
            parts.append(f'<h3 class="kind">{KINDS.get(kind, kind.capitalize())}'
                         f'<small>{len(cards)}</small></h3>\n<div class="models">\n'
                         + "\n".join(cards) + "\n</div>")
        blocks.append(f'''<section class="chapter" id="age-{a["key"]}">
<header class="chapter-head">
  <h2>{num}<span class="name">{html.escape(a["name"])}</span><span class="era">{html.escape(a["era"])}</span></h2>
  {tale}
  {player}
</header>
{"".join(parts)}
</section>''')
    shared = []
    for group, keys in data["LIBRARY"]:
        buttons = "".join(play_button(sound_label(k), [k]) for k in keys)
        shared.append(f'<div class="lib-group"><h3>{html.escape(group)}</h3>'
                      f'<div class="plays">{buttons}</div></div>')
    urls = {}
    for sprite in sorted({v[0] for v in index.values()}):
        urls[sprite] = dist.asset(sprites / "sounds" / f"{sprite}.ogg", "sounds", sprite)
    sound_data = json.dumps({"index": index, "urls": urls}, separators=(",", ":"))
    content = f'''<header class="intro">
  <p class="eyebrow">Ascension · Rise of Man · version {html.escape(ctx["version"])}</p>
  <h1>The ages and their models</h1>
  <p class="lead">Every building and unit in the game, age by age. Models are built from parts
  inside the game and baked before play; buildings change their look as their player advances
  through the ages. Each age opens with its story, as the game tells it when your people reach
  that age, and its main theme: the same two notes in the musical language of the age.</p>
  <div class="player">
    <span class="label">Main title — the theme, heroic and without an age</span>
    <audio controls preload="none" src="{title_src}"></audio>
  </div>
  <ul class="jump">{"".join(jump)}<li><a href="#shared">Shared cues</a></li></ul>
</header>
<section class="how" aria-label="How to read the cards">
  <div><strong>Views</strong>Each sheet shows a model four ways: from the game's camera, from
  the front, from the side and close up. Open the picture to see it larger.</div>
  <div><strong>Sounds</strong>A unit's or building's card plays its own voices and effects; where
  there are several variations, one is chosen at random, as in the game.<span class="nojs"> The
  sound buttons need JavaScript.</span></div>
  <div><strong>Made for the game</strong>Every sound is made for it: effects are synthesised,
  voices spoken by a speech model at authoring time.</div>
</section>
{"".join(blocks)}
<section class="chapter" id="shared">
<header class="chapter-head">
  <h2><span class="name">Shared cues</span><span class="era">ambience, interface, activity</span></h2>
  <p class="lead">The sounds that belong to no single type: the ambience of the world, the
  interface and its announcer, and the activity of units at work.</p>
</header>
<div class="lib">
{"".join(shared)}
</div>
</section>
<script type="application/json" id="sound-data">{sound_data.replace("</", "<\\/")}</script>'''
    check_public(content, "catalogue.html")
    page(dist, ctx, "catalogue.html", "Catalogue — Ascension",
         f"The fifteen ages of Ascension, each with its story and main theme, and all "
         f"{len(models)} building and unit models with their sounds.",
         content, "catalogue", ctx["js_tag"])
    return len(models), len(sounds), len(index)


def moved(dist: Dist, ctx: dict):
    """The pages task 135 published, now one: a plain link to the catalogue for old links."""
    for name, what in (("story.html", "The story of the ages is"),
                       ("units.html", "The units and buildings are"),
                       ("sounds.html", "The sounds are")):
        page(dist, ctx, name, "Moved to the catalogue — Ascension",
             f"{what} now part of the catalogue of Ascension.",
             f'''<header>
  <p class="eyebrow">Moved</p>
  <h1>{what} now in the catalogue</h1>
  <p class="lead">The ages with their stories and music, every unit and building with its
  sounds, and the shared cues are now one page: <a href="catalogue.html">the catalogue</a>.</p>
</header>''', "prose")


# ---------------------------------------------------------------- checks


def check_public(content: str, name: str):
    """The catalogue's text carries none of the review page's elements and no model id."""
    text = re.sub(r'<script type="application/json".*?</script>', " ", content, flags=re.S)
    text = html.unescape(re.sub(r"<[^>]+>", " ", text))
    for pattern, what in ((REVIEW_WORDS, "a review element"), (MODEL_ID, "a model id")):
        m = pattern.search(text)
        if m:
            sys.exit(f"{name} would carry {what}: {m.group(0)!r} near "
                     f"{text[max(0, m.start() - 80):m.end() + 80]!r}")


def check_links(dist: Path):
    """Every internal link and file exists; nothing points at the private review page; no page
    carries an email address, a server path or address, or a key's name."""
    bad = []
    for f in sorted(dist.glob("*.html")):
        text = f.read_text()
        ids = set(re.findall(r'\bid="([^"]+)"', text))
        for url in re.findall(r'\b(?:href|src)="([^"]+)"', text) + \
                [u.split(" ")[0] for s in re.findall(r'srcset="([^"]+)"', text) for u in s.split(", ")]:
            if re.search(r"claude\.ai|artifact|review", url, re.I):
                bad.append(f"{f.name}: {url}")
            if re.match(r"^(https?:|mailto:)", url):
                if not url.startswith(("https://github.com/alfu32/ascension",)):
                    bad.append(f"{f.name}: external {url}")
                continue
            path, _, frag = url.partition("#")
            target = dist / path if path else f
            if not target.exists():
                bad.append(f"{f.name}: missing {url}")
            elif frag:
                tids = ids if target == f else set(re.findall(r'\bid="([^"]+)"', target.read_text()))
                if frag not in tids:
                    bad.append(f"{f.name}: no anchor {url}")
        if re.search(r"[\w.+-]+@[\w-]+\.[\w.]+", re.sub(r"<[^>]+>", " ", text)):
            for m in re.findall(r"[\w.+-]+@[\w-]+\.[a-z]{2,}", re.sub(r"<[^>]+>", " ", text)):
                bad.append(f"{f.name}: looks like an email address: {m}")
        for m in PRIVATE.finditer(text):
            bad.append(f"{f.name}: private: {text[max(0, m.start() - 40):m.end() + 40]!r}")
        if re.search(r'<audio(?![^>]*preload="none")', text):
            bad.append(f"{f.name}: an audio player without preload=\"none\"")
        if re.search(r'\bstyle="', text):
            bad.append(f"{f.name}: an inline style (the server's policy blocks them)")
    data = json.loads(re.search(r'id="sound-data">(.*?)</script>',
                                (dist / "catalogue.html").read_text(), re.S).group(1).replace("<\\/", "</"))
    for url in data["urls"].values():
        if not (dist / url).exists():
            bad.append(f"catalogue.html: missing sprite {url}")
    if bad:
        sys.exit("link check failed:\n  " + "\n  ".join(bad[:40]))


# ---------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--binary", type=Path, default=APP / "target" / "release" / "rise-of-man")
    ap.add_argument("--cache", type=Path, default=SITE / "cache",
                    help="pictures, sprites and converted images kept between runs")
    ap.add_argument("--out", type=Path, default=SITE / "dist")
    ap.add_argument("--refresh-pictures", action="store_true",
                    help="take every model sheet again (after a change to the renderer)")
    ap.add_argument("--limit", type=int, help="only the first N models (a quick look)")
    args = ap.parse_args()
    binary = args.binary.resolve()
    if not binary.exists():
        sys.exit(f"no game binary at {binary}: cargo build --release --locked -p rom-app")
    cache = args.cache.resolve()
    cache.mkdir(parents=True, exist_ok=True)

    version = re.search(r'^version = "([^"]+)"', (APP / "Cargo.toml").read_text(), re.M).group(1)
    dist = Dist(args.out.resolve())
    emblem_dir = cache / "emblem"
    emblem_dir.mkdir(exist_ok=True)
    emblem_src = REPO / "packaging" / "ascension.png"
    for size in (64, 256):
        to_png(emblem_src, emblem_dir / f"emblem-{size}.png", size)
    to_png(emblem_src, emblem_dir / "favicon-32.png", 32)
    ctx = {
        "version": version,
        "css": dist.asset(SITE / "static" / "site.css", "static", "site"),
        "emblem": dist.asset(emblem_dir / "emblem-64.png", "img", "emblem"),
        "emblem_large": dist.asset(emblem_dir / "emblem-256.png", "img", "emblem"),
        "favicon": dist.asset(emblem_dir / "favicon-32.png", "img", "favicon"),
    }
    ctx["js_tag"] = f'<script src="{dist.asset(SITE / "static" / "site.js", "static", "site")}" defer></script>'

    models_json = manifest(binary, cache)
    fingerprints = {m["id"]: m["fingerprint"] for m in json.loads(models_json.read_text())["models"]}
    sprites = sound_sprites(cache)
    data = review_data(models_json, sprites)

    home(dist, ctx, cache)
    privacy(dist, ctx)
    log(f"model sheets ({len(data['MODELS'])}) ...")
    n_models, n_types, n_sounds = catalogue(dist, ctx, data, binary, cache, sprites, fingerprints,
                                            args.refresh_pictures, args.limit)
    moved(dist, ctx)
    check_links(dist.root)
    size = sum(f.stat().st_size for f in dist.root.rglob("*") if f.is_file())
    print(f"site: {dist.root} — {n_models} models, {n_types} types with {n_sounds} sounds, "
          f"{sum(1 for f in dist.root.rglob('*') if f.is_file())} files, {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
