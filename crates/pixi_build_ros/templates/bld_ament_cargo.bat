@echo off
setlocal DisableDelayedExpansion

set "source_dir=@SOURCE_DIR@"
set "ros_package_name=@ROS_PACKAGE_NAME@"
set "runtime_state=%BUILD_PREFIX%\.pixi-build-ros\ament-cargo"
set "cargo_home=%runtime_state%\cargo-home"
set "staging_dir=%runtime_state%\staging"
set "target_dir=%runtime_state%\target"
set "cargo=%BUILD_PREFIX%\Library\bin\cargo.exe"
set "rustc=%BUILD_PREFIX%\Library\bin\rustc.exe"
set "rustdoc=%BUILD_PREFIX%\Library\bin\rustdoc.exe"
set "output_root=%LIBRARY_PREFIX%"

if exist "%runtime_state%" rmdir /s /q "%runtime_state%"
if exist "%runtime_state%" goto :failure
if exist "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%" del /q "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%" || goto :failure
if exist "%output_root%\share\%ros_package_name%\package.xml" del /q "%output_root%\share\%ros_package_name%\package.xml" || goto :failure
if not exist "%output_root%\lib\%ros_package_name%\" mkdir "%output_root%\lib\%ros_package_name%" || goto :failure
if not exist "%output_root%\lib\%ros_package_name%\" goto :failure
if exist "%output_root%\lib\%ros_package_name%\*.exe" del /q "%output_root%\lib\%ros_package_name%\*.exe" || goto :failure
if exist "%output_root%\lib\%ros_package_name%\*.exe" goto :failure
mkdir "%cargo_home%" || goto :failure
if not exist "%cargo_home%\" goto :failure
mkdir "%staging_dir%" || goto :failure
if not exist "%staging_dir%\" goto :failure
mkdir "%target_dir%" || goto :failure
if not exist "%target_dir%\" goto :failure
set "CARGO_HOME=%cargo_home%"
set "CARGO=%cargo%"
set "RUSTC=%rustc%"
set "RUSTDOC=%rustdoc%"

"%cargo%" install --root "%staging_dir%" --path "%source_dir%" --target-dir "%target_dir%" --no-track --force
if errorlevel 1 goto :failure

for %%B in ("%staging_dir%\bin\*.exe") do if exist "%%~fB" copy /y "%%~fB" "%output_root%\lib\%ros_package_name%\%%~nxB" >nul || goto :failure
if not exist "%output_root%\share\ament_index\resource_index\packages\" mkdir "%output_root%\share\ament_index\resource_index\packages" || goto :failure
if not exist "%output_root%\share\ament_index\resource_index\packages\" goto :failure
if not exist "%output_root%\share\%ros_package_name%\" mkdir "%output_root%\share\%ros_package_name%" || goto :failure
if not exist "%output_root%\share\%ros_package_name%\" goto :failure
type nul > "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%" || goto :failure
copy /y "%source_dir%\package.xml" "%output_root%\share\%ros_package_name%\package.xml" >nul || goto :failure

for %%B in ("%staging_dir%\bin\*.exe") do if exist "%%~fB" for %%P in ("%output_root%\lib\%ros_package_name%\%%~nxB") do >> "%RATTLER_BUILD_PACKAGE_FILES%" echo(%%~fP
if errorlevel 1 goto :failure
for %%P in ("%output_root%\share\ament_index\resource_index\packages\%ros_package_name%") do >> "%RATTLER_BUILD_PACKAGE_FILES%" echo(%%~fP
if errorlevel 1 goto :failure
for %%P in ("%output_root%\share\%ros_package_name%\package.xml") do >> "%RATTLER_BUILD_PACKAGE_FILES%" echo(%%~fP
if errorlevel 1 goto :failure

rmdir /s /q "%runtime_state%"
if exist "%runtime_state%" goto :failure
exit /b 0

:failure
if exist "%output_root%\lib\%ros_package_name%" del /q "%output_root%\lib\%ros_package_name%\*.exe"
if exist "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%" del /q "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%"
if exist "%output_root%\share\%ros_package_name%\package.xml" del /q "%output_root%\share\%ros_package_name%\package.xml"
if exist "%runtime_state%" rmdir /s /q "%runtime_state%"
if exist "%output_root%\lib\%ros_package_name%\*.exe" exit /b 1
if exist "%output_root%\share\ament_index\resource_index\packages\%ros_package_name%" exit /b 1
if exist "%output_root%\share\%ros_package_name%\package.xml" exit /b 1
if exist "%runtime_state%" exit /b 1
exit /b 1
