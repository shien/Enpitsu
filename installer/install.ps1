# Enpitsu インストーラー
# 管理者権限で実行すること
#
# インストール:   .\install.ps1
# アンインストール: .\install.ps1 -Uninstall

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$DllName = "enpitsu.dll"
$DllSource = Join-Path $PSScriptRoot "..\target\release\$DllName"

if ($Uninstall) {
    if (-not (Test-Path $DllSource)) {
        Write-Host "エラー: DLL が見つかりません: $DllSource"
        exit 1
    }
    Write-Host "Enpitsu をアンインストールしています..."
    regsvr32 /u /s $DllSource
    if ($LASTEXITCODE -eq 0) {
        Write-Host "アンインストール完了。"
    } else {
        Write-Host "エラー: regsvr32 /u が失敗しました (exit code: $LASTEXITCODE)"
        exit 1
    }
} else {
    if (-not (Test-Path $DllSource)) {
        Write-Host "エラー: DLL が見つかりません: $DllSource"
        Write-Host "先に 'cargo build --release' を実行してください。"
        exit 1
    }
    Write-Host "Enpitsu をインストールしています..."
    Write-Host "DLL: $DllSource"
    regsvr32 /s $DllSource
    if ($LASTEXITCODE -eq 0) {
        Write-Host "インストール完了。"
        Write-Host "Windows の設定 → 時刻と言語 → 言語と地域 → 日本語 → 言語のオプション → インストールされているキーボード → キーボードの追加 から Enpitsu を追加してください。"
    } else {
        Write-Host "エラー: regsvr32 が失敗しました (exit code: $LASTEXITCODE)"
        exit 1
    }
}
