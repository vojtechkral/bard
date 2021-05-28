#!/bin/bash

echo "{
    \"file_name\": \"$1\",
    \"file_stem\": \"$2\",
    \"file\": \"$3\"
}" > "$4"
