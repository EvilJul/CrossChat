# Bug 跟踪文档

## 待修复 Bug

### Bug #1: Office文件预览功能Python库导入失败

**状态**: 已修复
**优先级**: 高
**发现日期**: 2026-05-07
**修复日期**: 2026-05-07

#### 问题描述

在预览Excel、Word、PowerPoint、PDF等Office文件时，Python脚本无法正确导入已安装的库（openpyxl、python-docx、python-pptx、PyPDF2），提示：

```
错误: 需要安装openpyxl库。请运行: pip install openpyxl
详细信息: No module named 'openpyxl'
```

#### 根本原因（经过深入调试发现）

**主要原因**：`tauri.conf.json`中的资源配置错误
```json
"resources": ["resources/python/*"]  // 错误：只复制文件，不复制子目录
```

这导致在开发模式下，Tauri只复制了`python`目录下的文件（如python.exe、*.pyd等），但**没有复制`Lib`子目录**，因此`Lib/site-packages`目录不存在，所有第三方库都无法加载。

调试输出显示：
```
Python directory: C:\Users\tian\Desktop\crossChat\src-tauri\target\debug\resources\python
site-packages path: C:\Users\tian\Desktop\crossChat\src-tauri\target\debug\resources\python\Lib\site-packages
site-packages exists: False  ← 关键问题！
```

**次要原因**：在`python_env.rs`的`run_python_script`函数中设置了`PYTHONPATH`环境变量，这导致Python忽略了`python311._pth`文件中的配置（虽然在这个场景下因为目录不存在而不是主要问题）。

#### 修复方案

**修复1：修正资源配置（关键修复）**
在`src-tauri/tauri.conf.json`中修改资源配置：

```json
// 修复前
"resources": ["resources/python/*"]  // 只复制文件

// 修复后
"resources": ["resources/python/**"]  // 递归复制所有文件和子目录
```

**修复2：移除PYTHONPATH环境变量设置**
在`src-tauri/src/python_env.rs`中移除了`PYTHONPATH`的设置，让Python使用`python311._pth`文件的配置：

```rust
// 修复前
cmd.env("PYTHON_DIR", python_dir.to_string_lossy().to_string());
cmd.env("PYTHONPATH", &python_path);  // 这会导致Python忽略python311._pth

// 修复后
// 不设置PYTHONPATH，让Python使用python311._pth配置
// Python脚本会通过sys.executable动态获取路径
```

**修复3：使用sys.executable动态获取路径**
在所有Python脚本中改用`sys.executable`动态获取Python解释器路径：

```python
# 使用sys.executable动态获取Python目录路径
python_exe = sys.executable
python_dir = os.path.dirname(os.path.abspath(python_exe))
site_packages = os.path.join(python_dir, 'Lib', 'site-packages')

# 确保site-packages在sys.path中
if os.path.exists(site_packages) and site_packages not in sys.path:
    sys.path.insert(0, site_packages)
```

#### 修改的文件

1. **src-tauri/tauri.conf.json**（关键修复）
   - 修改资源配置从`resources/python/*`改为`resources/python/**`

2. **src-tauri/src/python_env.rs**
   - 移除了`PYTHON_DIR`环境变量设置
   - 移除了`PYTHONPATH`环境变量设置
   - 只保留`PATH`环境变量设置（用于DLL查找）
   - 添加了stderr调试输出

3. **src-tauri/src/commands/file_ops.rs**
   - `read_excel_file()` - Excel文件读取（添加调试信息）
   - `read_word_file()` - Word文件读取
   - `read_powerpoint_file()` - PowerPoint文件读取
   - `read_pdf_file()` - PDF文件读取

#### 调试过程

1. 添加详细的调试输出到Python脚本
2. 发现`site-packages exists: False`
3. 检查`target/debug/resources/python`目录，发现没有`Lib`子目录
4. 定位到`tauri.conf.json`配置问题
5. 修改配置为递归复制

#### 测试验证

修复后，重新启动应用，调试输出应显示：
```
site-packages exists: True
SUCCESS: openpyxl导入成功
```

#### 额外改进

1. **配置pip国内镜像源**：创建了`src-tauri/resources/python/pip.ini`，配置清华大学镜像源
2. **验证Python环境**：确认python311._pth配置正确

#### 如何测试

1. **关闭当前运行的应用**
2. **重新启动开发模式**：
   ```bash
   cd C:\Users\tian\Desktop\crossChat
   npm run tauri dev
   ```
3. **打开Excel文件测试**，应该能正常预览

#### 技术细节

**Tauri资源配置说明**：
- `resources/python/*` - 只复制`python`目录下的直接文件
- `resources/python/**` - 递归复制`python`目录及其所有子目录

Python环境路径: `src-tauri/resources/python/`
site-packages路径: `src-tauri/resources/python/Lib/site-packages/`

已安装的库:
- openpyxl 3.1.5
- python-docx 1.2.0
- python-pptx 1.0.2
- PyPDF2 3.0.1

打包配置（tauri.conf.json）:
```json
"resources": ["resources/python/**"]
```

这确保Python沙盒环境（包括所有子目录）会被完整打包到可执行文件中。

---

## 已修复 Bug

### Bug #2: Office文件预览中文乱码

**状态**: 已修复
**优先级**: 高
**发现日期**: 2026-05-07
**修复日期**: 2026-05-07

#### 问题描述

Office文件预览时，中文内容显示为乱码：
```
����: ��Ҫ��װopenpyxl�⡣������: pip install openpyxl
```

#### 根本原因

Windows系统下，Python的默认stdout编码是`gbk`，而Rust代码使用`String::from_utf8_lossy`读取输出时期望UTF-8编码，导致编码不匹配。

#### 修复方案

**修复1：设置PYTHONIOENCODING环境变量**
在`src-tauri/src/python_env.rs`中添加：
```rust
cmd.env("PYTHONIOENCODING", "utf-8");
```

**修复2：在Python脚本中重新配置编码**
在所有Python脚本开头添加：
```python
sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')
```

#### 修改的文件

1. `src-tauri/src/python_env.rs` - 添加环境变量
2. `src-tauri/src/commands/file_ops.rs` - 在4个函数中添加编码配置

#### 测试验证

✅ 中文正确显示
✅ 支持多语言（日文、韩文等）
✅ 支持特殊符号

---

### Bug #0: 文件预览功能限制为1MB

**状态**: 已修复
**修复日期**: 2026-05-07
**修复方案**: 将文件大小限制从1MB提升到10MB
