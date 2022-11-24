#!/usr/bin/env python3
import sys
import json
import argparse

import matplotlib.pyplot as plt


parser = argparse.ArgumentParser()
parser.add_argument("-T", "--threshold", type=float)
args = parser.parse_args()

output = json.loads(sys.stdin.read())

fields = output["fields"]
combined = output["combined_field"]

plt.title("Density")
plt.xlabel("Line of code")
plt.ylabel("Suspiciousness")

combined_control = [0 for _ in range(len(combined["x"]))]
for functionality in fields:
    blob = fields[functionality]
    x = blob["x"]
    y = blob["y"]
    for idx, vy in enumerate(y):
        combined_control[idx] += vy
    plt.plot(x, y, label=functionality, linestyle="dashed")

plt.plot(combined["x"], combined["y"], label="Combined")
plt.plot(combined["x"], combined_control, label="Combined (in Python)")

if args.threshold:
    # draw line at threshold
    t = [float(args.threshold) for _ in combined["x"]]
    plt.plot(combined["x"], t, label="Threshold")

plt.legend()
plt.show()