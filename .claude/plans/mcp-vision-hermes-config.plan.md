# Plan: 从 Hermes 配置提取数据 → uvx 安装 mcp-vision → 自动图片解析

**复杂度**: 低  
**预计时间**: 10 分钟

---

## 1. 需求复述

从本机 Hermes Agent v0.15.1 的配置（`~/.hermes/config.yaml` + `~/.hermes/auth.json`）中提取视觉辅助相关参数，作为 mcp-vision MCP Server 的配置基础。使用 `uvx`（已安装 v0.7.8）安装运行，最终实现 Claude Code + DeepSeek 场景下的自动图片解析。

---

## 2. Hermes 配置提取与映射

### 2.1 数据来源

| 文件 | 提取节点 | 用途 |
|------|----------|------|
| `~/.hermes/config.yaml` | `auxiliary.vision` | 视觉辅助配置参数 |
| `~/.hermes/config.yaml` | `image_input_mode` | 图片处理行为模式 |
| `~/.hermes/auth.json` | `credential_pool` | 凭证池信息（确认 DeepSeek 纯文本 Key 的存在） |

### 2.2 字段映射

| Hermes 字段 | 原始值 | MCP 映射 | 操作 |
|---|---|---|---|
| `vision.provider` | `auto` | `MCP_OCR_PROVIDER` | **待定**：hermes 自动选择，MCP 需显式指定具体 provider |
| `vision.model` | `''` | `SILICONFLOW_MODEL` | **补充**：需指定，推荐 `deepseek-ai/DeepSeek-OCR` |
| `vision.timeout` | `120` | `MCP_OCR_TIMEOUT` | **复用**：设为 `120`，对齐 hermes 超时策略 |
| `vision.download_timeout` | `30` | `MCP_OCR_DOWNLOAD_TIMEOUT` | **复用**：设为 `30`，对齐 hermes 下载超时 |
| `vision.api_key` | `''` | `SILICONFLOW_API_KEY` | **需新建**：hermes 中视觉 Key 为空，需用户从硅基流动获取 |
| `image_input_mode` | `auto` | → Skill 触发策略 | **对齐**：Skill 设为粘贴即自动触发 |

### 2.3 Hermes 视觉辅助的实际状态

经过三层数据交叉分析，结论如下：

| 数据层 | 内容 | 含义 |
|--------|------|------|
| `config.yaml` → `auxiliary.vision` | `{provider: auto, model: '', api_key: ''}` | 框架配置了，但全部为空值/auto |
| `auth.json` → `credential_pool` | 仅 1 个凭证：`deepseek`（`DEEPSEEK_API_KEY`） | 唯一的 API Key 属于**纯文本**模型 |
| `models_dev_cache.json` | 115 个 provider 支持视觉输入 | 模型目录丰富，但**没有一个有对应 API Key** |
| `logs/agent.log` | Nous 未认证、OpenRouter 信用不足 | 两个自动路由后端均不可用 |

**结论：Hermes 的辅助视觉功能实际上不可用。** `provider: auto` 找不到任何可用的视觉后端——DeepSeek 是纯文本模型（`modalities.input: ['text']`），其他 115 个视觉 provider 都没有 API Key。Hermes 只提供了一个视觉辅助的**配置框架**，但没有可用的视觉后端。

### 2.4 可提取的数据

- **可复用**（3 项）：`timeout: 120`、`download_timeout: 30`、`image_input_mode: auto`
- **不可用**：API Key（空）、model（空）、provider（auto 无可用后端）
- **需要外部补充**：视觉 API Key —— 用户已提供硅基流动 Key（已验证可用，账户状态正常，`deepseek-ai/DeepSeek-OCR` 模型就绪）

---

## 3. 实施步骤

### Task 1：获取硅基流动视觉 API Key

