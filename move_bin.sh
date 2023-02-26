#!/bin/bash

src=(../bin/*)
dst=(user/src/bin/*)
# Something in `dst` but not in `src`,
# so I use two `dst` to make all files
# in `dst` appear at least twice.

all=(${src[@]} ${dst[@]} ${dst[@]})
uniq_files=$(echo ${all[@]} | tr -s ' ' '\n' | sed -E "s/.*\/(.*?)/\1/" | sort | uniq -u)
for f in $uniq_files
do
    mv ../bin/$f user/src/bin
done
