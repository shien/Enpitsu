#!/usr/bin/env bash
#
# Enpitsu Linux インストーラ
#
# CLI バイナリ・辞書・設定を XDG Base Directory 準拠で配置する。
# root 権限は不要（既定でユーザーローカルにインストールする）。
#
# 使い方:
#   ./install.sh                     ~/.local 以下にインストール
#   ./install.sh --prefix <dir>      インストール先を変更（バイナリは <dir>/bin/enpitsu）
#   ./install.sh --download-dict     SKK 辞書をダウンロードして配置
#   ./install.sh --uninstall         アンインストール（設定・ユーザー辞書は残す）
#   ./install.sh --uninstall --purge 設定・ユーザー辞書も含めて完全に削除
#   ./install.sh --help              このヘルプを表示

set -euo pipefail

# === パス定義 ===

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PREFIX="$HOME/.local"
DO_UNINSTALL=0
DO_PURGE=0
DO_DOWNLOAD_DICT=0

# XDG 準拠のディレクトリ（Rust 側 src/paths.rs と一致させること）
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/enpitsu"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/enpitsu"

DICT_URL="https://skk-dev.github.io/dict/SKK-JISYO.L.gz"

usage() {
    sed -n '3,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

# === 引数パース ===

while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall)     DO_UNINSTALL=1 ;;
        --purge)         DO_PURGE=1 ;;
        --download-dict) DO_DOWNLOAD_DICT=1 ;;
        --prefix)
            shift
            [ $# -gt 0 ] || { echo "エラー: --prefix の後にディレクトリを指定してください" >&2; exit 1; }
            PREFIX="$1"
            ;;
        --help|-h)       usage; exit 0 ;;
        *)               echo "エラー: 不明なオプション: $1" >&2; usage; exit 1 ;;
    esac
    shift
done

BIN_DIR="$PREFIX/bin"
BIN_PATH="$BIN_DIR/enpitsu"

# === 辞書ダウンロード ===

download_dict() {
    local dest_dir="$1"
    local dest="$dest_dir/SKK-JISYO.L"
    mkdir -p "$dest_dir"
    echo "SKK 辞書をダウンロードしています: $DICT_URL"

    local tmp_gz
    tmp_gz="$(mktemp)"
    local ok=0
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DICT_URL" -o "$tmp_gz" && ok=1 || ok=0
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$DICT_URL" -O "$tmp_gz" && ok=1 || ok=0
    else
        echo "警告: curl / wget が見つかりません。辞書のダウンロードをスキップします。" >&2
    fi

    if [ "$ok" -eq 1 ] && gunzip -c "$tmp_gz" > "$dest" 2>/dev/null; then
        rm -f "$tmp_gz"
        echo "辞書を配置しました: $dest"
    else
        rm -f "$tmp_gz"
        echo "警告: 辞書のダウンロードに失敗しました。" >&2
        echo "  手動で $DICT_URL を取得し、次のパスに展開してください:" >&2
        echo "  $dest" >&2
    fi
}

# === アンインストール ===

if [ "$DO_UNINSTALL" -eq 1 ]; then
    echo "Enpitsu をアンインストールしています..."

    if [ -e "$BIN_PATH" ]; then
        rm -f "$BIN_PATH"
        echo "削除: $BIN_PATH"
    else
        echo "バイナリは見つかりませんでした: $BIN_PATH"
    fi

    # 辞書ディレクトリ（インストーラが配置したもの）を削除
    if [ -d "$DATA_DIR/dict" ]; then
        rm -rf "$DATA_DIR/dict"
        echo "削除: $DATA_DIR/dict"
    fi

    if [ "$DO_PURGE" -eq 1 ]; then
        rm -rf "$CONFIG_DIR" "$DATA_DIR"
        echo "削除（--purge）: $CONFIG_DIR"
        echo "削除（--purge）: $DATA_DIR"
    else
        echo "設定・ユーザー辞書は残しました（削除するには --purge を指定）:"
        echo "  $CONFIG_DIR"
        echo "  $DATA_DIR"
    fi

    echo "アンインストール完了。"
    exit 0
fi

# === インストール ===

BIN_SRC="$REPO_ROOT/target/release/enpitsu"
if [ ! -x "$BIN_SRC" ]; then
    echo "エラー: リリースバイナリが見つかりません: $BIN_SRC" >&2
    echo "先に 'cargo build --release' を実行してください。" >&2
    exit 1
fi

echo "Enpitsu をインストールしています..."

# 1. バイナリの配置
mkdir -p "$BIN_DIR"
install -m 0755 "$BIN_SRC" "$BIN_PATH"
echo "バイナリを配置しました: $BIN_PATH"

# 2. 辞書の配置（リポジトリの dict/ にファイルがあればコピー）
if [ -d "$REPO_ROOT/dict" ] && [ -n "$(ls -A "$REPO_ROOT/dict" 2>/dev/null)" ]; then
    mkdir -p "$DATA_DIR/dict"
    cp -r "$REPO_ROOT/dict/." "$DATA_DIR/dict/"
    echo "辞書を配置しました: $DATA_DIR/dict"
fi

# 3. 辞書ダウンロード（オプション）
if [ "$DO_DOWNLOAD_DICT" -eq 1 ]; then
    download_dict "$DATA_DIR/dict"
fi

# 4. 設定ファイルの生成（--init-config 経由で default_toml を単一ソースとする）
"$BIN_PATH" --init-config

# 5. PATH の確認
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        echo "注意: $BIN_DIR が PATH に含まれていません。"
        echo "シェルの設定ファイル（例: ~/.bashrc）に次の行を追加してください:"
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        ;;
esac

echo ""
echo "インストール完了。'enpitsu' で CLI を起動できます。"
