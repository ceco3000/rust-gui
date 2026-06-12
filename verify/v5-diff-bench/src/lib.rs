//! V5: WidgetView diff 性能基准
//!
//! 验证声明式视图 diff-and-patch 在 1000 节点树上的比较性能。

use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct WidgetId(pub u64);

#[derive(Clone, PartialEq, Debug)]
pub enum PropValue {
    Str(Box<str>),
    Bool(bool),
    Num(i64),
    Color(u32),
}

#[derive(Clone, PartialEq, Debug)]
pub struct WidgetView {
    pub widget_type: &'static str,
    pub id: Option<WidgetId>,
    pub props: BTreeMap<&'static str, PropValue>,
    pub children: Vec<WidgetView>,
}

#[derive(Debug)]
pub enum Patch {
    UpdateProps {
        id: WidgetId,
        props: BTreeMap<&'static str, PropValue>,
    },
    Replace {
        id: WidgetId,
        new: WidgetView,
    },
    Insert {
        parent: WidgetId,
        index: usize,
        child: WidgetView,
    },
    Remove {
        parent: WidgetId,
        index: usize,
    },
}

/// 深度优先 diff，返回 patch 列表
pub fn diff(old: &WidgetView, new: &WidgetView) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_recursive(old, new, &mut patches);
    patches
}

fn diff_recursive(old: &WidgetView, new: &WidgetView, patches: &mut Vec<Patch>) {
    if old.widget_type != new.widget_type {
        if let Some(id) = old.id.clone() {
            patches.push(Patch::Replace { id, new: new.clone() });
        }
        return;
    }

    if old.props != new.props {
        if let Some(id) = old.id.clone() {
            let changed: BTreeMap<_, _> = new
                .props
                .iter()
                .filter(|(k, v)| old.props.get(*k) != Some(v))
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            if !changed.is_empty() {
                patches.push(Patch::UpdateProps { id, props: changed });
            }
        }
    }

    let max = old.children.len().max(new.children.len());
    for i in 0..max {
        match (old.children.get(i), new.children.get(i)) {
            (Some(a), Some(b)) => diff_recursive(a, b, patches),
            (None, Some(c)) => {
                if let Some(pid) = old.id.clone() {
                    patches.push(Patch::Insert { parent: pid, index: i, child: c.clone() });
                }
            }
            (Some(_), None) => {
                if let Some(pid) = old.id.clone() {
                    patches.push(Patch::Remove { parent: pid, index: i });
                }
            }
            (None, None) => {}
        }
    }
}

/// 生成平衡 widget 树
pub fn build_tree(depth: usize, fanout: usize) -> WidgetView {
    let mut next_id = 0u64;
    build_node(depth, fanout, &mut next_id)
}

fn build_node(depth: usize, fanout: usize, next_id: &mut u64) -> WidgetView {
    let id = WidgetId(*next_id);
    *next_id += 1;
    let mut props = BTreeMap::new();
    props.insert("visible", PropValue::Bool(true));
    props.insert("opacity", PropValue::Num(1));

    let children = if depth > 0 {
        (0..fanout).map(|_| build_node(depth - 1, fanout, next_id)).collect()
    } else {
        vec![]
    };

    WidgetView { widget_type: "Container", id: Some(id), props, children }
}

/// 修改约 n 个节点的属性
pub fn mutate_tree(view: &WidgetView, n: usize) -> WidgetView {
    let mut out = view.clone();
    let mut count = 0;
    mutate_rec(&mut out, n, &mut count, 0);
    out
}

fn mutate_rec(view: &mut WidgetView, target: usize, count: &mut usize, depth: usize) {
    if *count >= target {
        return;
    }
    if depth % 3 == 0 {
        view.props.insert("modified", PropValue::Num(*count as i64));
        *count += 1;
    }
    for child in &mut view.children {
        if *count >= target {
            return;
        }
        mutate_rec(child, target, count, depth + 1);
    }
}
