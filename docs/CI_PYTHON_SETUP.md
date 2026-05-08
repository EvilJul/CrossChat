# CI 构建中的 Python 环境打包问题修复

## 问题描述

在 GitHub Actions 构建时，Python 沙盒环境没有被打包进最终的安装包中，导致用户下载后无法使用依赖 Python 的功能（如 MCP 插件）。

## 根本原因

`.gitignore` 文件中排除了 Python 环境目录：

```gitignore
# Python沙盒环境（文件较大，不提交到git）
src-tauri/resources/python/
!src-tauri/resources/python/pip.ini
```

这导致 Python 环境文件没有被提交到 Git 仓库，GitHub Actions 在构建时无法获取这些文件。

## 解决方案

### 方案选择：动态下载和设置（已采用）

在 GitHub Actions 构建流程中，动态下载和配置 Python 环境，而不是将大文件提交到 Git 仓库。

**优点：**
- 保持 Git 仓库体积小
- 支持跨平台（Windows 和 Linux）
- 易于更新 Python 版本
- 符合最佳实践

### 实施的更改

#### 1. 更新 `setup_python.py` 脚本

增加了跨平台支持：

- **Windows**: 下载 Python 官方嵌入式版本（embed-amd64.zip）
- **Linux**: 使用 python-build-standalone 项目的预编译版本

关键改进：
- 自动检测操作系统
- 处理不同的压缩格式（zip/tar.gz）
- 设置正确的文件权限（Linux）
- 更好的错误处理和日志输出

#### 2. 更新 GitHub Actions 工作流

在 `.github/workflows/build.yml` 中添加了 Python 环境设置步骤：

**Windows 构建：**
```yaml
- name: Setup Python
  uses: actions/setup-python@v5
  with:
    python-version: "3.11"

- name: Setup Python sandbox environment
  run: |
    cd src-tauri/resources
    python setup_python.py
  shell: bash
```

**Linux 构建：**
```yaml
- name: Setup Python
  uses: actions/setup-python@v5
  with:
    python-version: "3.11"

- name: Setup Python sandbox environment
  run: |
    cd src-tauri/resources
    python setup_python.py
```

#### 3. Tauri 配置保持不变

`src-tauri/tauri.conf.json` 中的资源配置已经正确：

```json
"bundle": {
  "resources": [
    "resources/python"
  ]
}
```

## 构建流程

1. **Checkout 代码** - 获取源代码（不包含 Python 环境）
2. **Setup Python** - 安装 Python 3.11（用于运行设置脚本）
3. **Setup Python sandbox** - 运行 `setup_python.py` 下载并配置嵌入式 Python
4. **Build Tauri app** - Tauri 构建时会将 `resources/python` 目录打包进安装包

## 验证方法

### 本地验证

```bash
# 删除现有 Python 环境
rm -rf src-tauri/resources/python

# 运行设置脚本
cd src-tauri/resources
python setup_python.py

# 验证 Python 环境
ls -la python/
```

### CI 验证

1. 推送代码到 GitHub
2. 查看 Actions 运行日志，确认 "Setup Python sandbox environment" 步骤成功
3. 下载构建产物（Artifacts）
4. 解压安装包，检查是否包含 `resources/python` 目录

## 注意事项

1. **网络依赖**: 构建过程需要从互联网下载 Python 嵌入式版本（约 10-20 MB）
2. **构建时间**: 首次下载会增加约 30-60 秒的构建时间
3. **缓存**: 可以考虑添加 GitHub Actions 缓存来加速后续构建
4. **版本更新**: 修改 `setup_python.py` 中的 `PYTHON_VERSION` 变量即可更新 Python 版本

## 未来优化建议

### 1. 添加构建缓存

```yaml
- name: Cache Python environment
  uses: actions/cache@v4
  with:
    path: src-tauri/resources/python
    key: python-3.11.9-${{ runner.os }}
```

### 2. 预安装常用依赖

在 `setup_python.py` 中添加：

```python
def install_common_packages(python_dir):
    """预安装常用的 Python 包"""
    packages = ["requests", "beautifulsoup4", "lxml"]
    # ... 安装逻辑
```

### 3. 支持 macOS

添加 macOS 平台的 Python 嵌入式版本下载和配置。

## 相关文件

- `.github/workflows/build.yml` - CI 构建配置
- `src-tauri/resources/setup_python.py` - Python 环境设置脚本
- `src-tauri/tauri.conf.json` - Tauri 资源打包配置
- `.gitignore` - Git 忽略规则（保持 Python 目录被忽略）
