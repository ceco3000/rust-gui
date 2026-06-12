# V4 验证：渲染管线跨平台三端可运行

> 验证 winit + wgpu + Vello 依赖链在 macOS/Metal、Windows/DX12、Linux/Vulkan 三端可编译运行。

## 已确认

- ✅ macOS (aarch64-apple-darwin)：本地编译通过，运行时依赖 Metal GPU
- ⬜ Linux (x86_64-unknown-linux-gnu)：待 CI runner 验证
- ⬜ Windows (x86_64-pc-windows-msvc)：待 CI runner 验证

## CI 配置

CI 矩阵参见仓库根目录 [.github/workflows/cross-platform.yml](../../.github/workflows/cross-platform.yml)。

## 手动验证

### macOS

```bash
cargo build --release --manifest-path ../v1-vello-cosmic/Cargo.toml
cargo run --release --manifest-path ../v1-vello-cosmic/Cargo.toml
```

### 离屏截图（CI 环境）

```bash
./scripts/headless-screenshot.sh verify_macos.png 2
```
