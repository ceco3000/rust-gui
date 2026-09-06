# 界面风格审查：rgui window_demo（Accordion 组件 + 系列演示）

审查对象：rgui 自绘渲染引擎（vello + cosmic-text）主示例 `window_demo` 及系列验证 demo
审查方式：视觉逐像素分析 + 像素级取色采样（PIL）
素材目录：`tools/qa/d7_screenshots/`（共 13 张，全部为 Retina 2x，逻辑尺寸 = px ÷ 2）

> 说明：rgui 为自定义渲染引擎，窗口标题栏 / 交通灯 / 按钮 / 列表 / accordion 全部为**自绘控件**（非 SwiftUI/AppKit 原生控件）。本报告只对照 Apple macOS HIG 审查**视觉风格**，评价功能正确性 / 架构 / 业务逻辑均不在范围内。

---

## 总体评价

界面具备了 macOS 桌面的**基本骨架**：左上角自绘红黄绿交通灯、连续曲率大圆角窗口、外部柔和阴影、浅色标题文字对比度达标、焦点/选中/悬停不同态均有区分、列表项与 badge 状态提示清晰可读，这些是值得肯定的正面项。

但作为 macOS 桌面应用，整体观感**显著偏离** Apple 风格，主要踩在三个 HIG 硬标准上：**(1) 展开内容使用超大字号占位文本「details / Badge: 0」且溢出窗口右边界被裁切，文案层级失控；(2) 顶部 header 为整条饱和浅蓝「#98B8E8」色带，其上白字对比度实测仅 2.03:1（badge 白字 1.61:1），远低于 4.5:1 无障碍下限；(3) 窗口内容区为纯黑「#000000」平铺，无材质、无 elevation 分层、无阴影，接近终端/游戏窗口而非 macOS 深色应用**。

综合评级：**需整改**（存在 P0 级硬伤，修完 P0 后方可谈打磨）。

---

## P0 问题（必须修）

### P0-1 窗口内容区纯黑「#000000」平铺，无材质/无分层/无 elevation，观感非 macOS 深色应用
- **位置**：全部截图内容区（d10~d21）；取色采样 `content dominant = #000000`（d21 内容区 38384 像素全部黑色）
- **现状**：窗口内容区整体为纯黑平铺，无任何灰度分层、无材质(vibrancy)、无 elevation 阴影；标题栏「#232323/#302820」与内容区之间也无视觉分隔，黑成一片
- **HIG 依据**：
  > **HIG — Dark Mode**: "the system uses a dark color palette for all screens, views, menus, and controls，and may also use greater perceptual contrast to make foreground content stand out against the darker backgrounds."
  > **HIG — Materials**: "Materials help visually separate foreground elements, such as text and controls, from background elements... establishing visual hierarchy to help people more easily retain a sense of place."
  > **Design Tokens（深色模式数值）**: 窗口背景 `#282828`、侧边栏 `#1E1E1E`——深色绝非纯黑 #000000；需靠「底色分层 + 材质 + 微阴影」建立深度
- **整改建议**：内容区底色改用 `#282828`（或至少 `#1E1E1E` 系列），工具栏/标题栏用半透明材质或略亮的 `#2E2E2E`，内容区与标题栏之间留出 1px hairline 分隔（`rgba(255,255,255,0.1)`）；控件/面板用 `#3A3A3A` 抬升一级，形成浅-中-深的 elevation 阶梯

### P0-2 顶部 header 整条饱和浅蓝色带 + 白字对比度严重不足（2.03:1 / 1.61:1）
- **位置**：d10_final_collapsed / d10_final_expanded / d21_t1；采样 `header(accordion) dominant = #98B8E8`（18843 像素），`badge region dominant = #B0D0E8`；白字 `#FFFFFF`（d21 header brightest = #FFFFFF，位于 (272,156)）
- **现状**：header 是一整条横贯窗口全宽的饱和浅蓝「#98B8E8」色带（d21 row_y182 从 x112 到 x1111，即 0 边距贴满整行）；带内为白色文字「Accordion [-ɔ]」/「badge: 0」。实测 WCAG 对比度：白字/#98B8E8 = **2.03:1**；白字/#B0D0E8 = **1.61:1**
- **HIG 依据**：
  > **HIG — Accessibility**: "Text size Up to 17pts — Minimum contrast ratio **4.5:1**"; "A button with insufficient color contrast... strive to meet color contrast minimum standards"
  > **HIG — Color**: "Avoid using colors that make it hard to perceive content in your app... insufficient contrast can cause icons and text to blend with the background"
  > **HIG — Dark Mode 文字要求**: custom foreground/background 建议对比度 **7:1**（小字）
