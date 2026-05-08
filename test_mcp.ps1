# 测试 MCP 服务器启动
Write-Host "测试 MCP Fetch 服务器..." -ForegroundColor Cyan

# 启动 npx
$process = Start-Process -FilePath "npx" -ArgumentList "-y", "@modelcontextprotocol/server-fetch" -NoNewWindow -PassThru -RedirectStandardInput "input.txt" -RedirectStandardOutput "output.txt" -RedirectStandardError "error.txt"

# 创建初始化请求
$initRequest = @{
    jsonrpc = "2.0"
    id = 0
    method = "initialize"
    params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{
            name = "test"
            version = "1.0"
        }
    }
} | ConvertTo-Json -Compress

# 写入请求
$initRequest | Out-File -FilePath "input.txt" -Encoding UTF8

# 等待响应
Start-Sleep -Seconds 3

# 读取输出
Write-Host "`n=== STDOUT ===" -ForegroundColor Green
if (Test-Path "output.txt") {
    Get-Content "output.txt"
} else {
    Write-Host "无输出文件" -ForegroundColor Red
}

Write-Host "`n=== STDERR ===" -ForegroundColor Yellow
if (Test-Path "error.txt") {
    Get-Content "error.txt"
} else {
    Write-Host "无错误文件" -ForegroundColor Red
}

# 清理
Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Remove-Item "input.txt", "output.txt", "error.txt" -ErrorAction SilentlyContinue

Write-Host "`n测试完成" -ForegroundColor Cyan
