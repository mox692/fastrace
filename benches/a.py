# #!/usr/bin/env python3
# import sys
# import re

# def convert(stream):
#     # header_re = re.compile(r'^((?:\S+::)+\S+)')
#     header_re = re.compile(
#     r'^'                             # 行頭
#     r'((?:\S+::)+\S+)'               # 「トークン::トークン::…トークン」をキャプチャ
#     r'(?:\s+\S+:\S+\(\))?'           # （オプショナル）空白＋「ラベル:関数()」
#     )
#     # Matches:   MetricName:    12345|67890   (...)
#     metric_re = re.compile(r'^\s*([^:]+):\s*([\d,]+)\|[\d,]+\s*\(')

#     current_test = None


#     for line in stream:
#         # print(f"Processing line: {line.strip()}")

#         line = line.rstrip()
#         # Test header?
#         m = header_re.match(line)
#         if m:
#             current_test = m.group(1)
#             continue

#         # Metric line?
#         m = metric_re.match(line)
#         if m and current_test:
#             metric, value = m.groups()
#             # normalize metric into lowercase, hyphens->underscores
#             metric_key = metric.strip().lower().replace(' ', '_')
#             # remove any commas in the number
#             value = value.replace(',', '')
#             # default error to 0 (IAI doesn’t give a ±)
#             print(f"{metric_key} {current_test} ... bench: {value} (+/- 0)")

# if __name__ == "__main__":
#     if len(sys.argv) > 1:
#         with open(sys.argv[1], 'r') as f:
#             convert(f)
#     else:
#         convert(sys.stdin)



#!/usr/bin/env python3
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
