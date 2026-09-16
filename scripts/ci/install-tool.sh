#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 1 ]]; then
  echo 'Usage: install-tool.sh actionlint|mdbook|cargo-deny|cargo-fuzz' >&2
  exit 2
fi
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo 'These pinned CI binaries support Linux amd64 only.' >&2
  exit 2
fi
tool_name="$1"
case "$tool_name" in
  actionlint)
    tool_url='https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz'
    tool_sha='8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8'
    ;;
  mdbook)
    tool_url='https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-musl.tar.gz'
    tool_sha='5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89'
    ;;
  cargo-deny)
    tool_url='https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz'
    tool_sha='9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f'
    ;;
  cargo-fuzz)
    tool_url='https://github.com/rust-fuzz/cargo-fuzz/releases/download/0.13.2/cargo-fuzz-0.13.2-x86_64-unknown-linux-musl.tar.gz'
    tool_sha='b5b704018b63e0f151c17a057ac53b5111e1db545d1b9f72fee79f08a545931c'
    ;;
  *) echo "Unsupported tool: $tool_name" >&2; exit 2 ;;
esac

tool_dir="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/hypermind-${tool_name}.XXXXXX")"
tool_archive="$tool_dir/release.tar.gz"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  --connect-timeout 15 --max-time 180 "$tool_url" --output "$tool_archive"
printf '%s  %s\n' "$tool_sha" "$tool_archive" | sha256sum --check --strict >&2
mapfile -t tool_members < <(tar -tzf "$tool_archive" | awk -F / -v name="$tool_name" '$NF == name')
if [[ ${#tool_members[@]} != 1 ]]; then
  echo "Expected exactly one $tool_name binary in the verified release." >&2
  exit 1
fi
tool_member="${tool_members[0]}"
case "/$tool_member/" in
  *'/../'*|//* ) echo 'Unsafe archive member.' >&2; exit 1 ;;
esac
tar --extract --gzip --file "$tool_archive" --directory "$tool_dir" \
  --no-same-owner --no-same-permissions -- "$tool_member"
if [[ ! -f "$tool_dir/$tool_member" || -L "$tool_dir/$tool_member" ]]; then
  echo 'Release binary is not a regular file.' >&2
  exit 1
fi
mkdir "$tool_dir/bin"
install -m 0755 "$tool_dir/$tool_member" "$tool_dir/bin/$tool_name"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$tool_dir/bin" >> "$GITHUB_PATH"
fi
printf '%s\n' "$tool_dir/bin/$tool_name"
