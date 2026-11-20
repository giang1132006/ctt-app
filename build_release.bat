@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

cd /d "%~dp0"

set "SKIP_TESTS=0"
set "NO_PAUSE=0"

:parse_args
if "%~1"=="" goto args_done
if /I "%~1"=="--skip-tests" set "SKIP_TESTS=1"
if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
shift
goto parse_args
:args_done

echo ============================================================================
echo   CTT Project - Windows Release Build ^& Packaging Script
echo ============================================================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Khong tim thay 'cargo' trong PATH. Hay cai dat Rustup truoc khi build.
    goto fail
)

if "%SKIP_TESTS%"=="1" goto skip_tests
echo [1/4] Dang chay kiem tra unit ^& integration tests...
cargo test --all-targets
if errorlevel 1 (
    echo [ERROR] Tests that bai! Dung dong goi release.
    goto fail
)
echo.
goto after_tests

:skip_tests
echo [1/4] Bo qua tests ^(--skip-tests^).
echo.

:after_tests
echo [2/4] Dang bien dich ban Release toi uu ^(ctt-app.exe^)...
cargo build --release
if errorlevel 1 (
    echo [ERROR] Bien dich release that bai!
    goto fail
)
echo.

set "EXE_PATH="
for /f "usebackq delims=" %%T in (`cargo metadata --format-version 1 --no-deps 2^>nul ^| powershell -NoProfile -Command "$input | Out-String | ConvertFrom-Json | Select-Object -ExpandProperty target_directory"`) do (
    if exist "%%~T\release\ctt-app.exe" set "EXE_PATH=%%~T\release\ctt-app.exe"
)

if not defined EXE_PATH if defined CARGO_TARGET_DIR (
    if exist "%CARGO_TARGET_DIR%\release\ctt-app.exe" set "EXE_PATH=%CARGO_TARGET_DIR%\release\ctt-app.exe"
)
if not defined EXE_PATH if exist "%~dp0target\release\ctt-app.exe" (
    set "EXE_PATH=%~dp0target\release\ctt-app.exe"
)
if not defined EXE_PATH if exist "%USERPROFILE%\.cargo-targets\ctt-app\release\ctt-app.exe" (
    set "EXE_PATH=%USERPROFILE%\.cargo-targets\ctt-app\release\ctt-app.exe"
)

if not defined EXE_PATH (
    echo [ERROR] Khong tim thay file ctt-app.exe sau khi build release.
    goto fail
)

echo [3/4] Dang dong goi thu muc portable dist\ctt-app\ ...
set "DIST_ROOT=%~dp0dist"
set "PKG_DIR=%DIST_ROOT%\ctt-app"
set "ZIP_PATH=%DIST_ROOT%\ctt-app-windows-x64.zip"

if exist "%PKG_DIR%" rmdir /s /q "%PKG_DIR%"
if exist "%ZIP_PATH%" del /f /q "%ZIP_PATH%"
mkdir "%PKG_DIR%"
mkdir "%PKG_DIR%\profiles"

copy /y "%EXE_PATH%" "%PKG_DIR%\ctt-app.exe" >nul
if errorlevel 1 (
    echo [ERROR] Khong the copy ctt-app.exe vao %PKG_DIR%.
    goto fail
)

(
echo CTT Project - Huong dan su dung nhanh ^(Ban Portable Windows x64^)
echo ================================================================
echo.
echo 1. Khoi chay ung dung:
echo    - Chay truc tiep file ctt-app.exe ^(khong can cai dat^).
echo    - Neu ban dung macro/bind phim trong game hoac ung dung chay quyen Administrator,
echo      hay bam chuot phai vao ctt-app.exe -^> chon "Run as administrator".
echo.
echo 2. Phim tat dung khan cap ^(Emergency Stop^):
echo    - Mac dinh: Ctrl + Shift + F12 hoac bam nut "Stop All" tren thanh trang thai.
echo    - Khi bam Emergency Stop, toan bo phim/chuot dang giu ao va qua trinh ghi/phat
echo      macro se duoc giai phong va dung ngay lap tuc.
echo.
echo 3. Quy trinh tao Macro:
echo    - Vao tab "Tong quan" hoac "Binding" de tao it nhat 1 Profile.
echo    - Chuyen sang tab "Macros" -^> bam "+ Tao Macro" de mo cua so Timeline da track.
echo    - Them phim / chuot / khoang trong / vung lap I-O -^> dat ten va bam "Luu Macro".
echo.
echo 4. Luu y bao mat:
echo    - Phan mem chi hoat dong voi cua so giao dien hien thi ro trang thai hoat dong.
) > "%PKG_DIR%\HUONG_DAN_SU_DUNG.txt"
if errorlevel 1 (
    echo [ERROR] Khong the tao file HUONG_DAN_SU_DUNG.txt.
    goto fail
)

echo [4/4] Dang nen goi phan phoi ZIP (%ZIP_PATH%)...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Compress-Archive -Path '%PKG_DIR%\*' -DestinationPath '%ZIP_PATH%' -Force"
if errorlevel 1 (
    echo [ERROR] Nen file ZIP that bai!
    goto fail
)

echo.
echo ============================================================================
echo   DONG GOI THANH CONG!
echo ============================================================================
for %%F in ("%PKG_DIR%\ctt-app.exe") do echo   Executable : %%~fF (%%~zF bytes)
for %%Z in ("%ZIP_PATH%") do echo   Archive    : %%~fZ (%%~zZ bytes)
echo   Thu muc    : %PKG_DIR%
echo.
echo Ban co the gui file ZIP hoac thu muc dist\ctt-app\ cho nguoi dung giai nen va chay ctt-app.exe.
echo ============================================================================

if "%NO_PAUSE%"=="0" pause
exit /b 0

:fail
echo.
echo [FAILED] Qua trinh build/dong goi gap loi.
if "%NO_PAUSE%"=="0" pause
exit /b 1
