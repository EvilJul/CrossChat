#!/usr/bin/env python3
"""
安装Office文件处理依赖库
"""

import subprocess
import sys

def install_package(package):
    """安装Python包"""
    print(f"安装 {package}...")
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install", package, "--no-warn-script-location"])
        print(f"{package} 安装成功")
        return True
    except subprocess.CalledProcessError as e:
        print(f"{package} 安装失败: {e}")
        return False

def main():
    """主函数"""
    packages = [
        "openpyxl",      # 读取Excel文件
        "python-docx",   # 读取Word文件
        "python-pptx",   # 读取PowerPoint文件
        "PyPDF2",        # 读取PDF文件
    ]

    print("开始安装Office文件处理依赖库...")

    success_count = 0
    for package in packages:
        if install_package(package):
            success_count += 1

    print(f"\n安装完成: {success_count}/{len(packages)} 个包安装成功")

    if success_count == len(packages):
        print("所有依赖库安装成功！")
    else:
        print("部分依赖库安装失败，某些文件格式可能无法预览")

if __name__ == "__main__":
    main()
