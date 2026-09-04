//! D4 验收测试 · 状态 diff / snapshot（Patch 应用正确性）
//!
//! 注入：cp tools/qa/d4_tests/d4_acceptance_state.rs rgui-core/tests/
//! 运行：cargo test -p rgui-core --test d4_acceptance_state
//!
//! 契约基准：greenfield §B.1 + contract-design §2 + dev 现有 TDD 测试（rgui-core/tests/state_diff.rs）。
//! ⚠️ 当前（D4 进行中）dev 尚未实现 state::diff/apply_patch/Patch::SetProps → 本文件不注入，
//! 待 dev 实现该 API 后再注入（注入时若缺失即回归，退回 dev）。
//!
//! 对齐 dev 实际契约：`Patch::SetProps(PropValue)`、`diff(&a,&b)->Vec<Patch>`、
//! `apply_patch(&mut view, &patch)`（单补丁）。与 dev tests/state_diff.rs 同款语义，此处做独立补强。

use rgui_core::state::{apply_patch, diff, Patch, Snapshot, Snapshotter};
use rgui_core::view::{PropValue, WidgetView};

type M = ();

fn view_with_props<M>(props: PropValue, children: Vec<WidgetView<M>>) -> WidgetView<M> {
    let mut v = WidgetView::empty();
    v.props = props;
    v.children = children;
    v
}

// ============ P: diff ============

#[test]
fn p1_noop_diff_is_empty() {
    let a = view_with_props(PropValue::Int(1), vec![]);
    let b = view_with_props(PropValue::Int(1), vec![]);
    assert!(diff(&a, &b).is_empty());
}

#[test]
fn p2_single_prop_change_generates_patch() {
    let a = view_with_props(PropValue::Int(1), vec![]);
    let b = view_with_props(PropValue::Int(2), vec![]);
    assert!(!diff(&a, &b).is_empty());
}

#[test]
fn p4_child_count_change_generates_patch() {
    let a = view_with_props(PropValue::Unit, vec![view_with_props::<M>(PropValue::Int(1), vec![])]);
    let b = view_with_props(PropValue::Unit, vec![
        view_with_props::<M>(PropValue::Int(1), vec![]),
        view_with_props::<M>(PropValue::Int(2), vec![]),
    ]);
    assert!(!diff(&a, &b).is_empty());
}

#[test]
fn p6_same_value_same_type_empty() {
    let a = view_with_props(PropValue::Str("x".into()), vec![]);
    let b = view_with_props(PropValue::Str("x".into()), vec![]);
    assert!(diff(&a, &b).is_empty());
}

// ============ A: apply_patch ============

#[test]
fn a2_apply_setprops_reaches_target() {
    let mut v = view_with_props::<M>(PropValue::Int(1), vec![]);
    apply_patch(&mut v, &Patch::SetProps(PropValue::Int(2)));
    assert_eq!(v.props, PropValue::Int(2));
}

#[test]
fn a3_diff_apply_roundtrip_converges() {
    let a = view_with_props(PropValue::Int(1), vec![]);
    let b = view_with_props(PropValue::Int(9), vec![]);
    let target = b.clone();
    let mut result = a.clone();
    for p in diff(&a, &b) {
        apply_patch(&mut result, &p);
    }
    assert_eq!(result.props, target.props);
}

// ============ SN: snapshot ============

#[test]
fn sn1_snapshot_constructible_no_panic() {
    let _ = Snapshot::default();
    let _ = Snapshotter::default();
}

#[test]
fn sn2_snapshot_schema_reserved() {
    // schema 稳定性（D6/D7 序列化后启用完整断言）：当前收敛到「可构造、无 panic」
    let _snap = Snapshot::default();
}
