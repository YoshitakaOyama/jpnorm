#!/usr/bin/env bash
# Sudachi 同義語辞書 (Apache-2.0) を取得する。
#
# yurenizer など他の OSS と同様、ライブラリ本体にはバンドルせず、
# 必要な利用者が個別にダウンロードする方式。
set -euo pipefail

DEST="${1:-data/synonyms.txt}"
URL="https://raw.githubusercontent.com/WorksApplications/SudachiDict/develop/src/main/text/synonyms.txt"

mkdir -p "$(dirname "$DEST")"
curl -fSL -o "$DEST" "$URL"
echo "downloaded: $DEST ($(wc -l <"$DEST") lines)"
