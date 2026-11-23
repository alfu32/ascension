#!/usr/bin/env bash
# Publish site/dist/ to the server, atomically (task 135).
#
#   site/deploy.sh             build if site/dist/ is missing, upload, switch
#   site/deploy.sh --build     build first even if site/dist/ exists
#   site/deploy.sh --rollback  switch back to the previous upload
#
# The site lives in the server user's home, like the other web services there:
# ~/ascension-site/releases/<time> holds each upload, and the symlink ~/ascension-site/current is
# replaced in one rename, so a visitor never sees a half copied site. The previous uploads are
# kept (the last five) for --rollback. The user service ascension-site.service (site/serve.py on
# 127.0.0.1:18090) serves `current`; nginx proxies to it. The first deploy installs and starts
# the service; later deploys update serve.py and restart it only if it changed.
set -euo pipefail

HOST="${ASCENSION_SITE_HOST:-ubuntu@89.168.88.241}"
KEY="${ASCENSION_SITE_KEY:-$HOME/oraci/ed25519-oraci-1}"
ROOT="ascension-site"   # in the server user's home
KEEP=5

here="$(cd "$(dirname "$0")" && pwd)"
dist="$here/dist"
ssh_cmd=(ssh -i "$KEY" -o BatchMode=yes)

remote() { "${ssh_cmd[@]}" "$HOST" "$@"; }

if [[ "${1:-}" == "--rollback" ]]; then
    remote "set -e
        now=\$(basename \"\$(readlink $ROOT/current)\")
        prev=\$(ls -1 $ROOT/releases | sort | grep -B1 -x \"\$now\" | head -n1)
        [ -n \"\$prev\" ] && [ \"\$prev\" != \"\$now\" ] || { echo 'no earlier upload' >&2; exit 1; }
        ln -sfn releases/\$prev $ROOT/current.new && mv -T $ROOT/current.new $ROOT/current
        echo \"current: \$prev (was \$now)\""
    exit 0
fi

if [[ "${1:-}" == "--build" || ! -f "$dist/index.html" ]]; then
    "${PYTHON:-python3}" "$here/build.py"  # PYTHON: the environment with markdown (README.md)
fi
for page in index privacy story units sounds; do
    [[ -f "$dist/$page.html" ]] || { echo "site/dist/$page.html is missing" >&2; exit 1; }
done

stamp="$(date -u +%Y%m%d-%H%M%S)"
home="$(remote 'echo $HOME')"
remote "mkdir -p $ROOT/releases .config/systemd/user"
prev="$(remote "readlink $ROOT/current 2>/dev/null || true")"
link_dest=()
if [[ -n "$prev" ]]; then
    link_dest=(--link-dest="$home/$ROOT/$prev")  # unchanged files are hard links, not uploads
fi
rsync -rlt --delete --chmod=D755,F644 -e "${ssh_cmd[*]}" "${link_dest[@]}" \
    "$dist/" "$HOST:$ROOT/releases/$stamp/"
remote "set -e
    ln -sfn releases/$stamp $ROOT/current.new && mv -T $ROOT/current.new $ROOT/current
    cd $ROOT/releases && ls -1 | sort | head -n -$KEEP | xargs -r rm -rf --"
# The server and its service: install or update, then (re)start when needed.
changed="$(rsync -i --chmod=F644 -e "${ssh_cmd[*]}" "$here/serve.py" "$HOST:$ROOT/serve.py")"
unit_changed="$(rsync -i --chmod=F644 -e "${ssh_cmd[*]}" "$here/systemd/ascension-site.service" \
    "$HOST:.config/systemd/user/ascension-site.service")"
remote "export XDG_RUNTIME_DIR=/run/user/\$(id -u)
    systemctl --user daemon-reload
    systemctl --user enable -q ascension-site.service
    if [ -n '$changed$unit_changed' ] || ! systemctl --user is-active -q ascension-site.service; then
        systemctl --user restart ascension-site.service
    fi
    sleep 1
    curl -s -o /dev/null -w 'local check: %{http_code}\n' http://127.0.0.1:18090/"
echo "published releases/$stamp (previous: ${prev:-none})"