- **整改建议**：（a）白色文字换成深色（如 `labelPrimary #000`，此时对比度 ~10:1），或（b）色带压深到接近 systemBlue 深色变体（`#0A6AD4`/`#007AFF`）再配白字，保证对比度 ≥ 4.5:1；（c）整条饱和蓝色带本身不符合 macOS 工具栏惯例——建议改为透明/半透明材质 header + 1px hairline separator，仅对「当前选中项」用强调色填充

### P0-3 展开内容为超大字号占位文本，且溢出窗口右边界被裁切
- **位置**：d10_final_expanded（「Badge: 0」+「v」）、d21_t1（「v details」超大白字，x 从 ~700 延伸到窗口右缘 x1111 被裁切；采样 `details area dominant = #000000 / #F8F8F8 大块白`）
- **现状**：展开态渲染出一行**超大字号白色文本**「details」「Badge: 0」，字号目测为 Title/Large Title 级（约 26pt+），盖过 header 层级并横向溢出窗口右边界被裁切；「v」字符离群悬空、baseline 错位
- **HIG 依据**：
  > **HIG — Typography**: "Adjust font weight, size, and color as needed to emphasize important information and help people visualize hierarchy."（此处相反：展开内容用最大字号反而破坏了层级）
  > **HIG — Typography**: "Minimize the number of typefaces... avoid light font weights" + 层级主张 "Use weight for hierarchy, not just size"
  > **HIG — Layout**: "Extend content to fill the screen or window... ensure that scrollable layouts continue all the way to the bottom and the sides"（但**不得裁切/溢出**）; "Place items to convey their relative importance"（展开内容是第一级阅读对象，应比 header 次要）
- **整改建议**：展开内容用 `Body 13pt Regular`（或 `Callout 12pt`）作为正文层级，明显小于 header `Headline 13pt Bold`；内容区加上 20pt 内容边距防止文字贴边溢出；「details」占位文字应改为有意义的正文；修复「v」字符与文字块的基线对齐（或改用规范 chevron 箭头）

---

## P1 问题（明显偏差，不破坏整体但不协调）

### P1-1 折叠指示符用文字「[+-]」，非 macOS disclosure chevron
- **位置**：d10「Settings [+-]」/ d21「Accordion [-ɔ]」/ d11「Accordion [+]」
- **现状**：Accordion 折叠/展开指示器用文本「[+]」「[-]」甚至「[-ɔ]」自拼，非 macOS 惯用的 disclosure 三角（chevron）或旋转箭头
- **HIG 依据**：
  > **HIG — Focus And Selection / 通用控制惯例**: macOS 使用 disclosure chevron（macOS Big Sur+ 用 chevron.right / chevron.down）折叠区级内容；文本「[+]」「[-]」缺乏平台一致性
- **整改建议**：改用标准 chevron（实心三角向右=收起，向下=展开），图标与文字 baseline 对齐、间距 4-6pt；选 SF Symbols 风格线性描边

### P1-2 同窗口内 Accordion 头部文字颜色不一致（d10 深色 vs d21 白色）
- **位置**：d10_final_collapsed（header band 内采样到黑色文字像素 (452,148)=#000000，glyph 中心采样为底色 #9FBEEC 说明文字为深色） vs d21_t1（header brightest = 纯白 #FFFFFF）
- **现状**：同一个「Accordion 头部」在 d10 用深色文字、在 d21 用白色文字——样式驱动不一致，且 d10 的浅蓝底 + 深字与 d21 的浅蓝底 + 白字在语义上互相矛盾
- **HIG 依据**：
  > **HIG — Color**: "Avoid using the same color to mean different things. Use color consistently throughout your interface"
- **整改建议**：统一全应用 Accordion 头部文字颜色语义（推荐：浅色底配深色文字 labelPrimary，深色底配白字），并保证所选组合对比度 ≥ 4.5:1

### P1-3 标题栏为纯色无 vibrancy，与内容区几乎无分层
- **位置**：全部截图顶部；`titlebar = #232323/#302820`（采样 x3600 像素）
- **现状**：标题栏是一整块深灰纯色，无透明/材质；标题「rgui hit-test demo」等处字体颜色一致、无标题栏专属描边或分隔
- **HIG 依据**：
  > **HIG — Materials / Design Tokens（窗口与材质）**: 标题栏为一体化 toolbar，交通灯左上角间距 8-12pt；macOS 标题栏/侧边栏使用 semi-translucent + blur 毛玻璃材质，内容区不透明白
- **整改建议**：标题栏改用半透明材质（material），或至少加 1px hairline 分隔以与内容区区分；交通灯与窗口左缘保持 8-12pt

