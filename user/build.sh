#!/bin/bash
app_start=0x80400000
app_size_limit=0x20000
cd $(dirname "$0")

app_list=$(find src/bin -maxdepth 1 -type f | sort)

i=0
for app_name in ${app_list}
do
    app_name=$(basename ${app_name%%.*})
    base_addr=$((${app_start} + ${app_size_limit} * i))
    base_addr=$(printf "0x%08x" ${base_addr})
    sed -i "1,5s/0x[0-9a-f]*\?/${base_addr}/" src/linker.ld
    cargo build --bin ${app_name}
    echo -e "\033[1;33mbuilt ${app_name}\033[0m"
    i=$((i + 1))
done

sed -i "1,5s/0x[0-9a-f]*\?/${app_start}/" src/linker.ld