- **操作**：访问 [cloud.siliconflow.cn](https://cloud.siliconflow.cn) 注册账号，在「API 密钥」页面新建 Key
- **验证**：`curl -s "https://api.siliconflow.cn/v1/user/info" -H "Authorization: Bearer $KEY" | python3 -c "import sys,json; d=json.load(sys.stdin); print('OK' if d.get('status') else 'FAIL')"`

### Task 2：uvx 验证 mcp-vision 可用

- **操作**：执行 `uvx mcp-vision --help`
- **说明**：uvx 首次自动从 PyPI 拉取并缓存，无需 pip install
- **验证**：输出 help 信息无报错

### Task 3：修改 `~/.claude/settings.json`

- **操作**：在现有 `mcpServers: {}` 中添加 mcp-vision 配置
- **参数对齐说明**：

| MCP 环境变量 | 值 | 来源 |
|---|---|---|
| `SILICONFLOW_API_KEY` | `sk-xxx`（用户获取） | 新建 |
| `SILICONFLOW_MODEL` | `deepseek-ai/DeepSeek-OCR` | 推荐默认值 |
| `MCP_OCR_TIMEOUT` | `120` | 对齐 Hermes `vision.timeout` |
| `MCP_OCR_DOWNLOAD_TIMEOUT` | `30` | 对齐 Hermes `vision.download_timeout` |

- **配置模板**：

```json
{
  "mcpServers": {
    "mcp-vision": {
      "command": "uvx",
      "args": ["mcp-vision"],
      "env": {
        "SILICONFLOW_API_KEY": "<YOUR_SILICONFLOW_KEY>",
        "SILICONFLOW_MODEL": "deepseek-ai/DeepSeek-OCR",
        "MCP_OCR_TIMEOUT": "120",
        "MCP_OCR_DOWNLOAD_TIMEOUT": "30"
      }
    }
  }
}
```

### Task 4：创建自动触发 Skill

- **操作**：创建 `~/.claude/skills/image-analysis.md`
- **触发逻辑**：消息中出现 `[Image #N]` 占位符时自动调用 `analyze_image`（对齐 Hermes `image_input_mode: auto`）
- **规则**：禁止 DeepSeek 使用 Read 工具读取图片文件（DeepSeek 的常见错误行为）

### Task 5：端到端验证

- **操作**：重启 Claude Code → 检查 MCP 面板 mcp-vision 绿色 → 粘贴测试截图
- **验证**：DeepSeek 自动调用 mcp-vision 分析图片内容

---

## 4. 涉及文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `~/.claude/settings.json` | 修改 | 在空 `mcpServers` 中添加 mcp-vision |
| `~/.claude/skills/image-analysis.md` | 新建 | 图片自动触发 Skill |

---

## 5. 架构关系

```
Hermes Agent (独立进程)              Claude Code (MCP 架构)
├─ credential_pool                   ├─ DeepSeek V4 Pro (推理)
│  └─ DEEPSEEK_API_KEY               │   └─ 遇到 [Image #N]
│     (纯文本，不与 MCP 共享)          │      → 调用 MCP 工具
├─ auxiliary.vision: auto            │          ↓
│  (内部视觉路由)                     ├─ mcp-vision MCP Server
│                                    │   ├─ timeout: 120 ← hermes
│                                    │   ├─ download_timeout: 30 ← hermes
│                                    │   └─ 硅基流动视觉 API (独立 Key)
│                                    │
配置参数对齐 (timeout/download/auto)   Skill 自动触发 ← hermes image_input_mode
        ↑                                            ↑
        └──────────── 两个系统独立运行 ───────────────┘
```

---

## 6. 接受条件

- [ ] 硅基流动 API Key 已获取
- [ ] `uvx mcp-vision` 可正常启动
- [ ] `settings.json` 已更新，MCP 面板显示 mcp-vision 绿色
- [ ] 粘贴截图后 DeepSeek 自动调用 mcp-vision 分析
- [ ] Skill 禁止 DeepSeek 用 Read 读图片
- [ ] Hermes 的三项可复用参数（timeout/download/auto 行为）已映射到 MCP 配置
