@echo off
REM Copy designer_settings.json to release output directory after build
setlocal
set SRC=designer_settings.json
set DEST=target\release\designer_settings.json
if exist %SRC% copy /Y %SRC% %DEST%
endlocal
