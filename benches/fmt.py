#!/usr/bin/env python3

# A script to convert the output from `iai-callgrind` into the one
# that can be interpreted by `benchmark-action/github-action-benchmark`.

import sys
import re

header_re = re.compile(
    r'^'
    r'((?:\S+::)+\S+)'
    r'(?:\s+\S+:\S+\(\))?'
    r'$'
)
metric_re = re.compile(
    r'^\s*'
    r'([^:]+):'
    r'\s*([\d,]+)\|'
    r'(?:N/A|\d[\d,]*)'
    r'\s*\('
)

def convert(stream):
    current_test = None
    for line in stream:
        line = line.rstrip()

        m = header_re.match(line)
        if m:
            current_test = m.group(1)
            continue

        m = metric_re.match(line)
        if m and current_test:
            metric, value = m.groups()
            key = metric.strip().lower().replace(' ', '_')
            value = value.replace(',', '')
            print(f"test {current_test}____{key} ... bench: {value} count/iter (+/- 0)")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        with open(sys.argv[1], 'r') as f:
            convert(f)
    else:
        convert(sys.stdin)
