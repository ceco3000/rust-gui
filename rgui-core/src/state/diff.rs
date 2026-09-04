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

    // 公共索引范围内的子节点：递归对比，结构/内容不等则整棵替换
    let common = a.children.len().min(b.children.len());
    for i in 0..common {
        if !subtree_eq(&a.children[i], &b.children[i]) {
            patches.push(Patch::ReplaceChild {
                index: i,
                subtree: b.children[i].clone(),
            });
        }
    }

    // b 子节点更多 → 插入整棵子树（index 从 a_len 递增至 b_len-1）
    if b.children.len() > a.children.len() {
        for i in a.children.len()..b.children.len() {
            patches.push(Patch::InsertChild {
                index: i,
                subtree: b.children[i].clone(),
            });
        }
    }
    // a 子节点更多 → 删除多余尾部（持续删除 index=b_len 位置直至长度为 b_len）
    else if a.children.len() > b.children.len() {
        for _ in b.children.len()..a.children.len() {
            patches.push(Patch::RemoveChild {
                index: b.children.len(),
            });
        }
    }

    patches
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
}
