#!/usr/bin/env python3
import sys
import re

def convert(stream):
    # Matches: <test-name> <short|long>:<iter>
    header_re = re.compile(r'^((?:\S+::)+\S+)')
    # Matches:   MetricName:    12345|67890   (...)
    metric_re = re.compile(r'^\s*([^:]+):\s*([\d,]+)\|[\d,]+\s*\(')

    current_test = None


    for line in stream:
        # print(f"Processing line: {line.strip()}")

        line = line.rstrip()
        # Test header?
        m = header_re.match(line)
        if m:
            current_test = m.group(1)
            continue

        # Metric line?
        m = metric_re.match(line)
        if m and current_test:
            metric, value = m.groups()
            # normalize metric into lowercase, hyphens->underscores
            metric_key = metric.strip().lower().replace(' ', '_')
            # remove any commas in the number
            value = value.replace(',', '')
            # default error to 0 (IAI doesn’t give a ±)
            print(f"{metric_key} {current_test} ... bench: {value} (+/- 0)")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        with open(sys.argv[1], 'r') as f:
            convert(f)
    else:
        convert(sys.stdin)
