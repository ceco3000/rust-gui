//! 嵌入字体管理与加载。
//!
//! 框架在编译时将默认字体嵌入二进制，确保三平台默认字体一致。
//!
//! # 设计依据
//!
//! - D0 §7 不变量 7: 框架内置字体 Noto Sans CJK + Inter + Noto Color Emoji
//! - D3 §8: 利用 cosmic-text 的 `fontdb::Source::Binary` 嵌入二进制
//!
//! # 字体文件
//!
//! 字体 `.ttf` 文件位于 workspace 根目录 `assets/fonts/`，通过
//! [`include_bytes!`] 在编译时嵌入。若字体文件缺失，编译时会发出
//! 友好错误提示，并引导用户运行构建脚本下载。
//!
//! 默认嵌入字体：
//!
//! | 字体 | 用途 | 文件 |
//! |------|------|------|
//! | Inter | 拉丁/数字 UI 字体 | `Inter-Regular.ttf`, `Inter-Bold.ttf` |
//! | Noto Sans CJK SC | 中日韩字符 | `NotoSansCJKsc-Regular.otf`（可选） |
//! | Noto Color Emoji | Emoji | `NotoColorEmoji.ttf`（可选） |
//!
//! # Feature flags
//!
//! - `vello-backend`（默认启用）: 开启字体嵌入能力
//! - `embed-all-fonts`: 嵌入 CJK 和 Emoji 字体（体积较大，**预留 feature，字体文件尚未入库**）

use std::sync::Arc;

use fontdb::{Database, Source};

/// 嵌入字体元数据。
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// 字体的原始字节数据。
    pub data: &'static [u8],
    /// 字体族名称。
    pub family: &'static str,
    /// 字重（OpenType weight 值，400=Regular, 700=Bold）。
    pub weight: u16,
    /// 字体样式（正常/斜体）。
    pub style: fontdb::Style,
}

/// 内置嵌入字体列表。
///
/// 当前包含 Inter Regular + Inter Bold 两种字重。
/// 条目按 `(family, weight)` 排序，先普通后粗体。
pub static EMBEDDED_FONTS: &[EmbeddedFont] = &[
    // -- Inter Latin UI 字体 --
    EmbeddedFont {
        data: include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
        family: "Inter",
        weight: 400,
        style: fontdb::Style::Normal,
    },
    EmbeddedFont {
        data: include_bytes!("../../assets/fonts/Inter-Bold.ttf"),
        family: "Inter",
        weight: 700,
        style: fontdb::Style::Normal,
    },
    // -- Noto Sans CJK SC + Noto Color Emoji（未来扩展）--
    // 当字体文件准备就绪后，在此处添加条目并使用 `#[cfg(feature = "embed-all-fonts")]` 控制编译。
    // CJK 和 Emoji 字体体积较大（各约 10-15MB），通过 feature flag 控制以减小默认二进制体积。
];

/// 将所有嵌入字体注册到 `fontdb::Database`。
///
/// # 参数
///
/// * `db` — 可变引用，指向目标 `fontdb::Database`
pub fn register_embedded_fonts(db: &mut Database) {
    for font in EMBEDDED_FONTS {
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(font.data.to_vec());
        db.load_font_source(Source::Binary(data));
    }
}

/// 创建一个预置了所有嵌入字体的 `fontdb::Database`。
///
/// 返回的 Database 已加载 `EMBEDDED_FONTS` 中所有字体，
/// 可直接用于构造 `cosmic-text` 的 `FontSystem`。
#[must_use]
pub fn create_default_database() -> Database {
    let mut db = Database::new();
    register_embedded_fonts(&mut db);
    db
}

