#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Install Office file processing dependencies
"""

import subprocess
import sys

# Force UTF-8 encoding for stdout/stderr (fix Windows CI encoding issues)
if sys.stdout.encoding != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
if sys.stderr.encoding != 'utf-8':
    sys.stderr.reconfigure(encoding='utf-8')

def install_package(package):
    """Install Python package"""
    print(f"Installing {package}...")
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install", package, "--no-warn-script-location"])
        print(f"{package} installed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"{package} installation failed: {e}")
        return False

def main():
    """Main function"""
    packages = [
        "openpyxl",      # Read Excel files
        "python-docx",   # Read Word files
        "python-pptx",   # Read PowerPoint files
        "PyPDF2",        # Read PDF files
    ]

    print("Starting Office file processing dependencies installation...")

    success_count = 0
    for package in packages:
        if install_package(package):
            success_count += 1

    print(f"\nInstallation complete: {success_count}/{len(packages)} packages installed successfully")

    if success_count == len(packages):
        print("All dependencies installed successfully!")
    else:
        print("Some dependencies failed to install, certain file formats may not be previewable")

if __name__ == "__main__":
    main()
