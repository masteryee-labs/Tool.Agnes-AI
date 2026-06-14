//! ─── DAG 分層並行執行（Phase 2）─────────────────────────────────────────────
//!
//! 兩個確定性原語：
//!  1. `compute_dag_layers` — 依前置依賴將節點分層（Kahn 拓樸），偵測依賴環。
//!     純函式、0 token、可單元測試。
//!  2. `run_layers_parallel` — 同層節點以 tokio `JoinSet` 並行 await，跨層依序；
//!     結果依原始索引還原，與順序執行等價但牆鐘更短。
//!
//! 並行只發生在「同一層內彼此無依賴」的節點，確保結果確定且不破壞依賴順序。

/// 依前置依賴索引將 `n` 個節點分層。`prereqs[i]` 為節點 i 的前置節點索引集合。
/// 回傳每層的節點索引（層內依索引升冪，確定性）。前置索引越界或偵測到環時回傳 `Err`。
pub fn compute_dag_layers(n: usize, prereqs: &[Vec<usize>]) -> Result<Vec<Vec<usize>>, String> {
    if prereqs.len() != n {
        return Err(format!("prereqs 長度 {} 與節點數 {} 不符", prereqs.len(), n));
    }

    let mut indegree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (node, deps) in prereqs.iter().enumerate() {
        for &d in deps {
            if d >= n {
                return Err(format!("節點 {} 的前置索引 {} 越界（n={}）", node, d, n));
            }
            indegree[node] += 1;
            dependents[d].push(node);
        }
    }

    let mut remaining = n;
    let mut layers: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();

    while !current.is_empty() {
        current.sort_unstable();
        remaining -= current.len();
        let mut next: Vec<usize> = Vec::new();
        for &node in &current {
            for &dep in &dependents[node] {
                indegree[dep] -= 1;
                if indegree[dep] == 0 {
                    next.push(dep);
                }
            }
        }
        layers.push(std::mem::take(&mut current));
        current = next;
    }

    if remaining != 0 {
        return Err("偵測到依賴環，無法分層".to_string());
    }
    Ok(layers)
}

/// 同層並行執行器：對每層的索引 `i` 呼叫 `make_fut(i)` 取得 future，同層以 tokio
/// `JoinSet` 並行 await、跨層依序。回傳向量依原始索引 0..n 還原（確定性）。
///
/// 前置條件：`layers` 須覆蓋 0..n 全部索引一次（`compute_dag_layers` 的輸出滿足此性質）。
pub async fn run_layers_parallel<T, F, Fut>(n: usize, layers: &[Vec<usize>], make_fut: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> Fut,
    Fut: std::future::Future<Output = T> + Send + 'static,
{
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for layer in layers {
        let mut set: tokio::task::JoinSet<(usize, T)> = tokio::task::JoinSet::new();
        for &idx in layer {
            let fut = make_fut(idx);
            set.spawn(async move { (idx, fut.await) });
        }
        while let Some(joined) = set.join_next().await {
            if let Ok((idx, val)) = joined {
                if idx < n {
                    out[idx] = Some(val);
                }
            }
        }
    }
    out.into_iter()
        .map(|o| o.expect("layers 必須覆蓋每個索引一次"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_layers_are_sequential() {
        // 0 → 1 → 2：三層各一節點
        let prereqs = vec![vec![], vec![0], vec![1]];
        let layers = compute_dag_layers(3, &prereqs).unwrap();
        assert_eq!(layers, vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn test_independent_nodes_share_one_layer() {
        // 0、1 無依賴；2 依賴 0 與 1 → 同層 [0,1]，再 [2]
        let prereqs = vec![vec![], vec![], vec![0, 1]];
        let layers = compute_dag_layers(3, &prereqs).unwrap();
        assert_eq!(layers, vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn test_cycle_is_rejected() {
        // 0 → 1 → 0 形成環
        let prereqs = vec![vec![1], vec![0]];
        assert!(compute_dag_layers(2, &prereqs).is_err());
    }

    #[test]
    fn test_out_of_range_prereq_is_rejected() {
        let prereqs = vec![vec![5]];
        assert!(compute_dag_layers(1, &prereqs).is_err());
    }

    #[tokio::test]
    async fn test_parallel_preserves_index_order() {
        let prereqs = vec![vec![], vec![], vec![0, 1]];
        let layers = compute_dag_layers(3, &prereqs).unwrap();
        let out = run_layers_parallel(3, &layers, |i| async move { i * 10 }).await;
        assert_eq!(out, vec![0, 10, 20]);
    }

    #[tokio::test]
    async fn test_same_layer_runs_concurrently() {
        use std::time::{Duration, Instant};
        // 三個同層節點各睡 150ms：並行則總時間遠小於 450ms。
        let layers = vec![vec![0usize, 1, 2]];
        let start = Instant::now();
        let out = run_layers_parallel(3, &layers, |i| async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            i
        })
        .await;
        assert_eq!(out, vec![0, 1, 2]);
        assert!(
            start.elapsed() < Duration::from_millis(400),
            "同層三節點應並行，量得 {:?}",
            start.elapsed()
        );
    }
}
