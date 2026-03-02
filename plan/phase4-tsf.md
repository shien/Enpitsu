# Phase 4: TSF (Text Services Framework) 連携

## 目標

Windows の Text Services Framework (TSF) にIMEとして登録し、
任意のアプリケーションで日本語入力ができる最小限の状態にする。

## 背景

TSF は Windows の入力メソッドフレームワーク。IME は COM DLL として実装し、
以下のインターフェースを実装する必要がある:

- `ITfTextInputProcessor`: IME のメインインターフェース
- `ITfKeyEventSink`: キーボードイベントの受信
- `ITfCompositionSink`: 変換中テキストの管理

## 前提

- `windows` crate (Microsoft 公式 Rust バインディング) を使用
- ビルド成果物は DLL (`cdylib`)

## タスク

### 4.1 プロジェクト設定

- [ ] `Cargo.toml` に `windows` crate を追加
- [ ] `[lib]` セクションで `crate-type = ["cdylib", "rlib"]` を設定
- [ ] GUID の生成（IME 識別用の CLSID, Profile GUID）

**動作確認:**
- `cargo build` がエラーなく完了すること
- `cargo test` で既存テストが引き続きパスすること

### 4.2 COM DLL エントリポイント

- [ ] `src/lib.rs` に DLL エクスポート関数:
  - `DllGetClassObject`: COM オブジェクトのファクトリ
  - `DllCanUnloadNow`: アンロード可否
  - `DllRegisterServer` / `DllUnregisterServer`: レジストリ登録
- [ ] `ClassFactory` の実装 (`IClassFactory`)
- [ ] テスト: COM オブジェクトの生成

**動作確認:**
- `cargo test` で COM オブジェクト生成のユニットテストがパスすること
- `cargo build` で DLL (`cdylib`) が生成されること

### 4.3 ITfTextInputProcessor の実装

- [ ] `TextService` 構造体
- [ ] `Activate` / `Deactivate`: IME の有効化・無効化
- [ ] `ITfThreadMgrEventSink`: スレッドマネージャイベント
- [ ] テスト: Activate/Deactivate のライフサイクル

**動作確認:**
- `cargo test` で Activate/Deactivate ライフサイクルのテストがパスすること

### 4.4 キーイベント処理

- [ ] `ITfKeyEventSink` の実装
- [ ] `OnKeyDown` / `OnKeyUp`: キー入力のハンドリング
- [ ] `OnTestKeyDown`: キー入力を処理するか判定
- [ ] Phase 3 の `ConversionEngine` と接続
- [ ] テスト: キーイベントからエンジンへの変換

**動作確認:**
- `cargo test` でキーイベント処理のユニットテストがパスすること
- キーイベント → `EngineCommand` への変換が正しいことをテストで確認

### 4.5 Composition 管理

- [ ] `ITfCompositionSink` の実装
- [ ] 変換中テキスト（下線付き）の表示
- [ ] 確定テキストの挿入
- [ ] テスト: Composition の開始・更新・終了

**動作確認:**
- `cargo test` で Composition ライフサイクル（開始・更新・終了）のテストがパスすること

### 4.6 登録スクリプト

- [ ] `regsvr32` での登録/解除手順
- [ ] PowerShell スクリプトで簡易インストール
- [ ] 動作確認手順のドキュメント

**動作確認:**
- Windows 環境で `regsvr32 enpitsu.dll` が成功すること
- Windows の設定 → 入力メソッド一覧に Enpitsu が表示されること
- メモ帳でローマ字→ひらがな変換が動作することを手動確認

## 完了条件

- `regsvr32 enpitsu.dll` で IME がシステムに登録される
- Windows の入力メソッド一覧に表示される
- メモ帳等でローマ字→ひらがな変換が動作する（候補UIはまだなし）

## ファイル構成 (予定)

```
src/
├── lib.rs              # DLL エントリポイント + mod 宣言
├── text_service.rs     # ITfTextInputProcessor
├── key_event_sink.rs   # ITfKeyEventSink
├── composition.rs      # Composition 管理
├── registry.rs         # COM/TSF 登録処理
├── guids.rs            # CLSID, Profile GUID 定義
├── ...
installer/
└── install.ps1         # インストール用スクリプト
```

## 依存 crate (追加予定)

- `windows`: Win32 API, COM, TSF バインディング

## 注意事項

- TSF の開発・テストには Windows 環境が必要
- COM の実装は複雑なため、Microsoft の SampleIME を参考にする
- 64-bit / 32-bit 両方のビルドが必要になる場合がある
