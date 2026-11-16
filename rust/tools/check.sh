#!/bin/bash
# 푸시 전 검사: CI(macOS, Windows x64·ARM64)에서 실패할 것을 이 Mac에서 먼저 잡는다.
# 사용: rust/tools/check.sh   (git pre-push 훅이 rust/ 변경이 있을 때 자동 실행)
# 필요: rustup 대상 x86_64/aarch64-pc-windows-msvc, Homebrew llvm·lld, cargo-xwin
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"

step() { printf '\n▶ %s\n' "$1"; }

step "Windows 경로 함정 검사(테스트의 join(\"a/b\") 금지: Windows에서 구분자가 섞인다)"
if grep -rnE '\.join\("[^"]*/[^"]*"\)' crates --include=*.rs; then
  echo "✖ 위 줄을 .join(\"a\").join(\"b\")로 나눠 쓰세요"; exit 1
fi

step "macOS 테스트"
cargo test --workspace -q

step "macOS clippy"
cargo clippy --workspace --all-targets -q -- -D warnings

for t in x86_64-pc-windows-msvc aarch64-pc-windows-msvc; do
  step "Windows clippy ($t)"
  cargo xwin clippy --workspace --all-targets -q --target "$t" -- -D warnings
done

printf '\n✔ 모든 검사 통과\n'
