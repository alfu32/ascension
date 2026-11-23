#!/usr/bin/env bash
# Publish site/dist/ to the server, atomically (task 135).
#
#   site/deploy.sh             build if site/dist/ is missing, upload, switch
#   site/deploy.sh --build     build first even if site/dist/ exists
#   site/deploy.sh --rollback  switch back to the previous upload
#
# Each upload goes to a new folder /var/www/ascension-site/releases/<time>; the symlink
# /var/www/ascension-site/current is then replaced in one rename, so a visitor never sees a half
# copied site. The previous uploads are kept (the last five) for --rollback.
set -euo pipefail

HOST="${ASCENSION_SITE_HOST:-ubuntu@89.168.88.241}"
KEY="${ASCENSION_SITE_KEY:-$HOME/oraci/ed25519-oraci-1}"
ROOT="/var/www/ascension-site"
KEEP=5

here="$(cd "$(dirname "$0")" && pwd)"
dist="$here/dist"
ssh_cmd=(ssh -i "$KEY" -o BatchMode=yes)

remote() { "${ssh_cmd[@]}" "$HOST" "$@"; }

if [[ "${1:-}" == "--rollback" ]]; then
    remote "set -e; cd $ROOT/releases
        now=\$(basename \"\$(readlink $ROOT/current)\")
        prev=\$(ls -1 | sort | grep -B1 -x \"\$now\" | head -n1)
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
remote "test -d $ROOT && test -w $ROOT" || {
    echo "$ROOT does not exist on the server or is not writable: see site/README.md" >&2
    exit 1
}
remote "mkdir -p $ROOT/releases"
prev="$(remote "readlink $ROOT/current 2>/dev/null || true")"
link_dest=()
if [[ -n "$prev" ]]; then
    link_dest=(--link-dest="$ROOT/$prev")  # unchanged files are hard links, not uploads
fi
rsync -rlt --delete --chmod=D755,F644 -e "${ssh_cmd[*]}" "${link_dest[@]}" \
    "$dist/" "$HOST:$ROOT/releases/$stamp/"
remote "set -e
    ln -sfn releases/$stamp $ROOT/current.new && mv -T $ROOT/current.new $ROOT/current
    cd $ROOT/releases && ls -1 | sort | head -n -$KEEP | xargs -r rm -rf --"
echo "published releases/$stamp (previous: ${prev:-none})"
