@echo off
chcp 65001 >nul
echo ============================================================
echo 🎨 CrossChat 图标生成器
echo ============================================================
echo.

REM 检查是否存在 icon-1024.png
if exist "design\icon-1024.png" (
    echo ✅ 找到基础图标: design\icon-1024.png
    goto :generate
)

echo ⚠️  未找到基础 PNG 图标
echo.
echo 📋 请按照以下步骤操作:
echo.
echo 1. 打开浏览器访问: https://svgtopng.com/
echo 2. 点击上传，选择: design\app-icon.svg
echo 3. 设置尺寸为: 1024 x 1024
echo 4. 下载 PNG 文件
echo 5. 将下载的文件重命名为: icon-1024.png
echo 6. 将文件移动到: design\ 目录
echo.
echo 完成后按任意键继续...
pause >nul

if not exist "design\icon-1024.png" (
    echo.
    echo ❌ 仍然找不到 icon-1024.png
    echo 请确保文件在 design\ 目录中
    pause
    exit /b 1
)

:generate
echo.
echo 🎨 开始生成所有尺寸的图标...
echo.

cargo tauri icon design\icon-1024.png

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ============================================================
    echo ✅ 图标生成成功!
    echo ============================================================
    echo.
    echo 📋 下一步:
    echo 1. 重新编译应用: npm run tauri build
    echo 2. 安装新版本应用
    echo 3. 查看新图标效果
    echo.
    echo 💡 提示: 开发模式 ^(npm run tauri dev^) 下图标不会更新
    echo          需要编译后安装才能看到新图标
) else (
    echo.
    echo ❌ 图标生成失败
    echo.
    echo 可能的原因:
    echo 1. 未安装 Tauri CLI
    echo    解决: cargo install tauri-cli
    echo.
    echo 2. PNG 文件格式不正确
    echo    解决: 重新转换 SVG
)

echo.
pause
