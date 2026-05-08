@echo off
echo 设置Python沙盒环境...
python "%~dp0setup_python.py"
if errorlevel 1 (
    echo Python环境设置失败
    pause
    exit /b 1
)
echo Python环境设置完成！
pause
