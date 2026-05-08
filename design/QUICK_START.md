# 🚀 应用图标快速生成指南

## 3 步完成图标更新

### 步骤 1️⃣: 转换 SVG 为 PNG

1. 打开浏览器访问: **https://svgtopng.com/**
2. 点击上传，选择 `design/app-icon.svg`
3. 设置尺寸为 **1024 × 1024**
4. 点击下载，保存为 `icon-1024.png` 到 `design/` 目录

### 步骤 2️⃣: 生成所有尺寸

在项目根目录打开终端，运行：

```bash
cargo tauri icon design/icon-1024.png
```

这会自动生成所有需要的图标尺寸并放入 `src-tauri/icons/` 目录。

### 步骤 3️⃣: 重新编译应用

```bash
npm run tauri build
```

---

## ✅ 完成！

安装新编译的应用，你会看到全新的紫蓝渐变图标！

---

## 🎨 图标预览

想看看图标效果？在浏览器中打开：

```
design/icon-preview.html
```

---

## ❓ 遇到问题？

### Tauri Icon 命令找不到

```bash
cargo install tauri-cli
```

### 需要更多帮助

查看完整指南: `design/ICON_DESIGN_GUIDE.md`

---

**提示**: 如果你想自定义图标，可以编辑 `app-icon.svg` 文件，然后重复上述步骤。
