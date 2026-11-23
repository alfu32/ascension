#!/usr/bin/env python3
"""Build the public site of Ascension into site/dist/ (task 135).

    python3 site/build.py [--binary app/target/release/rise-of-man] [--cache site/cache]

Pages: index.html (README.md), privacy.html (the Store's privacy policy), story.html (the ages
and their main themes), units.html (every model's sheet), sounds.html (every type's sounds and
the shared cues). The model data, names and sound sprites come from the model review page's own
scripts (app/docs/art/review-page/build.py and sprites.py) run on the game's
`--gallery-manifest`; the pictures from `rise-of-man --model <id> --screenshot`. Pictures,
sprites and converted images are kept in the cache folder between runs.

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
        "any"]
KINDS = {"unit": "Units", "bld": "Buildings", "node": "Sites"}
KIND_ONE = {"unit": "unit", "bld": "building", "node": "site"}
EVENTS = [("select", "Selected"), ("move", "Move"), ("attack", "Attack"), ("build", "Build"),
          ("fire", "Firing"), ("fight", "Fighting"), ("moving", "Moving"), ("active", "Working"),
          ("train_done", "Unit ready"), ("build_done", "Build finished"),
          ("research_done", "Research finished")]
# Words that mark the story document's internal production notes; none may reach the page.
PRIVATE_WORDS = re.compile(r"\b(owner|sketch(es)?|accepted|rejected|v1|v2|verdict)\b", re.I)
NAV = [("story.html", "Story"), ("units.html", "Units & buildings"), ("sounds.html", "Sounds"),
       ("privacy.html", "Privacy")]


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
        for href, label in NAV)
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


def parse_story():
    """The public part of app/docs/ages-story.md: the theme, and per age its name, era, story
    and reasoning. The production notes (status, the rules for the pieces, what comes next)
    stay out."""
    text = (APP / "docs" / "ages-story.md").read_text()
    head, _, rest = text.partition("\n---\n")
    theme = head.split("\n\n")[1].strip()
    theme = theme.replace("The owner whistled it:", "It begins as a whistled tune:")
    ages_text = rest.split("\n---\n")[0]
    ages = []
    for block in re.split(r"^## ", ages_text, flags=re.M)[1:]:
        title, _, body = block.partition("\n")
        m = re.match(r"(\d+)\. (.+?) — (.+)$", title.strip())
        if not m:
            sys.exit(f"ages-story.md: unexpected age heading {title!r}")
        story = re.search(r'^\*Story:\* "(.*?)"\s*$', body, re.M | re.S)
        if not story:
            sys.exit(f"ages-story.md: no story for {title!r}")
        parts = []
        for para in re.split(r"\n\s*\n", body[:story.start()].strip()):
            pm = re.match(r"\*\*(.+?)\*\*\s*(\*\([^)]*\)\*\.?)?\s*(.*)$", para.strip(), re.S)
            if not pm:
                sys.exit(f"ages-story.md: unexpected paragraph in {title!r}: {para[:60]!r}")
            label = pm.group(1).rstrip(".").strip()
            if label.lower() == "interlude":
                continue  # the interlude is not on the page; its main theme is
            parts.append((label, clean(pm.group(3))))
        ages.append({"n": int(m.group(1)), "name": m.group(2), "era": m.group(3),
                     "story": " ".join(story.group(1).split()), "parts": parts})
    if len(ages) != 14:
        sys.exit(f"ages-story.md: expected 14 ages, found {len(ages)}")
    return clean(theme), ages


def clean(text: str) -> str:
    """Remove the production notes the story carries in parentheses."""
    text = re.sub(r"\s*\((v\d[^)]*)\)", "", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text


def inline_md(text: str) -> str:
    out = md(text).strip()
    return out


def story(dist: Dist, ctx: dict):
    theme, ages = parse_story()
    music = APP / "assets" / "music"
    title_src = dist.asset(music / "title.ogg", "music", "title")
    for a in ages:
        key = AGES[a["n"]]
        a["key"] = key
        a["audio"] = dist.asset(music / f"{key}.theme.ogg", "music", f"{key}.theme")
    nav = "".join(f'<li><a href="#age-{a["key"]}">{a["n"]}. {html.escape(a["name"])}</a></li>'
                  for a in ages)
    out = [f'''<header class="story-intro">
  <p class="eyebrow">The story and the music</p>
  <h1>Fourteen ages, one theme</h1>
  <div class="lead">{inline_md(theme)}</div>
  <p>Each age has its own story, shown when your people reach it, and its own main theme: the same
  two notes in the musical language of the age. Below, for every age: the forces at work, what
  the age is remembered for, how it sees itself, the art it speaks in, and how the theme sounds.</p>
  <div class="player">
    <span class="label">Main title — the theme, heroic and without an age</span>
    <audio controls preload="none" src="{title_src}"></audio>
  </div>
  <ul class="agenav">{nav}</ul>
</header>''']
    for a in ages:
        rows = "".join(f"<dt>{html.escape(label)}</dt><dd>{inline_md(text)}</dd>"
                       for label, text in a["parts"])
        out.append(f'''<section class="age" id="age-{a["key"]}">
  <h2><span class="num">{a["n"]}</span>{html.escape(a["name"])}<span class="era">{html.escape(a["era"])}</span></h2>
  <blockquote class="tale"><p>{html.escape(a["story"])}</p></blockquote>
  <div class="player">
    <span class="label">Main theme of the {html.escape(a["name"])} age</span>
    <audio controls preload="none" src="{a["audio"]}"></audio>
  </div>
  <dl>{rows}</dl>
</section>''')
    content = "\n".join(out)
    leak = PRIVATE_WORDS.search(re.sub(r"<[^>]+>", " ", content))
    if leak:
        sys.exit(f"story.html would carry a production note: {leak.group(0)!r} near "
                 f"{content[max(0, leak.start() - 80):leak.end() + 80]!r}")
    page(dist, ctx, "story.html", "The story — Ascension",
         "The fourteen ages of Ascension, their stories, and the main theme of each age.",
         content, "prose", ctx["js_tag"])
    return {a["key"]: a["name"] for a in ages}


def age_label(key: str, names: dict) -> str:
    return names.get(key, "Every age" if key == "any" else key.capitalize())


def entity_id(entity: str) -> str:
    return "e-" + slug(entity)


def units(dist: Dist, ctx: dict, data: dict, binary: Path, cache: Path, names: dict,
          fingerprints: dict, refresh: bool, limit: int | None):
    models = data["MODELS"]
    if limit:
        models = models[:limit]
    by_age: dict[str, dict[str, list]] = {}
    for m in models:
        kind = m["entity"].split(".", 1)[0]
        by_age.setdefault(m["age"], {}).setdefault(kind, []).append(m)
    order = sorted(by_age, key=lambda a: AGES.index(a) if a in AGES else 99)
    first_card = {}
    jump, blocks = [], []
    total = 0
    for age in order:
        label = age_label(age, names)
        count = sum(len(v) for v in by_age[age].values())
        total += count
        jump.append(f'<li><a href="#{age}">{html.escape(label)}<span class="n">{count}</span></a></li>')
        sections = []
        for kind in sorted(by_age[age], key=lambda k: list(KINDS).index(k) if k in KINDS else 9):
            cards = []
            for m in sorted(by_age[age][kind], key=lambda m: (m["name"], m["id"])):
                pics = sheet(binary, cache, m, fingerprints[m["id"]], refresh)
                urls = {w: dist.asset(p, "img/models", m["img"]) for w, p in pics.items()}
                w, h = image_size(pics[SHEET_WIDTHS[-1]])
                srcset = ", ".join(f"{u} {wd}w" for wd, u in urls.items())
                card_id = "m-" + slug(m["id"])
                first_card.setdefault(m["entity"], card_id)
                sounds = ""
                if m["entity"] in ctx["has_sounds"]:
                    sounds = (f'<p class="links"><a href="sounds.html#{entity_id(m["entity"])}">'
                              f'Hear its sounds</a></p>')
                alt = f"{m['name']}, {KIND_ONE.get(kind, kind)} of the {label} age: game camera, front, side and close views"
                cards.append(f'''<article class="card" id="{card_id}">
  <a class="pic" href="{urls[SHEET_WIDTHS[-1]]}"><img src="{urls[SHEET_WIDTHS[0]]}" srcset="{srcset}" sizes="(max-width: 960px) 100vw, 580px" width="{w}" height="{h}" loading="lazy" decoding="async" alt="{html.escape(alt)}"></a>
  <div class="body"><h3>{html.escape(m["name"])}</h3><p class="meta">{html.escape(KIND_ONE.get(kind, kind).capitalize())} · {html.escape(label)}</p>{sounds}</div>
</article>''')
            sections.append(f'<h3 class="kind">{KINDS.get(kind, kind)}</h3>\n<div class="grid">\n'
                            + "\n".join(cards) + "\n</div>")
        blocks.append(f'<section class="age-block" id="{age}"><h2>{html.escape(label)}'
                      f'<small>{count} models</small></h2>\n' + "\n".join(sections) + "</section>")
    content = f'''<header>
  <p class="eyebrow">Units &amp; buildings</p>
  <h1>Every model in the game</h1>
  <p class="lead">Buildings and units are built from parts inside the game and baked before play.
  Each sheet shows a model four ways: from the game's camera, from the front, from the side and
  close up. Buildings change their look as their owner advances through the ages.</p>
  <ul class="jump">{"".join(jump)}</ul>
</header>
{"".join(blocks)}'''
    page(dist, ctx, "units.html", "Units & buildings — Ascension",
         f"All {total} building and unit models of Ascension, age by age.", content)
    return first_card, total


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


def sounds_page(dist: Dist, ctx: dict, data: dict, sprites: Path, names: dict, first_card: dict):
    assignment = json.loads((APP / "tools" / "sounds" / "assignment.json").read_text())
    index = data["INDEX"]
    urls = {}
    for sprite in sorted({v[0] for v in index.values()}):
        urls[sprite] = dist.asset(sprites / "sounds" / f"{sprite}.ogg", "sounds", sprite)
    # Name and first age per type, from the models; a type without a model keeps its key's name.
    info = {}
    for m in data["MODELS"]:
        age = m["age"]
        old = info.get(m["entity"])
        if not old or AGES.index(age) < AGES.index(old["age"]):
            info[m["entity"]] = {"name": m["name"], "age": age}
    order = [k for k, _ in EVENTS]
    groups: dict[str, list] = {}
    for entity in sorted(assignment):
        events = {ev: ([x] if isinstance(x, str) else x) for ev, x in assignment[entity].items()}
        missing = [k for keys in events.values() for k in keys if k not in index]
        if missing:
            sys.exit(f"{entity}: sounds missing from the sprites: {missing[:5]}")
        meta = info.get(entity) or {"name": entity.split(".", 1)[1].replace("_", " ").capitalize(),
                                    "age": "any"}
        kind = entity.split(".", 1)[0]
        buttons = "".join(play_button(event_label(ev), events[ev]) for ev in sorted(
            events, key=lambda e: ((order.index(e) if e in order else 98), e)))
        model = (f'<a href="units.html#{first_card[entity]}">See the model</a>'
                 if entity in first_card else "")
        card = f'''<article class="snd" id="{entity_id(entity)}">
  <div class="who"><h3>{html.escape(meta["name"])}</h3><span class="meta">{KIND_ONE.get(kind, kind).capitalize()} · {html.escape(age_label(meta["age"], names))}</span>{model}</div>
  <div class="plays">{buttons}</div>
</article>'''
        groups.setdefault(meta["age"], []).append((list(KINDS).index(kind) if kind in KINDS else 9,
                                                   meta["name"], card))
    jump, blocks = [], []
    for age in sorted(groups, key=lambda a: AGES.index(a) if a in AGES else 99):
        label = age_label(age, names)
        cards = [c for _, _, c in sorted(groups[age])]
        jump.append(f'<li><a href="#{age}">{html.escape(label)}<span class="n">{len(cards)}</span></a></li>')
        blocks.append(f'<section class="age-block" id="{age}"><h2>{html.escape(label)}'
                      f'<small>{len(cards)} types</small></h2>\n<div class="snd-list">\n'
                      + "\n".join(cards) + "\n</div></section>")
    shared = []
    for group, keys in data["LIBRARY"]:
        buttons = "".join(play_button(sound_label(k), [k]) for k in keys)
        shared.append(f'<div class="lib-group"><h3>{html.escape(group)}</h3>'
                      f'<div class="plays">{buttons}</div></div>')
    sound_data = json.dumps({"index": index, "urls": urls}, separators=(",", ":"))
    content = f'''<header>
  <p class="eyebrow">Sounds</p>
  <h1>What every unit and building sounds like</h1>
  <p class="lead">Every sound in the game is made for it: effects are synthesised, voices spoken
  by a speech model at authoring time. Each unit and building type has its own lines and effects;
  press a button to hear one (where there are several variations, one is chosen at random, as in
  the game).</p>
  <p class="nojs">The sound buttons need JavaScript.</p>
  <ul class="jump"><li><a href="#shared">Shared cues</a></li>{"".join(jump)}</ul>
</header>
{"".join(blocks)}
<section class="age-block" id="shared"><h2>Shared cues<small>ambience, interface, activity</small></h2>
<p>The sounds that belong to no single type: the ambience of the world, the interface and its
announcer, and the activity of units at work.</p>
{"".join(shared)}
</section>
<script type="application/json" id="sound-data">{sound_data.replace("</", "<\\/")}</script>'''
    page(dist, ctx, "sounds.html", "Sounds — Ascension",
         "The voices and effects of every unit and building of Ascension, and the shared cues.",
         content, "", ctx["js_tag"])
    return len(assignment), len(index)


# ---------------------------------------------------------------- checks


def check_links(dist: Path):
    """Every internal link and file exists; nothing points at the private review page."""
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
    data = json.loads(re.search(r'id="sound-data">(.*?)</script>',
                                (dist / "sounds.html").read_text(), re.S).group(1).replace("<\\/", "</"))
    for url in data["urls"].values():
        if not (dist / url).exists():
            bad.append(f"sounds.html: missing sprite {url}")
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
    ctx["has_sounds"] = set(json.loads((APP / "tools" / "sounds" / "assignment.json").read_text()))

    home(dist, ctx, cache)
    privacy(dist, ctx)
    names = story(dist, ctx)
    log(f"model sheets ({len(data['MODELS'])}) ...")
    first_card, n_models = units(dist, ctx, data, binary, cache, names, fingerprints,
                                 args.refresh_pictures, args.limit)
    n_types, n_sounds = sounds_page(dist, ctx, data, sprites, names, first_card)
    check_links(dist.root)
    size = sum(f.stat().st_size for f in dist.root.rglob("*") if f.is_file())
    print(f"site: {dist.root} — {n_models} models, {n_types} types with {n_sounds} sounds, "
          f"{sum(1 for f in dist.root.rglob('*') if f.is_file())} files, {size / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
