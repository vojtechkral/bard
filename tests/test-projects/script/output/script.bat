@ECHO OFF

rem Windows paths contain backslashes - we need to be escape them for JSON:
set BARD=%BARD:\=\\%
set OUTPUT=%OUTPUT:\=\\%
set PROJECT_DIR=%PROJECT_DIR:\=\\%
set OUTPUT_DIR=%OUTPUT_DIR:\=\\%

(
echo BARD = "%BARD%"
echo OUTPUT = "%OUTPUT%"
echo PROJECT_DIR = "%PROJECT_DIR%"
echo OUTPUT_DIR = "%OUTPUT_DIR%"
) > "%OUTPUT_STEM%.toml"
