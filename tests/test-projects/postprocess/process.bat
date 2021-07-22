@ECHO OFF

rem Windows paths contain backslashes - we need to be escape them for JSON:
set bard=%1
set bard=%bard:\=\\%
set file=%4
set file=%file:\=\\%

(
echo {
echo "bard": "%bard%",
echo "file_name": "%2",
echo "file_stem": "%3",
echo "file": "%file%"
echo }
) > %5
