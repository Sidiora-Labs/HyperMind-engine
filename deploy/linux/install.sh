#!/usr/bin/env bash
set -euo pipefail

check_only=false
if [[ ${1:-} == --check ]]; then
  check_only=true
  shift
fi
if [[ $# != 1 ]]; then
  printf 'Usage: %s [--check] /absolute/path/to/hm\n' "$0" >&2
  exit 2
fi
[[ $(uname -s) == Linux ]] || { echo 'This installer requires Linux.' >&2; exit 1; }
binary=$(realpath -- "$1")
unit_source=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/hypermind.service
[[ -f $binary && -x $binary ]] || { echo 'Supply an executable hm binary.' >&2; exit 1; }
"$binary" --version
[[ -f $unit_source ]] || { echo 'Missing repository service unit.' >&2; exit 1; }
for dependency in systemctl install runuser getent useradd cmp; do
  command -v "$dependency" >/dev/null || { echo "Missing prerequisite: $dependency" >&2; exit 1; }
done
if getent passwd hypermind >/dev/null && [[ $(id -gn hypermind) != hypermind ]]; then
  echo 'Existing hypermind account must have primary group hypermind; no account or unit was changed.' >&2
  exit 1
fi
if $check_only; then
  echo 'Preflight passed. Install target: /usr/local/bin/hm; state: /var/lib/hypermind; unit: /etc/systemd/system/hypermind.service.'
  echo 'No files, users, or services were changed.'
  exit 0
fi
[[ $EUID == 0 ]] || { echo 'Run the installer with sudo, or use --check.' >&2; exit 1; }
if systemctl is-active --quiet hypermind.service; then
  echo 'Stop hypermind.service before installing or upgrading.' >&2
  exit 1
fi
unit_target=/etc/systemd/system/hypermind.service
if [[ -e $unit_target ]] && ! cmp -s -- "$unit_source" "$unit_target"; then
  echo 'Existing service unit differs; preserve it and install the binary manually.' >&2
  exit 1
fi
if ! getent passwd hypermind >/dev/null; then
  useradd --system --user-group --home-dir /var/lib/hypermind --no-create-home --shell /usr/sbin/nologin hypermind
fi
[[ $(id -u hypermind) != 0 ]] || { echo 'The service account must not be root.' >&2; exit 1; }
service_group=$(id -gn hypermind)
install -d -o hypermind -g "$service_group" -m 0700 /var/lib/hypermind
install -d -o root -g "$service_group" -m 0750 /etc/hypermind
if [[ $binary != /usr/local/bin/hm ]]; then
  install -o root -g root -m 0755 -- "$binary" /usr/local/bin/hm
fi
runuser -u hypermind -- /usr/local/bin/hm init --path /var/lib/hypermind --if-missing --json
install -o root -g root -m 0644 -- "$unit_source" "$unit_target"
systemctl daemon-reload
echo 'Installed; the daemon has not been started.'
echo 'Start explicitly: sudo systemctl enable --now hypermind.service'
echo 'Verify: sudo -u hypermind /usr/local/bin/hm doctor --config /var/lib/hypermind/hypermind.conf --require-healthy --json'
