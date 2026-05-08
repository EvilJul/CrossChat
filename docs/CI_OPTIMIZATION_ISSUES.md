# CI 构建优化问题清单

## 已发现的问题

### 🔴 严重问题

#### 1. Linux Python 可执行文件路径错误

**问题描述：**
`src-tauri/src/python_env.rs` 中 Linux 平台的 Python 可执行文件路径不正确：

```rust
#[cfg(not(target_os = "windows"))]
{
    python_dir.join("python3")  // ❌ 错误：应该是 bin/python3
}
```

**影响：**
- Linux 构建的应用无法找到 Python 可执行文件
- 所有依赖 Python 的功能在 Linux 上都会失败

**修复方案：**
```rust
#[cfg(not(target_os = "windows"))]
{
    python_dir.join("bin").join("python3")  // ✅ 正确
}
```

---

#### 2. setup_python.py 中 Linux Python 版本不一致

**问题描述：**
`setup_python.py` 中 Windows 使用 Python 3.11.9，但 Linux 使用 3.11.7：

```python
PYTHON_VERSION = "3.11.9"  # Windows 版本

# Linux 使用不同版本
PYTHON_EMBED_URL = "...cpython-3.11.7+20240107..."  # ❌ 版本不一致
```

**影响：**
- 跨平台版本不一致可能导致兼容性问题
- 用户在不同平台上可能遇到不同的 bug

**修复方案：**
使用更新的 python-build-standalone 版本，或明确说明版本差异的原因。

---

### 🟡 中等问题

#### 3. 缺少 CI 构建缓存

**问题描述：**
每次 CI 构建都需要重新下载 Python 环境（10-20 MB），增加构建时间和网络负担。

**影响：**
- 构建时间增加 30-60 秒
- 依赖外部网络稳定性
- 浪费 GitHub Actions 配额

**修复方案：**
添加 GitHub Actions 缓存：

```yaml
- name: Cache Python environment
  uses: actions/cache@v4
  with:
    path: src-tauri/resources/python
    key: python-embed-${{ runner.os }}-3.11.9
    restore-keys: |
      python-embed-${{ runner.os }}-
```

---

#### 4. 缺少 Python 环境验证步骤

**问题描述：**
CI 构建后没有验证 Python 环境是否正确设置。

**影响：**
- 可能打包了损坏的 Python 环境
- 用户下载后才发现问题

**修复方案：**
在构建后添加验证步骤：

```yaml
- name: Verify Python environment
  run: |
    if [ "$RUNNER_OS" == "Windows" ]; then
      ./src-tauri/resources/python/python.exe --version
    else
      ./src-tauri/resources/python/bin/python3 --version
    fi
  shell: bash
```

---

#### 5. 缺少 Office 依赖库的预安装

**问题描述：**
`install_office_deps.py` 脚本存在，但 CI 构建时没有运行，导致用户首次使用时需要下载这些库。

**影响：**
- 用户首次使用 Office 文件预览功能时需要等待下载
- 可能因网络问题导致安装失败

**修复方案：**
在 CI 构建时预安装这些依赖：

```yaml
- name: Install Python dependencies
  run: |
    cd src-tauri/resources
    if [ "$RUNNER_OS" == "Windows" ]; then
      python/python.exe install_office_deps.py
    else
      python/bin/python3 install_office_deps.py
    fi
  shell: bash
```

---

### 🟢 轻微问题

#### 6. pip.ini 配置文件未被打包

**问题描述：**
`.gitignore` 中有 `!src-tauri/resources/python/pip.ini`，但该文件在 Python 环境下载前不存在。

**影响：**
- 用户在国内使用时 pip 安装速度慢
- 可能因网络问题导致安装失败

**修复方案：**
在 `setup_python.py` 中自动创建 pip.ini：

```python
def setup_pip_config(python_dir):
    """配置 pip 国内镜像源"""
    if platform.system() == "Windows":
        pip_ini = python_dir / "pip.ini"
    else:
        pip_ini = python_dir / "pip.conf"
    
    config_content = """[global]
index-url = https://pypi.tuna.tsinghua.edu.cn/simple
trusted-host = pypi.tuna.tsinghua.edu.cn

[install]
trusted-host = pypi.tuna.tsinghua.edu.cn
"""
    pip_ini.write_text(config_content)
    print(f"pip 配置文件已创建: {pip_ini}")
```

---

#### 7. 缺少构建产物大小检查

**问题描述：**
没有检查最终安装包的大小，可能打包了不必要的文件。

**影响：**
- 安装包体积过大
- 下载和安装时间增加

**修复方案：**
添加构建产物大小报告：

```yaml
- name: Check bundle size
  run: |
    echo "=== Build Artifacts Size ==="
    if [ "$RUNNER_OS" == "Windows" ]; then
      du -sh src-tauri/target/release/bundle/msi/*.msi
      du -sh src-tauri/target/release/bundle/nsis/*.exe
    else
      du -sh src-tauri/target/release/bundle/appimage/*.AppImage
      du -sh src-tauri/target/release/bundle/deb/*.deb
    fi
  shell: bash
```

---

#### 8. 缺少错误处理和重试机制

**问题描述：**
`setup_python.py` 下载失败时没有重试机制。

**影响：**
- 网络波动可能导致构建失败
- 需要手动重新触发构建

**修复方案：**
添加下载重试逻辑：

```python
def download_with_retry(url, path, max_retries=3):
    """带重试的下载函数"""
    for attempt in range(max_retries):
        try:
            print(f"下载尝试 {attempt + 1}/{max_retries}...")
            urllib.request.urlretrieve(url, path)
            return True
        except Exception as e:
            print(f"下载失败: {e}")
            if attempt < max_retries - 1:
                time.sleep(2 ** attempt)  # 指数退避
            else:
                raise
    return False
```

---

## 优先级修复顺序

### 立即修复（阻塞性问题）
1. ✅ **Linux Python 可执行文件路径错误** - 导致 Linux 版本完全无法使用 Python 功能

### 高优先级（影响用户体验）
2. **添加 CI 构建缓存** - 减少构建时间和网络依赖
3. **预安装 Office 依赖库** - 改善首次使用体验
4. **添加 Python 环境验证** - 确保构建质量

### 中优先级（改进稳定性）
5. **统一跨平台 Python 版本** - 提高一致性
6. **添加下载重试机制** - 提高构建成功率
7. **自动创建 pip 配置** - 改善国内用户体验

### 低优先级（监控和优化）
8. **添加构建产物大小检查** - 监控包体积

---

## 相关文件

- `.github/workflows/build.yml` - CI 构建配置
- `src-tauri/resources/setup_python.py` - Python 环境设置脚本
- `src-tauri/src/python_env.rs` - Python 环境路径管理
- `src-tauri/resources/install_office_deps.py` - Office 依赖安装脚本
