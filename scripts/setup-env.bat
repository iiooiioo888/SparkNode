@echo off
REM SparkNode 環境配置啟動器（雙擊或 cmd 執行）
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-env.ps1" %*
if errorlevel 1 pause
