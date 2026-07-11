use enpitsu::config::{Config, InitResult};
use enpitsu::dictionary::Dictionary;
use enpitsu::engine::{ConversionEngine, EngineCommand};
use enpitsu::katakana;
use enpitsu::paths;
use enpitsu::user_dictionary::UserDictionary;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// `--name value` 形式の引数の値を取り出す。
fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|pos| args.get(pos + 1).map(|s| s.as_str()))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --init-config: デフォルト設定ファイルを生成して終了する
    if args.iter().any(|a| a == "--init-config") {
        let path = paths::config_file();
        match Config::init_file(&path) {
            Ok(InitResult::Created) => {
                println!("設定ファイルを作成しました: {}", path.display());
            }
            Ok(InitResult::AlreadyExists) => {
                println!("設定ファイルは既に存在します: {}", path.display());
            }
            Err(e) => {
                eprintln!("設定ファイルの作成に失敗: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // 設定ファイルをデフォルトパスから読み込む（無ければデフォルト値）
    let config_path = paths::config_file();
    let config_exists = config_path.exists();
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("設定の読み込みに失敗（デフォルト設定を使用）: {e}");
        Config::default_config()
    });
    if config_exists {
        eprintln!("設定を読み込みました: {}", config_path.display());
    }

    // 辞書の解決順: --dict 引数 > config.system_dict_path > デフォルトパス
    let dict_path: Option<PathBuf> = arg_value(&args, "--dict")
        .map(PathBuf::from)
        .or_else(|| config.system_dict_path.as_ref().map(PathBuf::from))
        .or_else(|| {
            let p = paths::default_dict_file();
            p.exists().then_some(p)
        });

    let dict = match dict_path {
        Some(path) => match Dictionary::load_from_file(&path) {
            Ok(d) => {
                eprintln!("辞書を読み込みました: {}", path.display());
                Some(d)
            }
            Err(e) => {
                eprintln!("辞書の読み込みに失敗: {e}");
                None
            }
        },
        None => {
            eprintln!("辞書が見つかりません（ローマ字→かな変換のみ動作します）");
            None
        }
    };

    // ユーザー辞書の解決順: --user-dict 引数 > (auto_learn 有効時) デフォルトパス
    let user_dict_path: Option<PathBuf> = arg_value(&args, "--user-dict")
        .map(PathBuf::from)
        .or_else(|| config.auto_learn.then(paths::user_dict_file));

    let user_dict = match user_dict_path {
        Some(ref path) => match UserDictionary::load(path) {
            Ok(ud) => {
                eprintln!("ユーザー辞書を読み込みました: {}", path.display());
                Some(ud)
            }
            Err(e) => {
                eprintln!("ユーザー辞書の読み込みに失敗: {e}");
                None
            }
        },
        None => None,
    };

    let has_dict = dict.is_some();
    let mut engine = ConversionEngine::new_with_user_dict(dict, user_dict);

    println!("Enpitsu - ローマ字→かな変換デモ");
    if has_dict {
        println!("辞書検索モード: ローマ字を入力すると漢字候補も表示します。");
    }
    println!("ローマ字を入力して Enter で変換します。");
    println!("空行または Ctrl+C で終了します。");
    println!();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.is_empty() {
            break;
        }

        // 各文字を InsertChar で処理
        let mut last_output = None;
        for ch in line.chars() {
            last_output = Some(engine.process(EngineCommand::InsertChar(ch)));
        }
        if let Some(ref out) = last_output {
            let _ = writeln!(
                stdout,
                "  composing: '{}' (cursor_pos={})",
                out.display, out.cursor_pos
            );
        }

        // 辞書ありの場合: Convert → 候補があれば表示してから Commit
        // 辞書なしの場合: Commit でひらがな確定
        if has_dict {
            let output = engine.process(EngineCommand::Convert);
            if engine.candidates().is_some() {
                // 候補あり: reading からひらがな・カタカナを表示
                let hiragana = engine.reading().to_string();
                let katakana_display = katakana::to_katakana(&hiragana);
                let _ = writeln!(stdout, "  ひらがな: {hiragana}");
                let _ = writeln!(stdout, "  カタカナ: {katakana_display}");

                let candidates = engine.candidates().unwrap();
                let _ = writeln!(
                    stdout,
                    "  変換候補: {}",
                    candidates
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(" / ")
                );

                let commit_output = engine.process(EngineCommand::Commit);
                let _ = writeln!(stdout, "  確定: {}", commit_output.committed);
            } else {
                // 候補なし: Convert がひらがなを自動確定
                let hiragana = &output.committed;
                let katakana_display = katakana::to_katakana(hiragana);
                let _ = writeln!(stdout, "  ひらがな: {hiragana}");
                let _ = writeln!(stdout, "  カタカナ: {katakana_display}");
                let _ = writeln!(stdout, "  変換候補: (なし)");
            }
        } else {
            let output = engine.process(EngineCommand::Commit);
            let hiragana = &output.committed;
            let katakana_display = katakana::to_katakana(hiragana);
            let _ = writeln!(stdout, "  ひらがな: {hiragana}");
            let _ = writeln!(stdout, "  カタカナ: {katakana_display}");
        }

        let _ = writeln!(stdout);
    }

    // ユーザー辞書の保存
    if let Some(ref path) = user_dict_path
        && let Some(ud) = engine.user_dict_mut()
        && ud.is_dirty()
    {
        match ud.save(path) {
            Ok(()) => eprintln!("ユーザー辞書を保存しました: {}", path.display()),
            Err(e) => eprintln!("ユーザー辞書の保存に失敗: {e}"),
        }
    }
}