/// 查询嵌入字体的概要信息。
///
/// 返回每个嵌入字体的 `(family, weight, style)` 元组列表。
/// 用于调试和日志输出。
#[must_use]
pub fn list_embedded_fonts() -> Vec<(&'static str, u16, fontdb::Style)> {
    EMBEDDED_FONTS
        .iter()
        .map(|f| (f.family, f.weight, f.style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_are_not_empty() {
        assert!(
            !EMBEDDED_FONTS.is_empty(),
            "should have at least one embedded font"
        );
    }

    #[test]
    fn inter_regular_is_present() {
        let has_inter_regular = EMBEDDED_FONTS
            .iter()
            .any(|f| f.family == "Inter" && f.weight == 400 && f.style == fontdb::Style::Normal);
        assert!(has_inter_regular, "Inter Regular should be embedded");
    }

    #[test]
    fn inter_bold_is_present() {
        let has_inter_bold = EMBEDDED_FONTS
            .iter()
            .any(|f| f.family == "Inter" && f.weight == 700 && f.style == fontdb::Style::Normal);
        assert!(has_inter_bold, "Inter Bold should be embedded");
    }

    #[test]
    fn embedded_font_data_is_valid_ttf() {
        // 验证嵌入字体数据至少有最小的有效 TTF 头部
        // TrueType 字体以 0x00010000 或 0x74727565 ('true') 开头
        for font in EMBEDDED_FONTS {
            assert!(
                font.data.len() > 12,
                "font {} data is too short: {} bytes",
                font.family,
                font.data.len()
            );

            let head: [u8; 4] = font.data[0..4].try_into().unwrap();
            let is_valid = head == [0x00, 0x01, 0x00, 0x00] // TrueType (v1.0)
                || head == [0x01, 0x00, 0x00, 0x00] // TrueType (v1.0, OTTO)
                || head == [0x74, 0x72, 0x75, 0x65] // TrueType ('true')
                || head == [0x4f, 0x54, 0x54, 0x4f]; // OpenType ('OTTO')
            assert!(
                is_valid,
                "font {} has invalid TTF/OTF header: {head:02x?}",
                font.family
            );
        }
    }

    #[test]
    fn register_embedded_fonts_populates_database() {
        let mut db = Database::new();
        let initial_count = db.faces().count();
        register_embedded_fonts(&mut db);
        let after_count = db.faces().count();
        assert!(
            after_count > initial_count,
            "should have added at least one font face to the database"
        );
    }

    #[test]
    fn create_default_database_has_fonts() {
        let db = create_default_database();
        let face_count = db.faces().count();
        assert!(
            face_count > 0,
            "default database should have fonts, got {face_count}"
        );
    }

    #[test]
    fn list_embedded_fonts_returns_all() {
        let listed = list_embedded_fonts();
        assert_eq!(
            listed.len(),
            EMBEDDED_FONTS.len(),
            "list should have the same count as EMBEDDED_FONTS"
        );

        // 验证每一项都出现在 EMBEDDED_FONTS 中
        for (family, weight, style) in &listed {
            let exists = EMBEDDED_FONTS
                .iter()
                .any(|f| f.family == *family && f.weight == *weight && f.style == *style);
            assert!(
                exists,
                "listed font {family} weight={weight} not in EMBEDDED_FONTS"
            );
        }
    }

    #[test]
    fn embedded_font_metadata_matches_data() {
        // 验证 Inter Regular 的数据长度在合理范围内
        let inter_reg = EMBEDDED_FONTS
            .iter()
            .find(|f| f.family == "Inter" && f.weight == 400)
            .expect("Inter Regular should exist");
        assert!(
            inter_reg.data.len() > 100_000,
            "Inter Regular should be at least 100KB, got {} bytes",
            inter_reg.data.len()
        );
        assert!(
            inter_reg.data.len() < 2_000_000,
            "Inter Regular should be under 2MB, got {} bytes",
            inter_reg.data.len()
        );
    }

    #[test]
    fn fontdb_can_query_loaded_fonts() {
        let db = create_default_database();
        // fontdb::Database::faces() 返回 impl Iterator<Item = &FaceInfo>
        let faces: Vec<_> = db.faces().collect();
        // FaceInfo.families 是 Vec<(String, Language)> — (font_family, language)
        let inter_faces: Vec<_> = faces
            .iter()
            .filter(|face| {
                face.families
                    .iter()
                    .any(|(name, _lang)| name.contains("Inter"))
            })
            .collect();
        assert!(
            inter_faces.len() >= 2,
            "should have at least 2 Inter faces (Regular + Bold), got {}",
            inter_faces.len()
        );
    }
}
