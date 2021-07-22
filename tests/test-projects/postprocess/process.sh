#!/bin/bash

echo "{
    \"bard\": \"$1\",
    \"file_name\": \"$2\",
    \"file_stem\": \"$3\",
    \"file\": \"$4\"
}" > "$5"
