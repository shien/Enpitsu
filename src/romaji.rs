//! ローマ字からひらがなへの変換テーブルと変換ロジック。
//!
//! ローマ字入力を1文字ずつ受け取り、確定したひらがなと
//! まだ確定していない入力バッファを返す。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// ローマ字→ひらがな変換テーブルのエントリ
struct RomajiEntry {
    romaji: &'static str,
    hiragana: &'static str,
}

/// 変換テーブル（長い順にマッチさせるため、長いエントリを先に配置）
const ROMAJI_TABLE: &[RomajiEntry] = &[
    // 3文字のエントリ
    RomajiEntry {
        romaji: "sha",
        hiragana: "しゃ",
    },
    RomajiEntry {
        romaji: "shi",
        hiragana: "し",
    },
    RomajiEntry {
        romaji: "shu",
        hiragana: "しゅ",
    },
    RomajiEntry {
        romaji: "sho",
        hiragana: "しょ",
    },
    RomajiEntry {
        romaji: "chi",
        hiragana: "ち",
    },
    RomajiEntry {
        romaji: "cha",
        hiragana: "ちゃ",
    },
    RomajiEntry {
        romaji: "chu",
        hiragana: "ちゅ",
    },
    RomajiEntry {
        romaji: "cho",
        hiragana: "ちょ",
    },
    RomajiEntry {
        romaji: "tsu",
        hiragana: "つ",
    },
    RomajiEntry {
        romaji: "kya",
        hiragana: "きゃ",
    },
    RomajiEntry {
        romaji: "kyu",
        hiragana: "きゅ",
    },
    RomajiEntry {
        romaji: "kyo",
        hiragana: "きょ",
    },
    RomajiEntry {
        romaji: "gya",
        hiragana: "ぎゃ",
    },
    RomajiEntry {
        romaji: "gyu",
        hiragana: "ぎゅ",
    },
    RomajiEntry {
        romaji: "gyo",
        hiragana: "ぎょ",
    },
    RomajiEntry {
        romaji: "nya",
        hiragana: "にゃ",
    },
    RomajiEntry {
        romaji: "nyu",
        hiragana: "にゅ",
    },
    RomajiEntry {
        romaji: "nyo",
        hiragana: "にょ",
    },
    RomajiEntry {
        romaji: "hya",
        hiragana: "ひゃ",
    },
    RomajiEntry {
        romaji: "hyu",
        hiragana: "ひゅ",
    },
    RomajiEntry {
        romaji: "hyo",
        hiragana: "ひょ",
    },
    RomajiEntry {
        romaji: "bya",
        hiragana: "びゃ",
    },
    RomajiEntry {
        romaji: "byu",
        hiragana: "びゅ",
    },
    RomajiEntry {
        romaji: "byo",
        hiragana: "びょ",
    },
    RomajiEntry {
        romaji: "pya",
        hiragana: "ぴゃ",
    },
    RomajiEntry {
        romaji: "pyu",
        hiragana: "ぴゅ",
    },
    RomajiEntry {
        romaji: "pyo",
        hiragana: "ぴょ",
    },
    RomajiEntry {
        romaji: "mya",
        hiragana: "みゃ",
    },
    RomajiEntry {
        romaji: "myu",
        hiragana: "みゅ",
    },
    RomajiEntry {
        romaji: "myo",
        hiragana: "みょ",
    },
    RomajiEntry {
        romaji: "rya",
        hiragana: "りゃ",
    },
    RomajiEntry {
        romaji: "ryu",
        hiragana: "りゅ",
    },
    RomajiEntry {
        romaji: "ryo",
        hiragana: "りょ",
    },
    RomajiEntry {
        romaji: "jya",
        hiragana: "じゃ",
    },
    RomajiEntry {
        romaji: "jyu",
        hiragana: "じゅ",
    },
    RomajiEntry {
        romaji: "jyo",
        hiragana: "じょ",
    },
    RomajiEntry {
        romaji: "zya",
        hiragana: "じゃ",
    },
    RomajiEntry {
        romaji: "zyu",
        hiragana: "じゅ",
    },
    RomajiEntry {
        romaji: "zyo",
        hiragana: "じょ",
    },
    RomajiEntry {
        romaji: "dya",
        hiragana: "ぢゃ",
    },
    RomajiEntry {
        romaji: "dyu",
        hiragana: "ぢゅ",
    },
    RomajiEntry {
        romaji: "dyo",
        hiragana: "ぢょ",
    },
    // 小文字かな (x系)
    RomajiEntry {
        romaji: "xya",
        hiragana: "ゃ",
    },
    RomajiEntry {
        romaji: "xyu",
        hiragana: "ゅ",
    },
    RomajiEntry {
        romaji: "xyo",
        hiragana: "ょ",
    },
    RomajiEntry {
        romaji: "xtu",
        hiragana: "っ",
    },
    RomajiEntry {
        romaji: "xwa",
        hiragana: "ゎ",
    },
    // 小文字かな (l系 = x系の別名)
    RomajiEntry {
        romaji: "lya",
        hiragana: "ゃ",
    },
    RomajiEntry {
        romaji: "lyu",
        hiragana: "ゅ",
    },
    RomajiEntry {
        romaji: "lyo",
        hiragana: "ょ",
    },
    RomajiEntry {
        romaji: "ltu",
        hiragana: "っ",
    },
    RomajiEntry {
        romaji: "lwa",
        hiragana: "ゎ",
    },
    // 2文字のエントリ
    RomajiEntry {
        romaji: "ka",
        hiragana: "か",
    },
    RomajiEntry {
        romaji: "ki",
        hiragana: "き",
    },
    RomajiEntry {
        romaji: "ku",
        hiragana: "く",
    },
    RomajiEntry {
        romaji: "ke",
        hiragana: "け",
    },
    RomajiEntry {
        romaji: "ko",
        hiragana: "こ",
    },
    RomajiEntry {
        romaji: "sa",
        hiragana: "さ",
    },
    RomajiEntry {
        romaji: "si",
        hiragana: "し",
    },
    RomajiEntry {
        romaji: "su",
        hiragana: "す",
    },
    RomajiEntry {
        romaji: "se",
        hiragana: "せ",
    },
    RomajiEntry {
        romaji: "so",
        hiragana: "そ",
    },
    RomajiEntry {
        romaji: "ta",
        hiragana: "た",
    },
    RomajiEntry {
        romaji: "ti",
        hiragana: "ち",
    },
    RomajiEntry {
        romaji: "tu",
        hiragana: "つ",
    },
    RomajiEntry {
        romaji: "te",
        hiragana: "て",
    },
    RomajiEntry {
        romaji: "to",
        hiragana: "と",
    },
    RomajiEntry {
        romaji: "na",
        hiragana: "な",
    },
    RomajiEntry {
        romaji: "ni",
        hiragana: "に",
    },
    RomajiEntry {
        romaji: "nu",
        hiragana: "ぬ",
    },
    RomajiEntry {
        romaji: "ne",
        hiragana: "ね",
    },
    RomajiEntry {
        romaji: "no",
        hiragana: "の",
    },
    RomajiEntry {
        romaji: "ha",
        hiragana: "は",
    },
    RomajiEntry {
        romaji: "hi",
        hiragana: "ひ",
    },
    RomajiEntry {
        romaji: "hu",
        hiragana: "ふ",
    },
    RomajiEntry {
        romaji: "fu",
        hiragana: "ふ",
    },
    RomajiEntry {
        romaji: "he",
        hiragana: "へ",
    },
    RomajiEntry {
        romaji: "ho",
        hiragana: "ほ",
    },
    RomajiEntry {
        romaji: "ma",
        hiragana: "ま",
    },
    RomajiEntry {
        romaji: "mi",
        hiragana: "み",
    },
    RomajiEntry {
        romaji: "mu",
        hiragana: "む",
    },
    RomajiEntry {
        romaji: "me",
        hiragana: "め",
    },
    RomajiEntry {
        romaji: "mo",
        hiragana: "も",
    },
    RomajiEntry {
        romaji: "ya",
        hiragana: "や",
    },
    RomajiEntry {
        romaji: "yu",
        hiragana: "ゆ",
    },
    RomajiEntry {
        romaji: "yo",
        hiragana: "よ",
    },
    RomajiEntry {
        romaji: "ra",
        hiragana: "ら",
    },
    RomajiEntry {
        romaji: "ri",
        hiragana: "り",
    },
    RomajiEntry {
        romaji: "ru",
        hiragana: "る",
    },
    RomajiEntry {
        romaji: "re",
        hiragana: "れ",
    },
    RomajiEntry {
        romaji: "ro",
        hiragana: "ろ",
    },
    RomajiEntry {
        romaji: "wa",
        hiragana: "わ",
    },
    RomajiEntry {
        romaji: "wi",
        hiragana: "ゐ",
    },
    RomajiEntry {
        romaji: "we",
        hiragana: "ゑ",
    },
    RomajiEntry {
        romaji: "wo",
        hiragana: "を",
    },
    RomajiEntry {
        romaji: "ga",
        hiragana: "が",
    },
    RomajiEntry {
        romaji: "gi",
        hiragana: "ぎ",
    },
    RomajiEntry {
        romaji: "gu",
        hiragana: "ぐ",
    },
    RomajiEntry {
        romaji: "ge",
        hiragana: "げ",
    },
    RomajiEntry {
        romaji: "go",
        hiragana: "ご",
    },
    RomajiEntry {
        romaji: "za",
        hiragana: "ざ",
    },
    RomajiEntry {
        romaji: "zi",
        hiragana: "じ",
    },
    RomajiEntry {
        romaji: "zu",
        hiragana: "ず",
    },
    RomajiEntry {
        romaji: "ze",
        hiragana: "ぜ",
    },
    RomajiEntry {
        romaji: "zo",
        hiragana: "ぞ",
    },
    RomajiEntry {
        romaji: "da",
        hiragana: "だ",
    },
    RomajiEntry {
        romaji: "di",
        hiragana: "ぢ",
    },
    RomajiEntry {
        romaji: "du",
        hiragana: "づ",
    },
    RomajiEntry {
        romaji: "de",
        hiragana: "で",
    },
    RomajiEntry {
        romaji: "do",
        hiragana: "ど",
    },
    RomajiEntry {
        romaji: "ba",
        hiragana: "ば",
    },
    RomajiEntry {
        romaji: "bi",
        hiragana: "び",
    },
    RomajiEntry {
        romaji: "bu",
        hiragana: "ぶ",
    },
    RomajiEntry {
        romaji: "be",
        hiragana: "べ",
    },
    RomajiEntry {
        romaji: "bo",
        hiragana: "ぼ",
    },
    RomajiEntry {
        romaji: "pa",
        hiragana: "ぱ",
    },
    RomajiEntry {
        romaji: "pi",
        hiragana: "ぴ",
    },
    RomajiEntry {
        romaji: "pu",
        hiragana: "ぷ",
    },
    RomajiEntry {
        romaji: "pe",
        hiragana: "ぺ",
    },
    RomajiEntry {
        romaji: "po",
        hiragana: "ぽ",
    },
    RomajiEntry {
        romaji: "ja",
        hiragana: "じゃ",
    },
    RomajiEntry {
        romaji: "ji",
        hiragana: "じ",
    },
    RomajiEntry {
        romaji: "ju",
        hiragana: "じゅ",
    },
    RomajiEntry {
        romaji: "jo",
        hiragana: "じょ",
    },
    // ふぁ行
    RomajiEntry {
        romaji: "fa",
        hiragana: "ふぁ",
    },
    RomajiEntry {
        romaji: "fi",
        hiragana: "ふぃ",
    },
    RomajiEntry {
        romaji: "fe",
        hiragana: "ふぇ",
    },
    RomajiEntry {
        romaji: "fo",
        hiragana: "ふぉ",
    },
    // ゔ行
    RomajiEntry {
        romaji: "va",
        hiragana: "ゔぁ",
    },
    RomajiEntry {
        romaji: "vi",
        hiragana: "ゔぃ",
    },
    RomajiEntry {
        romaji: "vu",
        hiragana: "ゔ",
    },
    RomajiEntry {
        romaji: "ve",
        hiragana: "ゔぇ",
    },
    RomajiEntry {
        romaji: "vo",
        hiragana: "ゔぉ",
    },
    // 小文字かな (x系 2文字)
    RomajiEntry {
        romaji: "xa",
        hiragana: "ぁ",
    },
    RomajiEntry {
        romaji: "xi",
        hiragana: "ぃ",
    },
    RomajiEntry {
        romaji: "xu",
        hiragana: "ぅ",
    },
    RomajiEntry {
        romaji: "xe",
        hiragana: "ぇ",
    },
    RomajiEntry {
        romaji: "xo",
        hiragana: "ぉ",
    },
    // 小文字かな (l系 2文字)
    RomajiEntry {
        romaji: "la",
        hiragana: "ぁ",
    },
    RomajiEntry {
        romaji: "li",
        hiragana: "ぃ",
    },
    RomajiEntry {
        romaji: "lu",
        hiragana: "ぅ",
    },
    RomajiEntry {
        romaji: "le",
        hiragana: "ぇ",
    },
    RomajiEntry {
        romaji: "lo",
        hiragana: "ぉ",
    },
    // 1文字のエントリ
    // 注意: "n" と "nn" はテーブルに含めず、convert() 内で特別に処理する。
    // これにより "nn" → 「ん」+ バッファに n を残す動作を実現する。
    RomajiEntry {
        romaji: "a",
        hiragana: "あ",
    },
    RomajiEntry {
        romaji: "i",
        hiragana: "い",
    },
    RomajiEntry {
        romaji: "u",
        hiragana: "う",
    },
    RomajiEntry {
        romaji: "e",
        hiragana: "え",
    },
    RomajiEntry {
        romaji: "o",
        hiragana: "お",
    },
    // 長音記号・句読点
    RomajiEntry {
        romaji: "-",
        hiragana: "ー",
    },
    RomajiEntry {
        romaji: ",",
        hiragana: "、",
    },
    RomajiEntry {
        romaji: ".",
        hiragana: "。",
    },
];

