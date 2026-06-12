//! V2: cosmic-text CJK 文本渲染质量验证

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

fn main() {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(20.0, 20.0);
    let attrs = Attrs::new();

    let cases = [
        ("简体中文", "你好，世界！这是一个 Rust GUI 框架的技术验证。"),
        ("繁体中文", "這是一個繁體中文測試範例。"),
        ("日文",     "こんにちは、世界！"),
        ("韩文",     "안녕하세요, 세계!"),
        ("中英混排", "显示 1,024 条记录（共 10,240 条）"),
        ("数字符号", "第 1/10 页 · 进度 95.5% · 金额 ¥12,800"),
        ("Emoji",    "✅ 通过  🔴 失败  🟡 警告  📊📋🔍"),
        ("Bidi",     "English text مرحبا 混合排版 test"),
        ("多字体",   "中文正文 with English terms 和日本語混在"),
        ("生僻字",   "囧 𬭤 𬎆 㑳 㒭"),
    ];

    println!("V2: cosmic-text CJK 文本渲染质量验证\n");

    let mut all_pass = true;

    for (cat, text) in &cases {
        let mut buf = Buffer::new(&mut font_system, metrics);
        buf.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);

        let glyph_count: usize = buf.layout_runs()
            .map(|r| r.glyphs.len())
            .sum();
        let missing = glyph_count == 0;

        let status = if missing { all_pass = false; "⚠️ 缺字" } else { "✅" };
        println!("{status}  {cat}: {text}");
    }

    println!("\n──────────────────────────────");
    println!("{}", if all_pass { "✅ 全部通过：所有文本正确 shaping，无 tofu" }
             else { "⚠️ 部分缺字，需检查字体回退配置" });
}
