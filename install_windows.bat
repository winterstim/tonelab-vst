@echo off
color 0A
echo =========================================
echo       Tonelab VST Installer (Windows)      
echo =========================================
echo This script will install Tonelab to your VST3 folder
echo (C:\Program Files\Common Files\VST3)
echo.

:: Automatically check & get admin rights
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Requesting Administrator privileges...
    echo Set UAC = CreateObject^("Shell.Application"^) > "%temp%\getadmin.vbs"
    echo UAC.ShellExecute "%~s0", "", "", "runas", 1 >> "%temp%\getadmin.vbs"
    "%temp%\getadmin.vbs"
    del "%temp%\getadmin.vbs"
    exit /B
)

set "DEST=C:\Program Files\Common Files\VST3"
if not exist "%DEST%" mkdir "%DEST%"

set "SRC=%~dp0tonelab_vst.vst3"

if not exist "%SRC%" (
    color 0C
    echo Error: tonelab_vst.vst3 not found in this folder.
    echo Please extract the entire ZIP archive before running this script.
    echo.
    pause
    exit /B 1
)

echo Installing plugin to %DEST%...
:: Remove old plugin directory if it exists
if exist "%DEST%\tonelab_vst.vst3" rmdir /s /q "%DEST%\tonelab_vst.vst3"

:: Copy the folder over (use robocopy or xcopy)
xcopy /E /I /Y /Q "%SRC%" "%DEST%\tonelab_vst.vst3\" > nul

if %errorLevel% neq 0 (
    color 0C
    echo Error: Failed to copy the plugin.
    echo.
    pause
    exit /B 1
)

echo.
echo =========================================
echo  Installation complete! 
echo  You can now use Tonelab in your DAW.
echo =========================================
echo.
pause
