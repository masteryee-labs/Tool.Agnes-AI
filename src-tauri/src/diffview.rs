//! 行級 diff 引擎：以 LCS（最長公共子序列）計算 before/after 的逐行差異，
//! 供 GUI 右側檔案變更面板渲染。純計算模組，無 I/O、無外部依賴。

/// DP 記憶體保險絲：LCS 動態規劃表的儲存格數上限（before 行數 × after 行數）。
/// 每格 4 bytes（u32），4_000_000 格 ≈ 16 MB；超過此值時 O(n×m) 的時間與記憶體
/// 都可能讓 GUI 凍結，故退化為「全刪＋全增」輸出，保證任何輸入都能即時回應。
pub const DIFF_DP_CELL_LIMIT: usize = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
}

/// 計算 before → after 的行級 diff。
///
/// - 空字串視為 0 行（`str::lines` 對 "" 回傳空迭代器）。
/// - CRLF 安全：`str::lines` 會剝除行尾 `\r`。
/// - `max_lines` 為輸出截斷上限：輸出行數截到 max_lines，但 stats 一律以
///   全量 diff 計數（截斷不影響 +N/-N 統計的正確性）。
/// - 行數乘積超過 [`DIFF_DP_CELL_LIMIT`] 時退化為「全刪＋全增」。
pub fn line_diff(before: &str, after: &str, max_lines: usize) -> (Vec<DiffLine>, DiffStats) {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    let mut full = compute_full_diff(&before_lines, &after_lines);

    let mut stats = DiffStats::default();
    for line in &full {
        match line.kind {
            DiffLineKind::Added => stats.added += 1,
            DiffLineKind::Removed => stats.removed += 1,
            DiffLineKind::Context => {}
        }
    }

    full.truncate(max_lines);
    (full, stats)
}