### P1-4 主题样式使用大面积粉色「#F5D2D2」填充头部，非系统语义色 + 大面积彩色
- **位置**：d19_style_theme.png；`header_pink dominant = #F5D2D2`（粉），标题栏 `#232323`
- **现状**：主题切换 demo 给 Accordion 头部整块填充粉色「#F5D2D2」，大面积彩色背景与深色窗口形成跳脱对比，且该粉并非 Apple 语义色
- **HIG 依据**：
  > **HIG — Color**: "Ensure that all your app's colors work well in light, dark, and increased contrast contexts"; "Test your app's color scheme under a variety of lighting conditions"
  > **Design Tokens（规则）**: "强调色只用于可交互或需注意的元素；大面积彩色背景极不符合 macOS 观感"; "避免硬编码纯黑纯白以外的任意灰/色，强调色用于可交互元素"
- **整改建议**：主题色改用系统语义色（systemBlue/systemTeal 等）做「当前为主色」区分，填充仅用于「当前选中 tab/主题」而非整条 header；其余 header 用中性灰/材质

### P1-5 模态窗口（d20）无明确 dismiss 按钮、无父界面遮罩、标题非任务型
- **位置**：d20_modal.png；标题「rgui d20 modal」，正文为两行选项「(A) 后台一」「(B) 后台二」，无可见按钮
- **现状**：模态以独立窗口呈现，无「取消/确定」按钮，无父界面变暗遮罩；标题为示例名而非描述任务的名称
- **HIG 依据**：
  > **HIG — Modality**: "Always give people an obvious way to dismiss a modal view... in desktop apps, people expect to find a button in the main content view."
  > **HIG — Modality**: "Make it easy to identify a modal view's task. When you provide a title that names the modal view's task"
- **整改建议**：为模态提供明确的「取消 / 确认」按钮（如右下角 push button）；父窗口加遮罩变暗；标题改为任务型名称

### P1-6 焦点态用「左竖条 + 实心三角」，非 macOS 整行高亮/焦点环
- **位置**：d13_focus_highlight.png（Accordion 头部左缘竖条 + 实心蓝色三角）
- **现状**：焦点指示为左侧细竖条 + 实心三角，非 macOS 的「列表整行高亮（accent 色填充）」或「焦点环」
- **HIG 依据**：
  > **HIG — Focus And Selection**: "In general, use a focus ring for a text or search field, but use a highlight in a list or collection. Although you can use a focus ring... it's usually easier for people to view lists and collections when an entire row is highlighted."
  > **HIG — Focus And Selection**: "Indicate focus using visual appearances that are consistent with the platform... the system draws focused list items using white text and a background highlight that matches the app's accent color"
- **整改建议**：列表/accordion 项的焦点/选中态改用「整行圆角高亮 + accent 色」（浅色模式浅蓝灰，深色模式 accent 色），去掉左竖条+三角的生僻组合

### P1-7 内容区左边距为 0，破坏窗口内容边距惯例
- **位置**：d21 header band row_y182 从 x112 到 x1111（贴合窗口左右缘 x112/x1150）
- **现状**：内容/header 直接贴齐窗口内缘（0 边距），无 20pt 内容边距；列表项「item 1: 11」等也几乎贴左
- **HIG 依据**：
  > **Design Tokens（间距规则）**: "窗口内容四周边距 20pt 是 macOS 惯例，< 12pt 即明显拥挤"; "所有间距取 8 的倍数"
- **整改建议**：内容区四周统一 20pt 边距（8pt 网格），header/内容/列表均内缩对齐

---

## P2 打磨项（细节优化）

### P2-1 交通灯直径与间距略偏大，绿色偏亮
- **位置**：d21_t1 交通灯采样 `red=#FE5C61, yellow=#FBC734, green=#3FC662`；bbox 红(130-157) 直径≈27px=**13.5pt**、圆心间距≈46px=**23pt**（红色中心 x≈143、绿色 x≈235）
- **现状**：自绘交通灯直径约 13.5pt、圆心间距约 23pt，微大于 macOS 标准（直径 ~12pt / 间距 ~20pt）；绿色 #3FC662 比 macOS 标准 #28C840 更亮更艳
- **HIG 依据**：> **Design Tokens（窗口与材质）**: 交通灯按钮为平台的窗口控件惯例，需与系统度量一致
- **整改建议**：直径收紧到 ~12pt、圆心间距收紧到 ~20pt；三色改用 macOS 标准 `#FF5F57 / #FEBC2E / #28C840`

### P2-2 窗口边缘为细描边，缺乏柔和加深投影
- **位置**：全部窗口外缘——可见一圈细浅色描边（约 hairline），而非 macOS 特有的柔和深色大投影
- **HIG 依据**：> **Design Tokens（窗口与阴影）**: "窗口阴影柔和大（macOS 特征）；卡片阴影浅（0 0 8-16pt rgba(0,0,0,0.1)）"
- **整改建议**：用大半径+低透明度柔影（偏移 ~0、blur ~24pt、rgb(0,0,0,0.3~0.5)）替代细描边，让窗口浮起

