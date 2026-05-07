# Bug 跟踪文档

## 待修复 Bug

### Bug #1: Office文件预览功能Python库导入失败

**状态**: 待修复
**优先级**: 高
**发现日期**: 2026-05-07

#### 问题描述

在预览Excel、Word、PowerPoint、PDF等Office文件时，Python脚本无法正确导入已安装的库（openpyxl、python-docx、python-pptx、PyPDF2），提示：

```
错误: 需要安装openpyxl库。请运行: pip install openpyxl
详细信息: No module named 'openpyxl'
```

#### 已尝试的修复方案

1. **设置PYTHONPATH环境变量** - 未生效
2. **在Python脚本中使用sys.path.insert** - 未生效
3. **使用PYTHON_DIR环境变量传递路径** - 未生效

#### 相关文件

- `src-tauri/src/commands/file_ops.rs` - Office文件读取逻辑
- `src-tauri/src/python_env.rs` - Python环境管理
- `src-tauri/resources/python/` - Python沙盒环境目录
- `src-tauri/resources/python/Lib/site-packages/` - 已安装的Python库

#### 技术细节

Python环境路径: `src-tauri/resources/python/`
site-packages路径: `src-tauri/resources/python/Lib/site-packages/`

已安装的库:
- openpyxl 3.1.5
- python-docx 1.2.0
- python-pptx 1.0.2
- PyPDF2 3.0.1

直接运行Python脚本测试导入成功，但通过Rust的std::process::Command调用时失败。

#### 可能的原因

1. Rust调用Python时，Python解释器的工作目录或路径配置不正确
2. Python嵌入式版本的路径解析机制与标准Python不同
3. 环境变量设置方式不正确
4. Python脚本中sys.executable返回的路径不正确

#### 下一步排查方向

1. 在Python脚本中打印sys.executable和sys.path，确认实际路径
2. 检查Rust调用Python时的当前工作目录
3. 考虑使用绝对路径硬编码site-packages位置
4. 考虑将Python库打包到单独目录，不依赖site-packages
5. 考虑使用Python的`-S`参数或修改`python311._pth`文件

---

## 已修复 Bug

### Bug #0: 文件预览功能限制为1MB

**状态**: 已修复
**修复日期**: 2026-05-07
**修复方案**: 将文件大小限制从1MB提升到10MB
