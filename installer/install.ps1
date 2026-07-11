# Enpitsu インストーラー（Windows）
# 管理者権限で実行すること
#
# インストール:       .\install.ps1
# インストール先変更:  .\install.ps1 -InstallDir "D:\Enpitsu"
# 辞書もダウンロード:  .\install.ps1 -DownloadDict
# アンインストール:    .\install.ps1 -Uninstall
# 完全アンインストール: .\install.ps1 -Uninstall -Purge   （設定・ユーザー辞書も削除）

param(
    [switch]$Uninstall,
    [switch]$Purge,
    [switch]$DownloadDict,
    [string]$InstallDir = (Join-Path $env:ProgramFiles "Enpitsu")
)

$ErrorActionPreference = "Stop"

$DllName = "enpitsu.dll"
$ExeName = "enpitsu.exe"
$RepoRoot = Join-Path $PSScriptRoot ".."
$DllSource = Join-Path $RepoRoot "target\release\$DllName"
$ExeSource = Join-Path $RepoRoot "target\release\$ExeName"
$InstalledDll = Join-Path $InstallDir $DllName
$AppDataDir = Join-Path $env:APPDATA "enpitsu"
$DictUrl = "https://skk-dev.github.io/dict/SKK-JISYO.L.gz"

# === 管理者権限チェック ===

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Host "エラー: 管理者権限が必要です。"
    Write-Host "PowerShell を「管理者として実行」してから再度お試しください。"
    exit 1
}

# === 辞書ダウンロード ===

function Get-SkkDict {
    param([string]$DestDir)

    $dest = Join-Path $DestDir "SKK-JISYO.L"
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    Write-Host "SKK 辞書をダウンロードしています: $DictUrl"

    $tmpGz = Join-Path $env:TEMP "enpitsu-SKK-JISYO.L.gz"
    try {
        Invoke-WebRequest -Uri $DictUrl -OutFile $tmpGz -UseBasicParsing
        $inStream = [System.IO.File]::OpenRead($tmpGz)
        $gzip = New-Object System.IO.Compression.GzipStream($inStream, [System.IO.Compression.CompressionMode]::Decompress)
        $outStream = [System.IO.File]::Create($dest)
        $gzip.CopyTo($outStream)
        $outStream.Close(); $gzip.Close(); $inStream.Close()
        Remove-Item $tmpGz -ErrorAction SilentlyContinue
        Write-Host "辞書を配置しました: $dest"
    } catch {
        Write-Host "警告: 辞書のダウンロードに失敗しました。"
        Write-Host "  手動で $DictUrl を取得し、次のパスに展開してください:"
        Write-Host "  $dest"
    }
}

# === アンインストール ===

if ($Uninstall) {
    Write-Host "Enpitsu をアンインストールしています..."

    # インストール先 DLL の登録解除
    if (Test-Path $InstalledDll) {
        regsvr32 /u /s $InstalledDll
        if ($LASTEXITCODE -eq 0) {
            Write-Host "登録を解除しました: $InstalledDll"
        } else {
            Write-Host "警告: regsvr32 /u が失敗しました (exit code: $LASTEXITCODE)"
        }
    } else {
        Write-Host "インストール済み DLL が見つかりません: $InstalledDll"
    }

    # 旧方式（target\release 直登録）の解除も試みる
    if ((Test-Path $DllSource) -and ((Resolve-Path $DllSource).Path -ne $InstalledDll)) {
        regsvr32 /u /s $DllSource | Out-Null
    }

    # インストールディレクトリの削除
    if (Test-Path $InstallDir) {
        try {
            Remove-Item -Recurse -Force $InstallDir -ErrorAction Stop
            Write-Host "削除しました: $InstallDir"
        } catch {
            Write-Host "DLL がロード中のため削除できませんでした。"
            Write-Host "PC を再起動後、次のフォルダを手動で削除してください: $InstallDir"
        }
    }

    # 設定・ユーザー辞書の扱い
    if ($Purge) {
        if (Test-Path $AppDataDir) {
            Remove-Item -Recurse -Force $AppDataDir -ErrorAction SilentlyContinue
            Write-Host "削除（-Purge）: $AppDataDir"
        }
    } else {
        Write-Host "設定・ユーザー辞書は残しました（削除するには -Purge を指定）: $AppDataDir"
    }

    Write-Host "アンインストール完了。"
    Write-Host "IME の DLL は各プロセスにロードされている場合があるため、PC の再起動を推奨します。"
    exit 0
}

# === インストール ===

if (-not (Test-Path $DllSource)) {
    Write-Host "エラー: DLL が見つかりません: $DllSource"
    Write-Host "先に 'cargo build --release' を実行してください。"
    exit 1
}

Write-Host "Enpitsu をインストールしています..."

# 旧方式（target\release 直登録）が残っていれば先に解除する
if ((Resolve-Path $DllSource).Path -ne $InstalledDll) {
    regsvr32 /u /s $DllSource | Out-Null
}

# 1. DLL の配置
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $DllSource $InstalledDll -Force
Write-Host "DLL を配置しました: $InstalledDll"

# 2. 辞書の配置（リポジトリの dict/ にファイルがあればコピー）
$DictSrc = Join-Path $RepoRoot "dict"
if ((Test-Path $DictSrc) -and (Get-ChildItem $DictSrc -ErrorAction SilentlyContinue)) {
    $DictDest = Join-Path $InstallDir "dict"
    New-Item -ItemType Directory -Force -Path $DictDest | Out-Null
    Copy-Item (Join-Path $DictSrc "*") $DictDest -Recurse -Force
    Write-Host "辞書を配置しました: $DictDest"
}

# 3. 辞書ダウンロード（オプション）
if ($DownloadDict) {
    Get-SkkDict -DestDir (Join-Path $InstallDir "dict")
}

# 4. DLL の登録（配置先の DLL を登録する）
regsvr32 /s $InstalledDll
if ($LASTEXITCODE -ne 0) {
    Write-Host "エラー: regsvr32 が失敗しました (exit code: $LASTEXITCODE)"
    exit 1
}
Write-Host "DLL を登録しました。"

# 5. 設定ファイルの生成（--init-config 経由で default_toml を単一ソースとする）
if (Test-Path $ExeSource) {
    & $ExeSource --init-config
} else {
    Write-Host "注意: $ExeSource が見つからないため、設定ファイルの自動生成をスキップしました。"
    Write-Host "  手動で生成するには: $ExeName --init-config"
}

Write-Host ""
Write-Host "インストール完了。"
Write-Host "Windows の設定 → 時刻と言語 → 言語と地域 → 日本語 → 言語のオプション → インストールされているキーボード → キーボードの追加 から Enpitsu を追加してください。"
