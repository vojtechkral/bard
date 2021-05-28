@ECHO OFF

set file=%3
set file=%file:\=\\%

(
echo {
echo "file_name": "%1",
echo "file_stem": "%2",
echo "file": "%file%"
echo }
) > %4
