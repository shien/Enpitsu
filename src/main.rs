use enpitsu::dictionary::Dictionary;
use enpitsu::engine::{ConversionEngine, EngineCommand};
use enpitsu::katakana;
use enpitsu::user_dictionary::UserDictionary;
use std::io::{self, BufRead, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --dict オプションで辞書ファイルを指定
    let dict = if let Some(pos) = args.iter().position(|a| a == "--dict") {
        let Some(path) = args.get(pos + 1) else {
            eprintln!("エラー: --dict の後に辞書ファイルパスを指定してください");
            std::process::exit(1);
        };
        match Dictionary::load_from_file(Path::new(path)) {
            Ok(d) => {
                eprintln!("辞書を読み込みました: {path}");
                Some(d)
            }
            Err(e) => {
                eprintln!("辞書の読み込みに失敗: {e}");
                None
            }
        }
    } else {
        None
    };

    // --user-dict オプションでユーザー辞書ファイルを指定
    let user_dict_path = args
        .iter()
        .position(|a| a == "--user-dict")
        .and_then(|pos| args.get(pos + 1).map(|s| s.as_str()));

    let user_dict = if let Some(path) = user_dict_path {
        match UserDictionary::load(Path::new(path)) {
            Ok(ud) => {
                eprintln!("ユーザー辞書を読み込みました: {path}");
                Some(ud)
            }
            Err(e) => {
                eprintln!("ユーザー辞書の読み込みに失敗: {e}");
                None
            }
        }
    } else {
        None
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
        for ch in line.chars() {
            engine.process(EngineCommand::InsertChar(ch));
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
    if let Some(path) = user_dict_path
        && let Some(ud) = engine.user_dict_mut()
        && ud.is_dirty()
    {
        match ud.save(Path::new(path)) {
            Ok(()) => eprintln!("ユーザー辞書を保存しました: {path}"),
            Err(e) => eprintln!("ユーザー辞書の保存に失敗: {e}"),
        }
    }
}
