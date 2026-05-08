#!/usr/bin/env python3
"""
设置Python沙盒环境脚本
用于下载和配置Python嵌入式版本
支持 Windows 和 Linux 平台
"""

import os
import sys
import zipfile
import tarfile
import urllib.request
import shutil
import platform
import time
from pathlib import Path

# Python版本配置
PYTHON_VERSION = "3.11.9"

# 根据平台选择下载URL
if platform.system() == "Windows":
    PYTHON_EMBED_URL = f"https://www.python.org/ftp/python/{PYTHON_VERSION}/python-{PYTHON_VERSION}-embed-amd64.zip"
    ARCHIVE_TYPE = "zip"
elif platform.system() == "Linux":
    # Linux 使用 python-build-standalone 项目的预编译版本
    PYTHON_EMBED_URL = f"https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.11.7+20240107-x86_64-unknown-linux-gnu-install_only.tar.gz"
    ARCHIVE_TYPE = "tar.gz"
else:
    print(f"不支持的平台: {platform.system()}")
    sys.exit(1)

def download_with_retry(url, path, max_retries=3):
    """带重试的下载函数"""
    for attempt in range(max_retries):
        try:
            print(f"下载尝试 {attempt + 1}/{max_retries}...")
            urllib.request.urlretrieve(url, path)
            print(f"下载成功: {path}")
            return True
        except Exception as e:
            print(f"下载失败: {e}")
            if attempt < max_retries - 1:
                wait_time = 2 ** attempt
                print(f"等待 {wait_time} 秒后重试...")
                time.sleep(wait_time)
            else:
                print("达到最大重试次数，下载失败")
                raise
    return False

def setup_pip_config(python_dir):
    """配置 pip 国内镜像源"""
    print("配置 pip 镜像源...")
    
    if platform.system() == "Windows":
        pip_config = python_dir / "pip.ini"
    else:
        # Linux 的 pip 配置在 pip.conf
        pip_config = python_dir / "pip.conf"
    
    config_content = """[global]
index-url = https://pypi.tuna.tsinghua.edu.cn/simple
trusted-host = pypi.tuna.tsinghua.edu.cn

[install]
trusted-host = pypi.tuna.tsinghua.edu.cn
"""
    
    try:
        pip_config.write_text(config_content)
        print(f"pip 配置文件已创建: {pip_config}")
    except Exception as e:
        print(f"创建 pip 配置文件失败: {e}")

def download_python_embed():
    """下载Python嵌入式版本"""
    resources_dir = Path(__file__).parent
    python_dir = resources_dir / "python"
    
    # 如果Python目录已存在，跳过
    if platform.system() == "Windows":
        python_exe = python_dir / "python.exe"
    else:
        python_exe = python_dir / "bin" / "python3"
    
    if python_dir.exists() and python_exe.exists():
        print(f"Python目录已存在: {python_dir}")
        print(f"Python可执行文件: {python_exe}")
        return python_dir

    print(f"下载Python {PYTHON_VERSION} 嵌入式版本 ({platform.system()})...")

    try:
        if ARCHIVE_TYPE == "zip":
            archive_path = resources_dir / "python-embed.zip"
        else:
            archive_path = resources_dir / "python-embed.tar.gz"

        # 下载Python嵌入式版本（带重试）
        print(f"从 {PYTHON_EMBED_URL} 下载...")
        download_with_retry(PYTHON_EMBED_URL, archive_path)

        # 解压到python目录
        print("解压文件...")
        python_dir.mkdir(exist_ok=True)
        
        if ARCHIVE_TYPE == "zip":
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                zip_ref.extractall(python_dir)
        else:
            with tarfile.open(archive_path, 'r:gz') as tar_ref:
                tar_ref.extractall(python_dir)
                # python-build-standalone 解压后有一层 python 目录
                extracted_dir = python_dir / "python"
                if extracted_dir.exists():
                    for item in extracted_dir.iterdir():
                        shutil.move(str(item), str(python_dir / item.name))
                    extracted_dir.rmdir()

        # 删除压缩文件
        archive_path.unlink()
        print("解压完成")

        if platform.system() == "Windows":
            # Windows 特定配置
            # 创建site-packages目录
            site_packages = python_dir / "Lib" / "site-packages"
            site_packages.mkdir(parents=True, exist_ok=True)

            # 修改python311._pth文件，启用site-packages
            pth_file = python_dir / "python311._pth"
            if pth_file.exists():
                content = pth_file.read_text()
                content = content.replace("#import site", "import site")
                pth_file.write_text(content)
                print("已启用 site-packages")
        else:
            # Linux 特定配置
            # 确保可执行权限
            bin_dir = python_dir / "bin"
            if bin_dir.exists():
                for exe_file in bin_dir.glob("python*"):
                    os.chmod(exe_file, 0o755)
                print("已设置可执行权限")

        # 配置 pip 镜像源
        setup_pip_config(python_dir)

        print(f"Python环境设置完成: {python_dir}")
        return python_dir

    except Exception as e:
        print(f"设置Python环境失败: {e}")
        import traceback
        traceback.print_exc()
        # 清理失败的安装
        if 'archive_path' in locals() and archive_path.exists():
            archive_path.unlink()
        if python_dir.exists():
            shutil.rmtree(python_dir)
        raise

def install_pip(python_dir):
    """安装pip到Python环境"""
    print("安装pip...")

    # 下载get-pip.py
    get_pip_url = "https://bootstrap.pypa.io/get-pip.py"
    get_pip_path = python_dir / "get-pip.py"

    try:
        download_with_retry(get_pip_url, get_pip_path)

        # 找到Python可执行文件
        if platform.system() == "Windows":
            python_exe = python_dir / "python.exe"
        else:
            python_exe = python_dir / "bin" / "python3"

        if python_exe.exists():
            import subprocess
            result = subprocess.run(
                [str(python_exe), str(get_pip_path), "--no-warn-script-location"],
                capture_output=True,
                text=True
            )
            if result.returncode == 0:
                print("pip安装完成")
            else:
                print(f"pip安装失败: {result.stderr}")
            get_pip_path.unlink()
        else:
            print(f"找不到Python可执行文件: {python_exe}")

    except Exception as e:
        print(f"安装pip失败: {e}")
        import traceback
        traceback.print_exc()
        if get_pip_path.exists():
            get_pip_path.unlink()

if __name__ == "__main__":
    try:
        python_dir = download_python_embed()
        install_pip(python_dir)
        print("\n" + "="*50)
        print("Python沙盒环境设置完成！")
        print("="*50)
        
        # 显示Python信息
        if platform.system() == "Windows":
            python_exe = python_dir / "python.exe"
        else:
            python_exe = python_dir / "bin" / "python3"
        
        if python_exe.exists():
            import subprocess
            result = subprocess.run([str(python_exe), "--version"], capture_output=True, text=True)
            print(f"Python版本: {result.stdout.strip()}")
            
            result = subprocess.run([str(python_exe), "-m", "pip", "--version"], capture_output=True, text=True)
            print(f"pip版本: {result.stdout.strip()}")
        
    except Exception as e:
        print(f"\n设置失败: {e}")
        sys.exit(1)
