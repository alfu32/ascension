# The public site

The static site at https://ascension-euw-1.duckdns.org/ (task 135): the project's README as the
home page, the privacy policy, the story of the ages with each age's main theme, every unit and
building model, and every sound. No cookies, no analytics, no scripts or fonts from elsewhere.

| File | What |
| --- | --- |
| `build.py` | builds the site from the repository into `dist/` (ignored by git) |
| `templates/page.html` | the page shell: header, navigation, footer |
| `static/site.css`, `static/site.js` | the look (navy and brass, light and dark) and the sound and music players |
| `deploy.sh` | uploads `dist/` to the server and switches to it atomically |
| `nginx/ascension.conf` | the server's nginx site: the static site and the game server's two paths |

## Pages

| Page | Made from |
| --- | --- |
| `index.html` | `README.md`; its pictures converted to WebP, links to files in the repository turned into links to GitHub |
| `privacy.html` | `packaging/microsoft-store-kit/documentation/PRIVACY.md` (the Store listing's privacy URL) |
| `story.html` | `app/docs/ages-story.md`: the theme and, for each age, its story and reasoning, with `app/assets/music/<age>.theme.ogg` and the main title `title.ogg`; the production notes are left out (the build stops if one gets through) |
| `units.html` | the gallery manifest and every model's sheet from the game (`rise-of-man --model <id> --screenshot`), names and ages as the model review page gives them |
| `sounds.html` | `app/tools/sounds/assignment.json` and the review page's sound sprites (`app/docs/art/review-page/sprites.py`), plus the shared cues |

`build.py` runs the review page's own `build.py` on the manifest and reads back the data it puts
in that page (models, names, ages, the sound index), then leaves every review element out.
Pictures, sounds, music, styles and scripts get a content hash in their file name, so the
server can let browsers keep them for a year while the pages are always revalidated.

## Building

Needs Python 3.12 or later, ffmpeg (with libwebp and libvorbis), an X11 display for the game's
screenshots, and a release build of the game from this checkout:

```sh
cd app && cargo build --release --locked -p rom-app && cd ..

# a throwaway Python environment with the one package the build uses
python3 -m venv /tmp/ascension-site-venv
/tmp/ascension-site-venv/bin/pip install "markdown==3.7"

/tmp/ascension-site-venv/bin/python site/build.py
```

Options: `--cache <dir>` (default `site/cache/`, ignored by git) keeps the model sheets, the
sound sprites and the converted pictures between runs. A model's sheet is taken again only when
its fingerprint changes; `--refresh-pictures` takes them all again (after a change to the
renderer). The first run takes the sheets of every model (a few seconds each) and bundles the
sounds (about ten minutes); later runs take a minute. `--limit N` builds the units page with the
first N models only, for a quick look.

The build checks every internal link, anchor and file, that no link leaves for anywhere but the
project on GitHub, and that no page carries an email address.

## Publishing

The site lives in the server user's home, like the other web services there. `site/deploy.sh`
(from the repository root) builds if `site/dist/` is missing, uploads it with `rsync` over ssh to
a new folder `~/ascension-site/releases/<time>` (unchanged files are hard links to the previous
upload) and switches the symlink `~/ascension-site/current` to it in one rename. The last five
uploads are kept; `site/deploy.sh --rollback` switches back to the previous one.
`site/deploy.sh --build` rebuilds first.

The site is served by `site/serve.py` (Python standard library: pages revalidated, hashed files
kept for a year, byte ranges for audio, the security headers) as the user service
`ascension-site.service` (`site/systemd/`) on `127.0.0.1:18090`. Each deploy copies `serve.py`
and the unit beside the site, enables the service and restarts it when either changed; a new
upload needs no restart (the server resolves `current` on every request).

The host and key default to the project's server; `ASCENSION_SITE_HOST` and
`ASCENSION_SITE_KEY` override them.

## One-time setup on the server (the owner, with sudo)

Only nginx needs sudo. The game server keeps its paths: nginx proxies `/ws` (the WebSocket every
game client connects to: a client given `https://ascension-euw-1.duckdns.org/` connects to
`wss://…/ws`) and `/status` to `rom serve` on `127.0.0.1:7400` exactly as before, with the same
upgrade headers and one-hour timeouts; every other path goes to the site's service.

```sh
# 1. from a development machine: publish the site (starts its service), and copy the new nginx
#    site beside the old one
site/deploy.sh
scp -i ~/oraci/ed25519-oraci-1 site/nginx/ascension.conf ubuntu@89.168.88.241:ascension-nginx/ascension.conf.site

# 2. on the server: keep the old config, install the new one, check it, reload
cp /etc/nginx/sites-available/ascension.conf ~/ascension-nginx/ascension.conf.before-site
sudo install -m 644 ~/ascension-nginx/ascension.conf.site /etc/nginx/sites-available/ascension.conf
sudo nginx -t && sudo systemctl reload nginx

# 3. check: the site, the game server's status, and a game client joining
curl -sI https://ascension-euw-1.duckdns.org/ | head -n 5
curl -s https://ascension-euw-1.duckdns.org/status | head -n 5
```

To go back: `sudo install -m 644 ~/ascension-nginx/ascension.conf.before-site
/etc/nginx/sites-available/ascension.conf`, `sudo nginx -t`, and reload.

Everyday: build (above), then `site/deploy.sh`; no sudo needed.
