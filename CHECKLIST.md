# CI 构建优化检查清单

## 📋 修复完成情况

### ✅ 已完成的修复

- [x] **修复 Linux Python 可执行文件路径错误**
  - 文件: `src-tauri/src/python_env.rs`
  - 修改: `python3` → `bin/python3`

- [x] **添加 CI 构建缓存**
  - 文件: `.github/workflows/build.yml`
  - Windows 缓存 key: `python-embed-windows-3.11.9`
  - Linux 缓存 key: `python-embed-linux-3.11.7`

- [x] **添加 Python 环境验证步骤**
  - 验证 Python 版本
  - 验证 pip 版本

- [x] **预安装 Office 依赖库**
  - openpyxl (Excel)
  - python-docx (Word)
  - python-pptx (PowerPoint)
  - PyPDF2 (PDF)

- [x] **改进 setup_python.py 脚本**
  - 添加下载重试机制（最多 3 次，指数退避）
  - 自动创建 pip 配置文件（国内镜像源）
  - 改进日志输出
  - 改进错误处理

- [x] **添加构建产物大小检查**
  - Windows: MSI 和 NSIS 安装包
  - Linux: AppImage 和 DEB 包

- [x] **创建详细文档**
  - `docs/CI_PYTHON_SETUP.md` - 问题和解决方案
  - `docs/CI_OPTIMIZATION_ISSUES.md` - 问题清单
  - `docs/CI_FIXES_SUMMARY.md` - 修复总结

---

## 🔍 需要验证的项目

### 本地验证

```bash
# 1. 清理现有 Python 环境
rm -rf src-tauri/resources/python

# 2. 运行设置脚本（Windows）
cd src-tauri/resources
python setup_python.py

# 3. 运行设置脚本（Linux）
cd src-tauri/resources
python3 setup_python.py

# 4. 验证 Python 环境
# Windows
./src-tauri/resources/python/python.exe --version
./src-tauri/resources/python/python.exe -m pip list

# Linux
./src-tauri/resources/python/bin/python3 --version
./src-tauri/resources/python/bin/python3 -m pip list

# 5. 构建应用
npm run tauri build

# 6. 检查构建产物
ls -lh src-tauri/target/release/bundle/
```

### CI 验证

- [ ] 推送代码到 GitHub
- [ ] 查看 GitHub Actions 运行日志
- [ ] 确认 "Cache Python environment" 步骤
- [ ] 确认 "Setup Python sandbox environment" 步骤
- [ ] 确认 "Verify Python environment" 步骤
- [ ] 确认 "Install Python dependencies" 步骤
- [ ] 确认 "Check bundle size" 输出
- [ ] 下载构建产物（Artifacts）
- [ ] 检查安装包大小是否合理

### 功能验证

- [ ] **Windows 平台**
  - [ ] 安装 MSI 或 NSIS 安装包
  - [ ] 测试 Office 文件预览（Excel, Word, PowerPoint, PDF）
  - [ ] 测试 MCP 插件功能
  - [ ] 测试 Python 脚本执行
  - [ ] 检查 Python 环境路径

- [ ] **Linux 平台**
  - [ ] 安装 AppImage 或 DEB 包
  - [ ] 测试 Office 文件预览
  - [ ] 测试 MCP 插件功能
  - [ ] 测试 Python 脚本执行
  - [ ] 检查 Python 环境路径和权限

---

## 📊 预期结果

### 构建日志应包含

```
✓ Setup Python
✓ Cache Python environment
  - Cache hit: false (首次) / true (后续)
✓ Setup Python sandbox environment
  下载Python 3.11.9 嵌入式版本 (Windows)
  下载尝试 1/3...
  下载成功: ...
  解压完成
  已启用 site-packages
  pip 配置文件已创建: ...
  pip安装完成
  Python沙盒环境设置完成！
  Python版本: Python 3.11.9
  pip版本: pip 24.x.x
✓ Verify Python environment
  Python 3.11.9
  pip 24.x.x
✓ Install Python dependencies
  安装 openpyxl...
  openpyxl 安装成功
  安装 python-docx...
  python-docx 安装成功
  安装 python-pptx...
  python-pptx 安装成功
  安装 PyPDF2...
  PyPDF2 安装成功
  安装完成: 4/4 个包安装成功
✓ Build Tauri app
✓ Check bundle size
  === Windows Build Artifacts Size ===
  45M  src-tauri/target/release/bundle/msi/crosschat_0.2.2_x64_en-US.msi
  45M  src-tauri/target/release/bundle/nsis/crosschat_0.2.2_x64-setup.exe
```

