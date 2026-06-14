//! V6: Taffy 布局 → 渲染坐标转换
//!
//! 验证 Taffy 布局结果可正确映射到渲染坐标系。

use taffy::TaffyTree;
use taffy::prelude::*;

fn main() {
    println!("V6: Taffy 布局 → 渲染坐标转换\n");
    test_flex_row();
    test_flex_column();
    test_grid();
    test_nested();
    println!("\n✅ 全部通过：Taffy 布局坐标可正确转换为渲染坐标");
}

fn test_flex_row() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let leaf = Style {
        size: Size {
            width: length(80.0),
            height: length(36.0),
        },
        ..Default::default()
    };
    let container = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: length(12.0),
        padding: length(16.0),
        ..Default::default()
    };
    let root = tree.new_leaf(container).unwrap();
    let b1 = tree.new_leaf(leaf.clone()).unwrap();
    let b2 = tree.new_leaf(leaf.clone()).unwrap();
    let b3 = tree.new_leaf(leaf.clone()).unwrap();
    tree.add_child(root, b1).unwrap();
    tree.add_child(root, b2).unwrap();
    tree.add_child(root, b3).unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let r = tree.layout(root).unwrap();
    let c1 = tree.layout(b1).unwrap();
    let c3 = tree.layout(b3).unwrap();

    let expected_w = 16.0 + 80.0 + 12.0 + 80.0 + 12.0 + 80.0 + 16.0;
    assert!(
        (r.size.width - expected_w).abs() < 1.0,
        "FlexRow: 容器宽度 {:.0} != 期望 {:.0}",
        r.size.width,
        expected_w
    );
    assert!((c1.location.x - 16.0).abs() < 1.0);
    assert!((c3.location.x - (16.0 + 80.0 + 12.0 + 80.0 + 12.0)).abs() < 1.0);
    println!(
        "✅ FlexRow: 3 按钮 80px gap=12 padding=16 → {:.0}x{:.0}",
        r.size.width, r.size.height
    );
}

fn test_flex_column() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let container = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: length(8.0),
        padding: length(16.0),
        ..Default::default()
    };
    let text = Style {
        size: Size {
            width: length(200.0),
            height: length(28.0),
        },
        ..Default::default()
    };
    let field = Style {
        size: Size {
            width: length(300.0),
            height: length(40.0),
        },
        ..Default::default()
    };
    let btn = Style {
        size: Size {
            width: length(120.0),
            height: length(36.0),
        },
        ..Default::default()
    };

    let root = tree.new_leaf(container).unwrap();
    let n1 = tree.new_leaf(text).unwrap();
    let n2 = tree.new_leaf(field).unwrap();
    let n3 = tree.new_leaf(btn).unwrap();
    tree.add_child(root, n1).unwrap();
    tree.add_child(root, n2).unwrap();
    tree.add_child(root, n3).unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let r = tree.layout(root).unwrap();
    let b_loc = tree.layout(n3).unwrap();
    let expected_y = 16.0 + 28.0 + 8.0 + 40.0 + 8.0;
    assert!(
        (b_loc.location.y - expected_y).abs() < 1.0,
        "FlexColumn: 按钮 y={:.0} != 期望 {:.0}",
        b_loc.location.y,
        expected_y
    );
    println!(
        "✅ FlexColumn: 垂直排列 gap=8 padding=16 → {:.0}x{:.0}",
        r.size.width, r.size.height
    );
}

fn test_grid() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let container = Style {
        display: Display::Grid,
        grid_template_columns: vec![length(200.0), fr(1.0)],
        gap: length(16.0),
        ..Default::default()
    };
    let sidebar = Style {
        size: Size {
            width: length(200.0),
            height: length(400.0),
        },
        ..Default::default()
    };
    let content = Style {
        size: Size {
            width: length(600.0),
            height: length(400.0),
        },
        ..Default::default()
    };

    let root = tree.new_leaf(container).unwrap();
    let side = tree.new_leaf(sidebar).unwrap();
    let cont = tree.new_leaf(content).unwrap();
    tree.add_child(root, side).unwrap();
    tree.add_child(root, cont).unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let cont_loc = tree.layout(cont).unwrap();
    let expected_x = 200.0 + 16.0;
    assert!(
        (cont_loc.location.x - expected_x).abs() < 1.0,
        "Grid: 内容 x={:.0} != 期望 {:.0}",
        cont_loc.location.x,
        expected_x
    );
    println!(
        "✅ Grid: 2 列 200px+1fr gap=16 → 内容 x={:.0}",
        cont_loc.location.x
    );
}

fn test_nested() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let container = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: length(12.0),
        padding: length(16.0),
        ..Default::default()
    };
    let row_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: length(8.0),
        ..Default::default()
    };
    let item = |w, h| Style {
        size: Size {
            width: length(w),
            height: length(h),
        },
        ..Default::default()
    };

    let root = tree.new_leaf(container).unwrap();
    let row = tree.new_leaf(row_style).unwrap();
    tree.add_child(root, row).unwrap();
    let nodes: Vec<_> = vec![
        item(24.0, 24.0),
        item(150.0, 24.0),
        item(50.0, 24.0),
        item(80.0, 36.0),
    ]
    .into_iter()
    .map(|s| {
        let n = tree.new_leaf(s).unwrap();
        tree.add_child(row, n).unwrap();
        n
    })
    .collect();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let btn_x = tree.layout(nodes[3]).unwrap().location.x;
    let expected_x = 0.0 + 24.0 + 8.0 + 150.0 + 8.0 + 50.0 + 8.0;
    assert!(
        (btn_x - expected_x).abs() < 1.0,
        "嵌套: 按钮 x={:.0} != 期望 {:.0}",
        btn_x,
        expected_x
    );
    println!(
        "✅ 嵌套: FlexColumn > FlexRow [Icon, Title, Spacer, Button] → 按钮 x={:.0}",
        btn_x
    );
}
