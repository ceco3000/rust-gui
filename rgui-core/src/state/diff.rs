//! 视图差分：`Patch` / `diff` / `apply_patch`（greenfield §B.1 / §C.1 state/diff.rs）。
//!
//! ## 语义（P1-1 修复后）
//!
//! - `diff(a, b)` 对子节点**递归对比**：公共索引范围内，若子树不等价则生成
//!   `ReplaceChild`（整棵替换为目标子树）——不再只生成 `SetChildCount` 后依赖空节点补齐。
//! - 子节点增删：
//!   - `b` 子节点更多 → `InsertChild { index, subtree }`（携带 `b.children[i]` 完整内容）。
//!   - `a` 子节点更多 → `RemoveChild { index }`（删除多余尾部）。
//! - `apply_patch(view, &[])` 幂等；`diff(a,b)` 后 `apply_patch(b, patches)` **收敛**
//!   （含子节点结构域，`apply(diff(a,b)) == b` 成立）。
//!
//! ## 局限（契约注释，P1-2 明确）
//!
//! 当前 diff 为**位置型（position-based）**，**无 key-based reconcile**：子节点顺序交换
//! 会被当作每处 differs，生成 replace 而非最小化移动。**key-based 排序 reconcile 留 D5。**
//! 当前 diff 支持：根 props 变更 + 子节点**结构增删** + 子树**内容变更**（patch 为替换子树）。

use crate::view::{PropValue, WidgetView};

/// 视图差分补丁（泛型于消息 `M`，因需携带整棵子树 `WidgetView<M>`）。
#[derive(Debug, Clone, PartialEq)]
pub enum Patch<M> {
    /// 替换当前节点 props。
    SetProps(PropValue),
    /// 替换指定索引子节点的 props（仅子节点 props 变、结构不变时）。
    SetChildProps { index: usize, props: PropValue },
    /// 替换指定索引子节点为整棵目标子树（结构/内容差异时）。
    ReplaceChild { index: usize, subtree: WidgetView<M> },
    /// 在 index 处插入整棵目标子树。
    InsertChild { index: usize, subtree: WidgetView<M> },
    /// 删除指定索引的子节点（用于超出的尾部）。
    RemoveChild { index: usize },
    /// 把索引 `from` 的子节点移动到 `to`（D18：key-based reconcile 顺序复用）。
    MoveChild { from: usize, to: usize },
}

/// 递归比较两个视图子树是否等价（props + children 结构）。
fn subtree_eq<M>(a: &WidgetView<M>, b: &WidgetView<M>) -> bool {
    if a.props != b.props {
        return false;
    }
    if a.children.len() != b.children.len() {
        return false;
    }
    a.children
        .iter()
        .zip(b.children.iter())
        .all(|(ca, cb)| subtree_eq(ca, cb))
}

/// 计算 `a` → `b` 的差分补丁序列。
pub fn diff<M: Clone>(a: &WidgetView<M>, b: &WidgetView<M>) -> Vec<Patch<M>> {
    let mut patches = Vec::new();

    // 根 props
    if a.props != b.props {
        patches.push(Patch::SetProps(b.props.clone()));
    }

    // key-based reconcile：children 全有 key 时按 key 匹配复用（move/update 而非索引重建）；否则位置型。
    let a_all_key = a.children.iter().all(|c| c.key.is_some());
    let b_all_key = b.children.is_empty() || b.children.iter().all(|c| c.key.is_some());
    if a_all_key && b_all_key && a.children.len() + b.children.len() > 0 {
        diff_children_keyed(&mut patches, &a.children, &b.children);
    } else {
        // 位置型（现有）：公共索引递归对比 + 尾部增删
        let common = a.children.len().min(b.children.len());
        for i in 0..common {
            if !subtree_eq(&a.children[i], &b.children[i]) {
                patches.push(Patch::ReplaceChild {
                    index: i,
                    subtree: b.children[i].clone(),
                });
            }
        }
        if b.children.len() > a.children.len() {
            for i in a.children.len()..b.children.len() {
                patches.push(Patch::InsertChild {
                    index: i,
                    subtree: b.children[i].clone(),
                });
            }
        } else if a.children.len() > b.children.len() {
            for _ in b.children.len()..a.children.len() {
                patches.push(Patch::RemoveChild {
                    index: b.children.len(),
                });
            }
        }
    }

    patches
}

