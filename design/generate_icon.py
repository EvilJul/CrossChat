#!/usr/bin/env python3
"""
CrossChat 图标生成脚本
自动将 SVG 转换为 PNG 并生成所有需要的图标尺寸
"""

import os
import sys
import subprocess
from pathlib import Path

def check_dependencies():
    """检查必要的依赖"""
    print("🔍 检查依赖...")
    
    # 检查 Python PIL/Pillow
    try:
        from PIL import Image
        print("✅ Pillow 已安装")
        return True
    except ImportError:
        print("❌ 缺少 Pillow 库")
        print("\n请安装 Pillow:")
        print("  pip install Pillow")
        return False

def svg_to_png_pillow(svg_path, png_path, size):
    """使用 Pillow 转换 SVG 为 PNG（需要 cairosvg）"""
    try:
        import cairosvg
        from PIL import Image
        import io
        
        # 使用 cairosvg 转换
        png_data = cairosvg.svg2png(
            url=str(svg_path),
            output_width=size,
            output_height=size
        )
        
        # 使用 Pillow 保存
        image = Image.open(io.BytesIO(png_data))
        image.save(png_path, 'PNG')
        return True
    except ImportError:
        return False
    except Exception as e:
        print(f"❌ 转换失败: {e}")
        return False

def create_base_png():
    """创建基础 1024x1024 PNG 图标"""
    print("\n📐 生成基础图标 (1024x1024)...")
    
    svg_path = Path("design/app-icon.svg")
    png_path = Path("design/icon-1024.png")
    
    if not svg_path.exists():
        print(f"❌ 找不到 SVG 文件: {svg_path}")
        return False
    
    # 尝试使用 cairosvg
    if svg_to_png_pillow(svg_path, png_path, 1024):
        print(f"✅ 已生成: {png_path}")
        return True
    
    # 如果失败，提示用户手动转换
    print("\n⚠️  自动转换失败，请手动转换:")
    print(f"1. 访问: https://svgtopng.com/")
    print(f"2. 上传: {svg_path}")
    print(f"3. 设置尺寸: 1024 x 1024")
    print(f"4. 下载并保存为: {png_path}")
    print("\n完成后按 Enter 继续...")
    input()
    
    if png_path.exists():
        print("✅ 检测到 PNG 文件")
        return True
    else:
        print("❌ 未找到 PNG 文件")
        return False

def generate_icons():
    """使用 Tauri CLI 生成所有图标"""
    print("\n🎨 使用 Tauri 生成所有尺寸的图标...")
    
    png_path = Path("design/icon-1024.png")
    
    if not png_path.exists():
        print(f"❌ 找不到基础 PNG: {png_path}")
        return False
    
    try:
        # 运行 cargo tauri icon
        result = subprocess.run(
            ["cargo", "tauri", "icon", str(png_path)],
            capture_output=True,
            text=True,
            check=True
        )
        
        print("✅ 图标生成成功!")
        print(result.stdout)
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ 生成失败: {e}")
        print(e.stderr)
        return False
    except FileNotFoundError:
        print("❌ 找不到 cargo tauri 命令")
        print("\n请安装 Tauri CLI:")
        print("  cargo install tauri-cli")
        return False

def main():
    """主函数"""
    print("=" * 60)
    print("🎨 CrossChat 图标生成器")
    print("=" * 60)
    
    # 检查依赖
    if not check_dependencies():
        print("\n💡 提示: 如果无法安装依赖，可以手动转换 SVG")
        print("   查看 design/QUICK_START.md 了解详情")
        sys.exit(1)
    
    # 创建基础 PNG
    if not create_base_png():
        print("\n❌ 无法创建基础 PNG 图标")
        sys.exit(1)
    
    # 生成所有图标
    if not generate_icons():
        print("\n❌ 无法生成图标文件")
        sys.exit(1)
    
    print("\n" + "=" * 60)
    print("✅ 图标生成完成!")
    print("=" * 60)
    print("\n📋 下一步:")
    print("1. 重新编译应用: npm run tauri build")
    print("2. 安装新版本应用")
    print("3. 查看新图标效果")
    print("\n💡 提示: 开发模式下图标可能不会更新，需要编译后安装")

if __name__ == "__main__":
    main()
