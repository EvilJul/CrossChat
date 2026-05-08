#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Setup Python sandbox environment script
Download and configure Python embedded version
Support Windows and Linux platforms
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

# Force UTF-8 encoding for stdout/stderr (fix Windows CI encoding issues)
if sys.stdout.encoding != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
if sys.stderr.encoding != 'utf-8':
    sys.stderr.reconfigure(encoding='utf-8')

# Python version configuration
PYTHON_VERSION = "3.11.9"

# Select download URL based on platform
if platform.system() == "Windows":
    PYTHON_EMBED_URL = f"https://www.python.org/ftp/python/{PYTHON_VERSION}/python-{PYTHON_VERSION}-embed-amd64.zip"
    ARCHIVE_TYPE = "zip"
elif platform.system() == "Linux":
    # Linux uses pre-compiled version from python-build-standalone project
    PYTHON_EMBED_URL = f"https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.11.7+20240107-x86_64-unknown-linux-gnu-install_only.tar.gz"
    ARCHIVE_TYPE = "tar.gz"
else:
    print(f"Unsupported platform: {platform.system()}")
    sys.exit(1)

def download_with_retry(url, path, max_retries=3):
    """Download function with retry"""
    for attempt in range(max_retries):
        try:
            print(f"Download attempt {attempt + 1}/{max_retries}...")
            urllib.request.urlretrieve(url, path)
            print(f"Download successful: {path}")
            return True
        except Exception as e:
            print(f"Download failed: {e}")
            if attempt < max_retries - 1:
                wait_time = 2 ** attempt
                print(f"Waiting {wait_time} seconds before retry...")
                time.sleep(wait_time)
            else:
                print("Max retries reached, download failed")
                raise
    return False

def setup_pip_config(python_dir):
    """Configure pip mirror source"""
    print("Configuring pip mirror...")
    
    if platform.system() == "Windows":
        pip_config = python_dir / "pip.ini"
    else:
        # Linux pip config is in pip.conf
        pip_config = python_dir / "pip.conf"
    
    config_content = """[global]
index-url = https://pypi.tuna.tsinghua.edu.cn/simple
trusted-host = pypi.tuna.tsinghua.edu.cn

[install]
trusted-host = pypi.tuna.tsinghua.edu.cn
"""
    
    try:
        pip_config.write_text(config_content)
        print(f"pip config file created: {pip_config}")
    except Exception as e:
        print(f"Failed to create pip config file: {e}")

def download_python_embed():
    """Download Python embedded version"""
    resources_dir = Path(__file__).parent
    python_dir = resources_dir / "python"
    
    # Skip if Python directory already exists
    if platform.system() == "Windows":
        python_exe = python_dir / "python.exe"
    else:
        python_exe = python_dir / "bin" / "python3"
    
    if python_dir.exists() and python_exe.exists():
        print(f"Python directory already exists: {python_dir}")
        print(f"Python executable: {python_exe}")
        return python_dir

    print(f"Downloading Python {PYTHON_VERSION} embedded version ({platform.system()})...")

    try:
        if ARCHIVE_TYPE == "zip":
            archive_path = resources_dir / "python-embed.zip"
        else:
            archive_path = resources_dir / "python-embed.tar.gz"

        # Download Python embedded version (with retry)
        print(f"Downloading from {PYTHON_EMBED_URL}...")
        download_with_retry(PYTHON_EMBED_URL, archive_path)

        # Extract to python directory
        print("Extracting files...")
        python_dir.mkdir(exist_ok=True)
        
        if ARCHIVE_TYPE == "zip":
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                zip_ref.extractall(python_dir)
        else:
            with tarfile.open(archive_path, 'r:gz') as tar_ref:
                tar_ref.extractall(python_dir)
                # python-build-standalone has an extra python directory layer
                extracted_dir = python_dir / "python"
                if extracted_dir.exists():
                    for item in extracted_dir.iterdir():
                        shutil.move(str(item), str(python_dir / item.name))
                    extracted_dir.rmdir()

        # Delete archive file
        archive_path.unlink()
        print("Extraction complete")

        if platform.system() == "Windows":
            # Windows specific configuration
            # Create site-packages directory
            site_packages = python_dir / "Lib" / "site-packages"
            site_packages.mkdir(parents=True, exist_ok=True)

            # Modify python311._pth file to enable site-packages
            pth_file = python_dir / "python311._pth"
            if pth_file.exists():
                content = pth_file.read_text()
                content = content.replace("#import site", "import site")
                pth_file.write_text(content)
                print("site-packages enabled")
        else:
            # Linux specific configuration
            # Ensure executable permissions
            bin_dir = python_dir / "bin"
            if bin_dir.exists():
                for exe_file in bin_dir.glob("python*"):
                    os.chmod(exe_file, 0o755)
                print("Executable permissions set")

        # Configure pip mirror
        setup_pip_config(python_dir)

        print(f"Python environment setup complete: {python_dir}")
        return python_dir

    except Exception as e:
        print(f"Failed to setup Python environment: {e}")
        import traceback
        traceback.print_exc()
        # Clean up failed installation
        if 'archive_path' in locals() and archive_path.exists():
            archive_path.unlink()
        if python_dir.exists():
            shutil.rmtree(python_dir)
        raise

def install_pip(python_dir):
    """Install pip to Python environment"""
    print("Installing pip...")

    # Download get-pip.py
    get_pip_url = "https://bootstrap.pypa.io/get-pip.py"
    get_pip_path = python_dir / "get-pip.py"

    try:
        download_with_retry(get_pip_url, get_pip_path)

        # Find Python executable
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
                print("pip installation complete")
            else:
                print(f"pip installation failed: {result.stderr}")
            get_pip_path.unlink()
        else:
            print(f"Python executable not found: {python_exe}")

    except Exception as e:
        print(f"Failed to install pip: {e}")
        import traceback
        traceback.print_exc()
        if get_pip_path.exists():
            get_pip_path.unlink()

if __name__ == "__main__":
    try:
        python_dir = download_python_embed()
        install_pip(python_dir)
        print("\n" + "="*50)
        print("Python sandbox environment setup complete!")
        print("="*50)
        
        # Display Python information
        if platform.system() == "Windows":
            python_exe = python_dir / "python.exe"
        else:
            python_exe = python_dir / "bin" / "python3"
        
        if python_exe.exists():
            import subprocess
            result = subprocess.run([str(python_exe), "--version"], capture_output=True, text=True)
            print(f"Python version: {result.stdout.strip()}")
            
            result = subprocess.run([str(python_exe), "-m", "pip", "--version"], capture_output=True, text=True)
            print(f"pip version: {result.stdout.strip()}")
        
    except Exception as e:
        print(f"\nSetup failed: {e}")
        sys.exit(1)
