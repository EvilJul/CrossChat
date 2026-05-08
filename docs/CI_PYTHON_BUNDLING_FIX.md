# CI Python环境打包问题修复

## 问题描述

编译后的产物只有8MB，没有包含Python沙盒环境（应该有50MB+）。这导致没有安装Python环境的用户无法使用应用程序。

## 根本原因

Tauri 2.x的资源配置格式错误。使用了数组格式：
```json
"resources": [
  "resources/python"
]
```

但Tauri 2.x要求使用对象格式来指定资源的源路径和目标路径。

## 解决方案

### 1. 修复Tauri配置 (src-tauri/tauri.conf.json)

将资源配置从数组格式改为对象格式：
```json
"resources": {
  "resources/python": "./"
}
```

这告诉Tauri将`resources/python`目录复制到应用程序根目录。

### 2. 增强CI验证 (.github/workflows/build.yml)

添加了多个验证步骤：

#### Windows构建：
- **Python环境验证**：检查python.exe是否存在，如果不存在则失败
- **目录大小检查**：在构建前显示Python目录大小（应该是70-80MB）
- **打包内容检查**：使用7z检查NSIS安装包是否包含Python文件

#### Linux构建：
- **Python环境验证**：检查python3可执行文件是否存在
- **目录大小检查**：在构建前显示Python目录大小
- **打包内容检查**：使用dpkg检查DEB包是否包含Python文件

### 3. 之前已修复的编码问题

- `setup_python.py`：所有中文消息已替换为英文
- `install_office_deps.py`：所有中文消息已替换为英文
- 两个文件都添加了UTF-8编码强制设置

## 预期结果

修复后的构建产物大小：
- **Windows NSIS**: ~60-70MB（包含Python环境）
- **Windows MSI**: ~60-70MB（包含Python环境）
- **Linux DEB**: ~60-70MB（包含Python环境）
- **Linux RPM**: ~60-70MB（包含Python环境）

## 验证方法

### 本地验证
```bash
# 检查本地Python目录大小
Get-ChildItem -Path "src-tauri/resources/python" -Recurse -File | Measure-Object -Property Length -Sum

# 应该显示约75MB
```

### CI验证
查看CI日志中的以下输出：
1. "Python Environment Size" - 应该显示70-80MB
2. "Checking if Python is bundled" - 应该找到Python相关文件
3. 最终产物大小 - 应该是60-70MB而不是8MB

## 相关文件

- `src-tauri/tauri.conf.json` - Tauri配置文件
- `.github/workflows/build.yml` - CI工作流
- `src-tauri/resources/setup_python.py` - Python环境设置脚本
- `src-tauri/resources/install_office_deps.py` - Python依赖安装脚本

## 注意事项

1. **缓存问题**：GitHub Actions会缓存Python环境，如果缓存损坏，需要清除缓存
2. **路径问题**：确保`resources/python`目录在构建前存在且完整
3. **权限问题**：Linux上需要确保Python可执行文件有执行权限（setup_python.py已处理）

## 时间线

- **v0.2.3**: 初始版本，发现打包问题
- **2026-05-08**: 修复Windows编码问题
- **2026-05-08**: 修复Linux AppImage打包失败（改用DEB+RPM）
- **2026-05-08**: 修复Python环境打包问题（本次修复）