/// 全量 diff：LCS 回溯產生 Context/Removed/Added 序列；超限時退化為全刪＋全增。
fn compute_full_diff(before: &[&str], after: &[&str]) -> Vec<DiffLine> {
    let n = before.len();
    let m = after.len();

    // DP 保險絲：checked_mul 同時防 usize 乘法溢位
    let over_limit = match n.checked_mul(m) {
        Some(cells) => cells > DIFF_DP_CELL_LIMIT,
        None => true,
    };
    if over_limit {
        let mut out = Vec::with_capacity(n + m);
        out.extend(before.iter().map(|line| DiffLine {
            kind: DiffLineKind::Removed,
            text: (*line).to_string(),
        }));
        out.extend(after.iter().map(|line| DiffLine {
            kind: DiffLineKind::Added,
            text: (*line).to_string(),
        }));
        return out;
    }

    // LCS DP 表：dp[i*width + j] = before[..i] 與 after[..j] 的 LCS 長度
    let width = m + 1;
    let mut dp = vec![0u32; (n + 1) * width];
    for i in 1..=n {
        for j in 1..=m {
            dp[i * width + j] = if before[i - 1] == after[j - 1] {
                dp[(i - 1) * width + (j - 1)] + 1
            } else {
                dp[(i - 1) * width + j].max(dp[i * width + (j - 1)])
            };
        }
    }

    // 從 (n, m) 回溯到 (0, 0)，反向收集後反轉成正序
    let mut out = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && before[i - 1] == after[j - 1] {
            out.push(DiffLine {
                kind: DiffLineKind::Context,
                text: before[i - 1].to_string(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i * width + (j - 1)] >= dp[(i - 1) * width + j]) {
            out.push(DiffLine {
                kind: DiffLineKind::Added,
                text: after[j - 1].to_string(),
            });
            j -= 1;
        } else {
            out.push(DiffLine {
                kind: DiffLineKind::Removed,
                text: before[i - 1].to_string(),
            });
            i -= 1;
        }
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 測試共用的輸出上限：大到不會影響非截斷測試的結果。
    const NO_TRUNCATION: usize = 10_000;

    fn kinds(lines: &[DiffLine]) -> Vec<DiffLineKind> {
        lines.iter().map(|l| l.kind).collect()
    }

    #[test]
    fn identical_content_yields_zero_stats() {
        let src = "fn main() {\n    println!(\"hi\");\n}";
        let (lines, stats) = line_diff(src, src, NO_TRUNCATION);
        assert_eq!(stats.added, 0);
        assert_eq!(stats.removed, 0);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.kind == DiffLineKind::Context));
    }

    #[test]
    fn pure_addition() {
        let (lines, stats) = line_diff("a\nb", "a\nb\nc\nd", NO_TRUNCATION);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.removed, 0);
        assert_eq!(
            kinds(&lines),
            vec![
                DiffLineKind::Context,
                DiffLineKind::Context,
                DiffLineKind::Added,
                DiffLineKind::Added,
            ]
        );
        assert_eq!(lines[2].text, "c");
        assert_eq!(lines[3].text, "d");
    }

    #[test]
    fn pure_removal() {
        let (lines, stats) = line_diff("a\nb\nc\nd", "a\nd", NO_TRUNCATION);
        assert_eq!(stats.added, 0);
        assert_eq!(stats.removed, 2);
        let removed: Vec<&str> = lines
            .iter()
            .filter(|l| l.kind == DiffLineKind::Removed)
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(removed, vec!["b", "c"]);
    }

    #[test]
    fn middle_modification() {
        let (lines, stats) = line_diff("a\nb\nc", "a\nX\nc", NO_TRUNCATION);
        assert_eq!(stats.added, 1);
        assert_eq!(stats.removed, 1);
        assert_eq!(
            kinds(&lines),
            vec![
                DiffLineKind::Context,
                DiffLineKind::Removed,
                DiffLineKind::Added,
                DiffLineKind::Context,
            ]
        );
        assert_eq!(lines[1].text, "b");
        assert_eq!(lines[2].text, "X");
    }

    #[test]
    fn empty_before_counts_as_zero_lines() {
        // "" 是 0 行——不可被當成 1 行空字串而冒出 Removed ""
        let (lines, stats) = line_diff("", "x\ny", NO_TRUNCATION);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.removed, 0);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|l| l.kind == DiffLineKind::Added));

        // 反向：有內容 → 空字串
        let (lines2, stats2) = line_diff("x\ny", "", NO_TRUNCATION);
        assert_eq!(stats2.added, 0);
        assert_eq!(stats2.removed, 2);
        assert_eq!(lines2.len(), 2);
    }

    #[test]
    fn truncation_keeps_full_stats() {
        let truncate_to = 2;
        let (lines, stats) = line_diff("", "1\n2\n3\n4\n5", truncate_to);
        assert_eq!(lines.len(), truncate_to);
        assert_eq!(stats.added, 5); // stats 以全量計，不受截斷影響
        assert_eq!(stats.removed, 0);
        assert_eq!(lines[0].text, "1");
        assert_eq!(lines[1].text, "2");
    }

    #[test]
    fn crlf_input_is_safe() {
        let (lines, stats) = line_diff("a\r\nb\r\n", "a\r\nc\r\n", NO_TRUNCATION);
        assert_eq!(stats.added, 1);
        assert_eq!(stats.removed, 1);
        // str::lines 剝除 \r——輸出行文字不得殘留 CR
        assert!(lines.iter().all(|l| !l.text.contains('\r')));
    }

    #[test]
    fn dp_fuse_degrades_to_full_remove_add() {
        // 2001 × 2000 = 4_002_000 格 > DIFF_DP_CELL_LIMIT → 觸發保險絲
        let big_n = 2001;
        let big_m = 2000;
        let before: String = (0..big_n).map(|i| format!("line {}\n", i)).collect();
        let after: String = (0..big_m).map(|i| format!("line {}\n", i)).collect();
        let (lines, stats) = line_diff(&before, &after, usize::MAX);
        // 退化模式：即使內容大量重疊也輸出全刪＋全增
        assert_eq!(stats.removed, big_n);
        assert_eq!(stats.added, big_m);
        assert_eq!(lines.len(), big_n + big_m);
        assert_eq!(lines[0].kind, DiffLineKind::Removed);
        assert_eq!(lines[big_n].kind, DiffLineKind::Added);
    }
}