/// key-based children reconcile（D18）：按 key 匹配复用子节点，minimal move/update 而非索引重建。
/// 通过 (key 匹配的) MoveChild + 内容 ReplaceChild + 新增 InsertChild + 缺失 RemoveChild 收敛。
fn diff_children_keyed<M: Clone>(patches: &mut Vec<Patch<M>>, a: &[WidgetView<M>], b: &[WidgetView<M>]) {
    // 模拟当前 a（与应用 patch 后一致），保证生成的 index 有效
    let mut list: Vec<WidgetView<M>> = a.to_vec();

    // 1. 移除 a 中其 key 不在 b 的子节点（倒序，避免 index 位移）
    for i in (0..list.len()).rev() {
        let k = list[i].key;
        let in_b = b.iter().any(|bc| bc.key == k);
        if !in_b {
            patches.push(Patch::RemoveChild { index: i });
            list.remove(i);
        }
    }

    // 2. 按 b 顺序重建：同 key 匹配（move + content update）、b 独有（insert）
    for j in 0..b.len() {
        let bj = &b[j];
        let src = match bj.key {
            Some(k) => list.iter().position(|c| c.key == Some(k)),
            None => (j < list.len()).then_some(j),
        };
        match src {
            Some(i) if i == j => {
                if !subtree_eq(&list[i], bj) {
                    patches.push(Patch::ReplaceChild { index: j, subtree: bj.clone() });
                }
                list[j] = bj.clone();
            }
            Some(i) => {
                patches.push(Patch::MoveChild { from: i, to: j });
                let c = list.remove(i);
                list.insert(j, c);
                if !subtree_eq(&list[j], bj) {
                    patches.push(Patch::ReplaceChild { index: j, subtree: bj.clone() });
                }
                list[j] = bj.clone();
            }
            None => {
                patches.push(Patch::InsertChild { index: j, subtree: bj.clone() });
                list.insert(j, bj.clone());
            }
        }
    }
}

