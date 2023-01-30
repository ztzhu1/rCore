#!/bin/bash
cd $(dirname "$0")

app_list=$(find src/bin -maxdepth 1 -type f | sort)

for app_name in ${app_list}
do
    app_name=$(basename ${app_name%%.*})
    cargo build --bin ${app_name}
    echo -e "\033[1;33mbuilt ${app_name}\033[0m"
done