/// ローマ字→ひらがな変換の結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionResult {
    /// 確定したひらがな文字列
    pub output: String,
    /// まだ確定していないバッファの残り
    pub pending: String,
}

/// 完全一致マップ: romaji → hiragana
static FULL_MATCH_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    ROMAJI_TABLE
        .iter()
        .map(|e| (e.romaji, e.hiragana))
        .collect()
});

/// 部分一致セット: エントリの全ての「真の接頭辞」を保持。
/// 例: "sha" → {"s", "sh"} を登録。エントリ自身は含まない。
static PREFIX_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    for entry in ROMAJI_TABLE {
        let romaji = entry.romaji;
        // romaji の各真の接頭辞を登録
        for len in 1..romaji.len() {
            set.insert(&romaji[..len]);
        }
    }
    set
});

/// 指定のバッファに対して O(1) でマッチ判定する。
/// バッファがエントリの真の接頭辞の場合は Partial（入力途中）。
fn try_match(buffer: &str) -> MatchResult {
    let has_longer_partial = PREFIX_SET.contains(buffer);
    let full_match = FULL_MATCH_MAP.get(buffer).copied();

    // より長いエントリが存在する場合は確定せず待機する。
    // これにより "n" は "na","ni" 等の可能性があるため即座に「ん」にならない。
    if has_longer_partial {
        MatchResult::Partial
    } else if let Some(hiragana) = full_match {
        MatchResult::Full { hiragana }
    } else {
        MatchResult::None
    }
}

