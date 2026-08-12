#!/usr/bin/env python3
"""Diffs the two harnesses' output: estimates must agree, timings get tabulated.

Invoked by compare.sh; not meant to be run directly. Both harnesses print the
same line format on purpose, so one parser serves both:

    --- hll ---
    lg_config_k=12 target=Hll8 items=200000 reps=2
    distinct   200000 items  5.52 ns/op  min 5.52  max 5.63  181.3 M/s  reps=2 estimate=200342

Only the scenario lines are read. The parameter header is skipped rather than
compared, because the two sides legitimately word it differently (HLL prints a
target type, the others do not) -- the estimate is the check that they really
ran the same workload, and it is a far stronger one than matching a header
string would be.
"""

import sys

# The bottom rung of the harnesses' --ladder. Below it HLL is still in its
# coupon list and CPC is still changing flavour, so the timings are not a
# steady-state cost. Keep in step with bench_common.h's ladder().
LADDER_FLOOR = 1_000_000


def parse(path):
    """Returns {(family, items, scenario): (ns_per_op, estimate)}."""
    rows = {}
    family = None
    with open(path) as f:
        for line in f:
            fields = line.split()
            if line.startswith("--- "):
                family = fields[1]
            elif len(fields) >= 3 and fields[2] == "items" and family:
                estimate = next(
                    x.split("=", 1)[1] for x in fields if x.startswith("estimate=")
                )
                key = (family, int(fields[1]), fields[0])
                rows[key] = (float(fields[3]), float(estimate))
    return rows


def main():
    cpp, rust = parse(sys.argv[1]), parse(sys.argv[2])

    # A key present on one side only means the harnesses have drifted apart --
    # a scenario added without its counterpart, or a ladder rung on one side.
    # Report it rather than silently comparing the intersection.
    failures = [
        f"{'/'.join(map(str, key))}: only the {side} harness ran it"
        for keys, side in ((cpp.keys() - rust.keys(), "C++"), (rust.keys() - cpp.keys(), "Rust"))
        for key in sorted(keys)
    ]

    print(
        f"{'family':<17} {'items':>12} {'scenario':<9} {'C++':>8} {'Rust':>8} "
        f"{'overhead':>9} {'ratio':>6}"
    )
    for key in sorted(cpp.keys() & rust.keys()):
        family, items, scenario = key
        cpp_ns, cpp_estimate = cpp[key]
        rust_ns, rust_estimate = rust[key]
        if cpp_estimate != rust_estimate:
            failures.append(
                f"{family}/{items}/{scenario}: C++ estimated {cpp_estimate:.0f} but "
                f"Rust estimated {rust_estimate:.0f}"
            )
        print(
            f"{family:<17} {items:>12} {scenario:<9} {cpp_ns:>8.2f} {rust_ns:>8.2f} "
            f"{rust_ns - cpp_ns:>+9.2f} {rust_ns / cpp_ns:>5.2f}x"
        )

    # The table is on stdout and the diagnosis on stderr; without this the
    # error appears above the table it refers to whenever stdout is a pipe.
    sys.stdout.flush()

    if failures:
        print(
            "\nerror: the two harnesses did not run the same workload, so the "
            "overhead column above is meaningless:",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print(
            "\nBoth sides feed identical keys through identical hashing, so their "
            "estimates must match exactly. Check that lg_k, the key spaces and the "
            "scenario set agree -- see the 'Keeping them in sync' section of "
            "benches/cpp_reference/README.md.",
            file=sys.stderr,
        )
        sys.exit(1)

    print("\nEstimates agree on every scenario: both sides ran the same workload.")
    print("Prefer quoting the overhead column over the ratio -- see AGENTS.md.")

    # Below the ladder's floor the timings are noise, and it shows: negative
    # overheads, meaning Rust "beat" the C++ it calls into. The parity check
    # above says nothing about that -- it proves the workloads match, not that
    # the timings are worth reading -- so say so rather than let a smoke run be
    # mistaken for a measurement.
    if min(items for _, items, _ in cpp) < LADDER_FLOOR:
        print(
            f"\nnote: ran below {LADDER_FLOOR:,} items. Treat the timings as a smoke "
            "check only;\nre-run with --ladder before quoting any of them.",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