### 安装包应包含

```
安装包/
├── crosschat.exe (主程序)
├── resources/
│   └── python/
│       ├── python.exe (Windows) 或 bin/python3 (Linux)
│       ├── python311.dll (Windows)
│       ├── Lib/ (Windows) 或 lib/ (Linux)
│       │   └── site-packages/
│       │       ├── openpyxl/
│       │       ├── docx/
│       │       ├── pptx/
│       │       └── PyPDF2/
│       └── pip.ini (Windows) 或 pip.conf (Linux)
└── ...
```

---

## 🚨 常见问题排查

### 问题 1: Python 下载失败

**症状**: `下载失败: HTTP Error 404` 或 `Connection timeout`

**解决方案**:
1. 检查网络连接
2. 验证下载 URL 是否有效
3. 重试机制会自动处理（最多 3 次）
4. 如果持续失败，考虑使用镜像源

### 问题 2: 缓存未命中

**症状**: 每次构建都重新下载 Python

**解决方案**:
1. 检查缓存 key 是否正确
2. 验证 `path` 参数是否正确
3. 查看 GitHub Actions 缓存使用情况

### 问题 3: Python 环境验证失败

**症状**: `python.exe: command not found` 或 `Permission denied`

**解决方案**:
1. 检查 Python 可执行文件路径
2. Linux: 检查文件权限 (`chmod +x`)
3. 查看 `setup_python.py` 日志

### 问题 4: Office 依赖安装失败

**症状**: `pip install` 失败或超时

**解决方案**:
1. 检查 pip 配置是否正确
2. 验证网络连接
3. 尝试使用不同的镜像源
4. 检查 Python 环境是否正确设置

### 问题 5: 构建产物过大

**症状**: 安装包超过 60 MB

**解决方案**:
1. 检查是否包含了不必要的文件
2. 考虑移除未使用的 Python 库
3. 压缩 Python 环境

---

## 📝 提交清单

### 提交前检查

- [x] 所有代码修改已完成
- [x] 文档已更新
- [x] 本地测试通过
- [ ] 代码已格式化
- [ ] 提交信息清晰

### 提交命令

```bash
# 查看修改
git status

# 添加修改的文件
git add .github/workflows/build.yml
git add src-tauri/src/python_env.rs
git add src-tauri/resources/setup_python.py
git add docs/CI_PYTHON_SETUP.md
git add docs/CI_OPTIMIZATION_ISSUES.md
git add docs/CI_FIXES_SUMMARY.md
git add CHECKLIST.md

# 提交
git commit -m "fix: 修复 CI 构建中 Python 环境打包问题

主要修复:
- 修复 Linux Python 可执行文件路径错误
- 添加 CI 构建缓存，减少构建时间
- 添加 Python 环境验证步骤
- 预安装 Office 文件处理依赖库
- 改进 setup_python.py 脚本（重试机制、pip 配置）
- 添加构建产物大小检查

详细信息请查看:
- docs/CI_PYTHON_SETUP.md
- docs/CI_OPTIMIZATION_ISSUES.md
- docs/CI_FIXES_SUMMARY.md"

# 推送
git push origin master
```

---

## 🎯 下一步行动

### 立即执行

1. **提交代码**
   ```bash
   git add .
   git commit -m "fix: 修复 CI 构建中 Python 环境打包问题"
   git push
   ```

2. **监控构建**
   - 访问 GitHub Actions 页面
   - 查看构建日志
   - 确认所有步骤成功

3. **下载测试**
   - 下载构建产物
   - 在干净的系统上安装
   - 测试所有 Python 相关功能

### 后续优化

1. **监控构建性能**（1 周内）
   - 跟踪构建时间
   - 跟踪缓存命中率
   - 跟踪构建成功率

2. **收集用户反馈**（2 周内）
   - Office 文件预览功能
   - MCP 插件功能
   - 安装包大小

3. **持续优化**（1 月内）
   - 优化安装包大小
   - 改进构建速度
   - 添加更多测试

---

## 📚 相关文档

- [CI_PYTHON_SETUP.md](docs/CI_PYTHON_SETUP.md) - 问题描述和解决方案
- [CI_OPTIMIZATION_ISSUES.md](docs/CI_OPTIMIZATION_ISSUES.md) - 详细问题清单
- [CI_FIXES_SUMMARY.md](docs/CI_FIXES_SUMMARY.md) - 修复总结和对比
- [README.md](README.md) - 项目说明
- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构文档

---

**最后更新**: 2026-05-08
**版本**: 0.2.2
**状态**: ✅ 修复完成，等待验证
