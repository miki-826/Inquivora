#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn オーバーラップの重複部分は除去される() {
        let prev = "8月から試験導入を開始します。";
        let next = "を開始します。詳細は来週共有します。";
        assert_eq!(dedupe_overlap(prev, next), "詳細は来週共有します。");
    }

    #[test]
    fn 重複がなければそのまま返る() {
        assert_eq!(
            dedupe_overlap("前の発言です。", "全く別の発言です。"),
            "全く別の発言です。"
        );
    }

    #[test]
    fn 短すぎる一致は除去しない() {
        assert_eq!(dedupe_overlap("終わります。", "。次の議題です。"), "。次の議題です。");
    }

    #[test]
    fn 新テキスト全体が重複なら空文字になる() {
        assert_eq!(dedupe_overlap("これは既に書いた内容です。", "書いた内容です。"), "");
    }

    #[test]
    fn 前後の空白を無視して比較する() {
        let prev = "導入を開始します。 ";
        let next = " 開始します。次へ進みます。";
        assert_eq!(dedupe_overlap(prev, next), "次へ進みます。");
    }

    #[test]
    fn 前セグメントが空なら新テキストを返す() {
        assert_eq!(dedupe_overlap("", "新しい発言"), "新しい発言");
    }
}