/// 应用补丁到视图（原地修改）。`&[]` 为空时无副作用。
pub fn apply_patch<M: Clone>(view: &mut WidgetView<M>, patches: &[Patch<M>]) {
    for p in patches {
        match p {
            Patch::SetProps(props) => {
                view.props = props.clone();
            }
            Patch::SetChildProps { index, props } => {
                if let Some(child) = view.children.get_mut(*index) {
                    child.props = props.clone();
                }
            }
            Patch::ReplaceChild { index, subtree } => {
                if let Some(slot) = view.children.get_mut(*index) {
                    *slot = subtree.clone();
                }
            }
            Patch::InsertChild { index, subtree } => {
                if *index <= view.children.len() {
                    view.children.insert(*index, subtree.clone());
                }
            }
            Patch::RemoveChild { index } => {
                if *index < view.children.len() {
                    view.children.remove(*index);
                }
            }
            Patch::MoveChild { from, to } => {
                if *from < view.children.len() {
                    let child = view.children.remove(*from);
                    let to = (*to).min(view.children.len());
                    view.children.insert(to, child);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestView = WidgetView<()>;

    /// 递归比较两个视图树（props + children 结构）。
    fn tree_eq(a: &TestView, b: &TestView) -> bool {
        if a.props != b.props {
            return false;
        }
        if a.children.len() != b.children.len() {
            return false;
        }
        a.children
            .iter()
            .zip(b.children.iter())
            .all(|(ca, cb)| tree_eq(ca, cb))
    }

    fn leaf(props: PropValue) -> TestView {
        let mut v = WidgetView::empty();
        v.props = props;
        v
    }

    fn with_children(props: PropValue, children: Vec<TestView>) -> TestView {
        let mut v = WidgetView::empty();
        v.props = props;
        v.children = children;
        v
    }

    #[test]
    fn diff_identical_empty_is_empty() {
        let a = TestView::empty();
        let b = TestView::empty();
        assert!(diff(&a, &b).is_empty());
        assert!(tree_eq(&a, &b));
    }

    /// roundtrip 收敛：diff(a,b) 后 apply 应重建 b（含子节点结构）。P1-1 修复后 GREEN。
    #[test]
    fn roundtrip_deep_child_content_change_converges() {
        // 复刻 risk-review §1.1：根 props 相同，子数 2→1，index0 深层子 100→999
        let a = with_children(
            PropValue::Int(1),
            vec![
                with_children(PropValue::Int(10), vec![leaf(PropValue::Int(100))]),
                with_children(PropValue::Int(20), vec![leaf(PropValue::Int(200))]),
            ],
        );
        let b = with_children(
            PropValue::Int(1),
            vec![with_children(PropValue::Int(10), vec![leaf(PropValue::Int(999))])],
        );
        let patches = diff(&a, &b);
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "deep child content change should converge");
    }

    /// roundtrip：1→2 增加子节点（含整棵子树）。
    #[test]
    fn roundtrip_add_child_subtree_converges() {
        let a = with_children(PropValue::Unit, vec![leaf(PropValue::Int(1))]);
        let b = with_children(
            PropValue::Unit,
            vec![
                leaf(PropValue::Int(1)),
                with_children(PropValue::Int(2), vec![leaf(PropValue::Int(3))]),
            ],
        );
        let patches = diff(&a, &b);
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "add child subtree should converge");
    }

    /// roundtrip：2→1 删除子节点。
    #[test]
    fn roundtrip_remove_child_converges() {
        let a = with_children(
            PropValue::Unit,
            vec![leaf(PropValue::Int(1)), leaf(PropValue::Int(2))],
        );
        let b = with_children(PropValue::Unit, vec![leaf(PropValue::Int(1))]);
        let patches = diff(&a, &b);
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "remove child should converge");
    }

    /// roundtrip：子节点顺序不变但 props 均变。
    #[test]
    fn roundtrip_sibling_props_change_converges() {
        let a = with_children(
            PropValue::Unit,
            vec![leaf(PropValue::Int(1)), leaf(PropValue::Int(2))],
        );
        let b = with_children(
            PropValue::Unit,
            vec![leaf(PropValue::Int(11)), leaf(PropValue::Int(22))],
        );
        let patches = diff(&a, &b);
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "sibling props change should converge");
    }

    fn kleaf(key: u64, props: PropValue) -> TestView {
        let mut v = WidgetView::empty();
        v.key = Some(key);
        v.props = props;
        v
    }

    /// D18：key-based reorder——children 顺序交换（有 key），应复用（MoveChild）而非索引 replace；收敛。
    #[test]
    fn keyed_reorder_reuses_by_key() {
        let a = with_children(PropValue::Unit, vec![kleaf(1, PropValue::Int(1)), kleaf(2, PropValue::Int(2))]);
        let b = with_children(PropValue::Unit, vec![kleaf(2, PropValue::Int(22)), kleaf(1, PropValue::Int(1))]);
        let patches = diff(&a, &b);
        assert!(
            patches.iter().any(|p| matches!(p, Patch::MoveChild { .. })),
            "key 顺序交换应产生 MoveChild（复用），got {patches:?}"
        );
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "keyed reorder should converge");
        assert_eq!(result.children[0].key, Some(2), "key 2 应复用在前");
        assert_eq!(result.children[0].props, PropValue::Int(22));
    }

    /// D18：keyed 删除中间项——只删 key 缺失者，邻位 key 复用（不 replace）。
    #[test]
    fn keyed_remove_middle_keeps_neighbors() {
        let a = with_children(
            PropValue::Unit,
            vec![kleaf(1, PropValue::Int(1)), kleaf(2, PropValue::Int(2)), kleaf(3, PropValue::Int(3))],
        );
        let b = with_children(PropValue::Unit, vec![kleaf(1, PropValue::Int(1)), kleaf(3, PropValue::Int(3))]);
        let patches = diff(&a, &b);
        assert!(
            patches.iter().any(|p| matches!(p, Patch::RemoveChild { index: 1 })),
            "应删除中间 key2 子节点，而不误伤邻位"
        );
        assert!(
            !patches.iter().any(|p| matches!(p, Patch::ReplaceChild { .. })),
            "邻位 key 应复用（无 replace）"
        );
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "keyed remove should converge");
        assert_eq!(result.children.len(), 2);
    }

    /// D18：keyed 增删 + 重排 + 内容更新 roundtrip 收敛。
    #[test]
    fn keyed_add_remove_and_reorder_roundtrip_converges() {
        let a = with_children(
            PropValue::Unit,
            vec![kleaf(1, PropValue::Int(1)), kleaf(2, PropValue::Int(2)), kleaf(3, PropValue::Int(3))],
        );
        let b = with_children(
            PropValue::Unit,
            vec![kleaf(4, PropValue::Int(40)), kleaf(2, PropValue::Int(22)), kleaf(1, PropValue::Int(1))],
        );
        let patches = diff(&a, &b);
        let mut result = a.clone();
        apply_patch(&mut result, &patches);
        assert!(tree_eq(&result, &b), "keyed add/remove/reorder should converge");
        assert_eq!(result.children[0].key, Some(4), "新增 key4 应在首");
        assert_eq!(result.children[0].props, PropValue::Int(40));
        assert_eq!(result.children[1].key, Some(2));
        assert_eq!(result.children[1].props, PropValue::Int(22));
    }
}