### P2-3 文字体系未采用 SF Pro 字号阶梯（几乎全为大字号）
- **位置**：全界面正文与占位字（如「details」「Badge: 0」等大字号）
- **现状**：未见 Body 13pt / Callout 12pt 的小号正文；大量文字用大字号，无 8-16pt 的细字号梯队
- **HIG 依据**：> **HIG — Typography**: 桌面默认 13pt / 最小 10pt; "Use weight for hierarchy, not just size"; **Design Tokens（字体层级）**: Body 13pt Regular / Callout 12pt / Footnote 10pt
- **整改建议**：正文统一 Body 13pt Regular，辅助说明 Callout 12pt，脚注/标签 Footnote 10pt；用字重（Regular → Semibold → Bold）建立层级，仅在标题处放大

### P2-4 展开/收起动效未见缓动与「减弱动态效果」适配
- **位置**：d10_final_collapsed（收起）/ d10_final_expanded（展开）对比
- **现状**：展开为瞬间切换，未观察到 0.2-0.3s ease-in-out 的平滑过渡，也未提及适配「Reduce Motion」
- **HIG 依据**：> **Design Tokens（动效）**: "默认时长 0.2-0.3s，缓动曲线 ease-in-out；窗口出现/面板展开用缩放+淡入；尊重『减弱动态效果』系统设置"
- **整改建议**：对展开/收起加 0.2-0.3s ease-in-out 过渡；监听系统 Reduce Motion。

---

## 正面记录（做得符合 MAC 风格的地方）

1. **保留 macOS 交通灯（红/黄/绿）**：自绘窗口左上角三色圆点位置、配色方向正确（红 #FE5C61 / 黄 #FBC734 / 绿 #3FC662），符合 macOS 窗口控件约定
2. **连续曲率大圆角窗口**：窗口采用约 12pt 连续曲率圆角，贴合 Big Sur+ 窗口形态
3. **标题文字对比度达标**：标题「rgui hit-test demo」浅色字（约 #B8B0A8）/ 深色标题栏（#302820）实测 **6.77:1**，满足 4.5:1 无障碍要求（这是全界面唯一明显达标的前景文字）
4. **焦点/选中/悬停态有区分**：d13 焦点高亮、d14 焦点背景高亮、d19 主题切换等 demo 对同一组件表达了多种状态，交互反馈意识正确
5. **列表项与状态提示可读**：d18 动态列表「item 1: 11 / item 2: 22 / item 3: 33」以白字配深底，对比充分；badge 计数「badge: 0」提供了状态提示
6. **缩放/换行/描边等行为可验证**：d15 缩放比例、d17 布局换行、d16 描边边框、d20 模态等 demo，覆盖了多状态/多尺寸的验证面

---

## 整改优先级建议

**顺序：P0 → P1 → P2**。

1. **先修 P0（必须）**：P0-1（纯黑无材质，window 观感未达标）、P0-2（header 白字/浅蓝对比度 2.03:1 失效 + 整条饱和色带）、P0-3（展开内容超大字号溢出裁切）。这三条触碰 HIG 的**无障碍对比度下限**与**窗口材质分层**两个硬标准，是「像不像 macOS」的决定性因素，无论 demo 属性都必须改。
2. **再处理 P1（明显不协调）**：P1-1 折叠 chevron、P1-2 文字颜色不一致、P1-4 大面积粉色、P1-5 模态无 dismiss、P1-6 焦点态形态——这些把「控件语义 + 界面一致性」拉回 macOS 惯例，一次改到位可显著提升统一感。
3. **最后打磨 P2**：交通灯度量、柔影、字号阶梯、动效——属于锦上添花的细节，不影响主观判断，可并入后续迭代。

> 优先级排序理由：P0 直接影响「是否被识别为 macOS 应用」与「可读性/可用性（无障碍）」，是风格上线的红线；P1 影响整体协调与平台语义，成本适中收益大；P2 为专业度打磨，可延后。建议首轮迭代锁定 P0 三项 + P1-1/P1-2/P1-6，其余随功能推进。

---

*报告生成依据：mac-hig-design 技能五步流程（读图提取 → 对照 HIG 参考 → 逐项审计 → 分级报告 → HIG 引用）。HIG 参考：`references/hig/layout.md`、`typography.md`、`color.md`、`accessibility.md`、`focus-and-selection.md`、`modality.md`、`materials.md`、`dark-mode.md` + `references/design-tokens.md`（数值规范）。颜色与对比度均经像素采样实测，标「约」为估算值，精确值建议用取色工具复核。*