enum MatchResult {
    /// 完全一致が見つかった
    Full { hiragana: &'static str },
    /// 部分一致がある（まだ入力途中の可能性）
    Partial,
    /// どのエントリにもマッチしない
    None,
}

/// ローマ字文字列をひらがなに変換する。
///
/// 入力全体を一括変換する。未確定部分は `pending` として返す。
pub fn convert(input: &str) -> ConversionResult {
    let input = input.to_lowercase();
    let mut output = String::new();
    let mut buffer = String::new();

    for ch in input.chars() {
        // nn の処理: 最初の n を「ん」として確定し、2番目の n はバッファに残す
        if buffer == "n" && ch == 'n' {
            output.push('ん');
            buffer.clear();
            buffer.push('n');
            continue;
        }

        // 促音の処理: 同じ子音が連続した場合（nn以外）
        if buffer.len() == 1
            && ch == buffer.chars().next().unwrap()
            && ch != 'a'
            && ch != 'i'
            && ch != 'u'
            && ch != 'e'
            && ch != 'o'
        {
            output.push('っ');
            buffer.clear();
        }

        buffer.push(ch);

        match try_match(&buffer) {
            MatchResult::Full { hiragana } => {
                output.push_str(hiragana);
                buffer.clear();
            }
            MatchResult::Partial => {
                // まだ入力途中なのでバッファに保持
            }
            MatchResult::None => {
                // 「n」+子音の場合、「ん」に変換してリトライ
                if buffer.len() >= 2 && buffer.starts_with('n') {
                    output.push('ん');
                    let remaining = buffer[1..].to_string();
                    buffer.clear();
                    buffer.push_str(&remaining);
                    // リトライ
                    match try_match(&buffer) {
                        MatchResult::Full { hiragana } => {
                            output.push_str(hiragana);
                            buffer.clear();
                        }
                        MatchResult::Partial => {}
                        MatchResult::None => {
                            // マッチしない文字はそのまま出力
                            output.push_str(&buffer);
                            buffer.clear();
                        }
                    }
                } else {
                    // マッチしない文字はそのまま出力
                    output.push_str(&buffer);
                    buffer.clear();
                }
            }
        }
    }

    ConversionResult {
        output,
        pending: buffer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 基本的な母音の変換 ===

    #[test]
    fn vowels() {
        let result = convert("aiueo");
        assert_eq!(result.output, "あいうえお");
        assert_eq!(result.pending, "");
    }

    // === 基本的な子音+母音の変換 ===

    #[test]
    fn ka_row() {
        let result = convert("kakikukeko");
        assert_eq!(result.output, "かきくけこ");
    }

    #[test]
    fn sa_row() {
        let result = convert("sasisuseso");
        assert_eq!(result.output, "さしすせそ");
    }

    #[test]
    fn ta_row() {
        let result = convert("tatituteto");
        assert_eq!(result.output, "たちつてと");
    }

    #[test]
    fn na_row() {
        let result = convert("naninuneno");
        assert_eq!(result.output, "なにぬねの");
    }

    #[test]
    fn ha_row() {
        let result = convert("hahihuheho");
        assert_eq!(result.output, "はひふへほ");
    }

    #[test]
    fn ma_row() {
        let result = convert("mamimumemo");
        assert_eq!(result.output, "まみむめも");
    }

    #[test]
    fn ya_row() {
        let result = convert("yayuyo");
        assert_eq!(result.output, "やゆよ");
    }

    #[test]
    fn ra_row() {
        let result = convert("rarirurero");
        assert_eq!(result.output, "らりるれろ");
    }

    #[test]
    fn wa_row() {
        let result = convert("wawo");
        assert_eq!(result.output, "わを");
    }

    // === 濁音・半濁音 ===

    #[test]
    fn ga_row() {
        let result = convert("gagigugego");
        assert_eq!(result.output, "がぎぐげご");
    }

    #[test]
    fn pa_row() {
        let result = convert("papipupepo");
        assert_eq!(result.output, "ぱぴぷぺぽ");
    }

    // === 拗音 ===

    #[test]
    fn kya_group() {
        let result = convert("kyakyukyo");
        assert_eq!(result.output, "きゃきゅきょ");
    }

    #[test]
    fn sha_group() {
        let result = convert("shashishu");
        assert_eq!(result.output, "しゃししゅ");
    }

    #[test]
    fn cha_group() {
        let result = convert("chachichucho");
        assert_eq!(result.output, "ちゃちちゅちょ");
    }

    // === 促音（っ） ===

    #[test]
    fn sokuon_kk() {
        let result = convert("kakko");
        assert_eq!(result.output, "かっこ");
    }

    #[test]
    fn sokuon_tt() {
        let result = convert("kitte");
        assert_eq!(result.output, "きって");
    }

    #[test]
    fn sokuon_pp() {
        let result = convert("nippon");
        assert_eq!(result.output, "にっぽ");
        assert_eq!(result.pending, "n");
    }

    // === 「ん」の処理 ===

    #[test]
    fn nn_explicit() {
        // nn → 最初の n が「ん」に確定、2番目の n はバッファに残る
        let result = convert("nn");
        assert_eq!(result.output, "ん");
        assert_eq!(result.pending, "n");
    }

    #[test]
    fn n_before_consonant() {
        let result = convert("kanta");
        assert_eq!(result.output, "かんた");
    }

    #[test]
    fn n_pending() {
        // 「n」単体は未確定（後ろに母音が来る可能性がある）
        let result = convert("n");
        assert_eq!(result.output, "");
        assert_eq!(result.pending, "n");
    }

    #[test]
    fn n_before_vowel_is_na() {
        let result = convert("na");
        assert_eq!(result.output, "な");
    }

    // === 単語レベルのテスト ===

    #[test]
    fn word_konnichiwa() {
        let result = convert("konnichiwa");
        assert_eq!(result.output, "こんにちわ");
    }

    #[test]
    fn word_tokyo() {
        let result = convert("toukyou");
        assert_eq!(result.output, "とうきょう");
    }

    #[test]
    fn word_nihongo() {
        let result = convert("nihongo");
        assert_eq!(result.output, "にほんご");
        assert_eq!(result.pending, "");
    }

    #[test]
    fn word_gakkou() {
        let result = convert("gakkou");
        assert_eq!(result.output, "がっこう");
    }

    // === エッジケース ===

    #[test]
    fn empty_input() {
        let result = convert("");
        assert_eq!(result.output, "");
        assert_eq!(result.pending, "");
    }

    #[test]
    fn uppercase_input() {
        let result = convert("AIUEO");
        assert_eq!(result.output, "あいうえお");
    }

    #[test]
    fn fu_alternative() {
        let result = convert("fu");
        assert_eq!(result.output, "ふ");
    }

    #[test]
    fn tsu_alternative() {
        let result = convert("tsu");
        assert_eq!(result.output, "つ");
    }

    #[test]
    fn pending_partial_input() {
        // 「k」は子音だけで、次の母音を待っている状態
        let result = convert("k");
        assert_eq!(result.output, "");
        assert_eq!(result.pending, "k");
    }

    #[test]
    fn pending_sh() {
        let result = convert("sh");
        assert_eq!(result.output, "");
        assert_eq!(result.pending, "sh");
    }

    // === 小文字かな (x系) ===

    #[test]
    fn komoji_xa() {
        assert_eq!(convert("xa").output, "ぁ");
        assert_eq!(convert("xi").output, "ぃ");
        assert_eq!(convert("xu").output, "ぅ");
        assert_eq!(convert("xe").output, "ぇ");
        assert_eq!(convert("xo").output, "ぉ");
    }

    #[test]
    fn komoji_xya() {
        assert_eq!(convert("xya").output, "ゃ");
        assert_eq!(convert("xyu").output, "ゅ");
        assert_eq!(convert("xyo").output, "ょ");
    }

    #[test]
    fn komoji_xtu() {
        assert_eq!(convert("xtu").output, "っ");
    }

    #[test]
    fn komoji_xwa() {
        assert_eq!(convert("xwa").output, "ゎ");
    }

    // === 小文字かな (l系 = x系の別名) ===

    #[test]
    fn komoji_la() {
        assert_eq!(convert("la").output, "ぁ");
        assert_eq!(convert("li").output, "ぃ");
        assert_eq!(convert("lu").output, "ぅ");
        assert_eq!(convert("le").output, "ぇ");
        assert_eq!(convert("lo").output, "ぉ");
    }

    #[test]
    fn komoji_lya() {
        assert_eq!(convert("lya").output, "ゃ");
        assert_eq!(convert("lyu").output, "ゅ");
        assert_eq!(convert("lyo").output, "ょ");
    }

    #[test]
    fn komoji_ltu() {
        assert_eq!(convert("ltu").output, "っ");
    }

    #[test]
    fn komoji_lwa() {
        assert_eq!(convert("lwa").output, "ゎ");
    }

    // === ぢゃ行 ===

    #[test]
    fn dya_group() {
        assert_eq!(convert("dya").output, "ぢゃ");
        assert_eq!(convert("dyu").output, "ぢゅ");
        assert_eq!(convert("dyo").output, "ぢょ");
    }

    // === じゃ行 (zy 系 = jya 系の別名) ===

    #[test]
    fn zya_group() {
        assert_eq!(convert("zya").output, "じゃ");
        assert_eq!(convert("zyu").output, "じゅ");
        assert_eq!(convert("zyo").output, "じょ");
    }

    // === ふぁ行 ===

    #[test]
    fn fa_group() {
        assert_eq!(convert("fa").output, "ふぁ");
        assert_eq!(convert("fi").output, "ふぃ");
        assert_eq!(convert("fe").output, "ふぇ");
        assert_eq!(convert("fo").output, "ふぉ");
    }

    // === ゔ行 ===

    #[test]
    fn va_group() {
        assert_eq!(convert("va").output, "ゔぁ");
        assert_eq!(convert("vi").output, "ゔぃ");
        assert_eq!(convert("vu").output, "ゔ");
        assert_eq!(convert("ve").output, "ゔぇ");
        assert_eq!(convert("vo").output, "ゔぉ");
    }

    // === 長音記号 ===

    #[test]
    fn chouon() {
        assert_eq!(convert("-").output, "ー");
    }

    // === 句読点 ===

    #[test]
    fn kutouten() {
        assert_eq!(convert(",").output, "、");
        assert_eq!(convert(".").output, "。");
    }

    // === 拡充テーブルの単語テスト ===

    #[test]
    fn word_with_chouon() {
        let result = convert("ra-men");
        assert_eq!(result.output, "らーめ");
        assert_eq!(result.pending, "n");
    }

    #[test]
    fn word_with_kutouten() {
        let result = convert("sou,sou.");
        assert_eq!(result.output, "そう、そう。");
    }
}
