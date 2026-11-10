"""Find (amount, accumulated rate) pairs for each branch of the 128-bit division in iCKB Logic.

Usage (run with python3 -I):
  division_paths.py [samples]
  division_paths.py check <reach.rs>

`deposit_to_ickb` divides `amount * 10^16` by the accumulated rate. On RISC-V that division is
`u128_div_rem` of compiler_builtins 0.1.101, built from `impl_trifecta!` with 32-bit halves
(src/int/specialized_div_rem/trifecta.rs). `trifecta` below ports it line by line and returns the
branch that produced the result. The search samples amounts and rates, checks the port against
Python's own division on every sample, and prints the count and first example per branch.
The rows of `DIVISION_ROWS` in ../src/tests/fuzz/reach.rs come from such searches, with more samples
for the rare branches; `check` confirms that each row takes the branch its label names.
"""
import collections
import random
import re
import sys

M64 = (1 << 64) - 1
M128 = (1 << 128) - 1
GENESIS_AR = 10**16
CKB = 10**8


def lz(x):
    return 128 - x.bit_length()


def trifecta(duo, div):
    n, n_h = 64, 32
    assert div != 0
    div_lz, duo_lz = lz(div), lz(duo)
    if div_lz <= duo_lz:
        return (1, duo - div, "early1") if duo >= div else (0, duo, "early0")
    if duo_lz >= n:
        return duo // div, duo % div, "half"
    if div_lz >= n + n_h:
        # Three half divisions by a 32-bit divisor; no accumulated rate is that small.
        return duo // div, duo % div, "small-divisor"
    if div_lz - duo_lz < n_h:
        shift = n - duo_lz
        quo = (duo >> shift) // (div >> shift)
        tmp = quo * div
        if tmp > M128 or duo < tmp:
            return quo - 1, (duo + div - tmp) & M128, "A-corrected"
        return quo, duo - tmp, "A"
    quo = 0
    div_extra = (n + n_h) - div_lz
    div_sig_n_h_add1 = ((div >> div_extra) & 0xFFFFFFFF) + 1
    while True:
        duo_extra = n - duo_lz
        duo_sig_n = (duo >> duo_extra) & M64
        if div_extra <= duo_extra:
            quo_part = duo_sig_n // div_sig_n_h_add1
            extra_shl = duo_extra - div_extra
            quo += quo_part << extra_shl
            duo -= ((div * quo_part) & M128) << extra_shl
        else:
            shift = n - duo_lz
            quo_part = ((duo >> shift) & M64) // ((div >> shift) & M64)
            tmp = (quo_part * div) & M128
            if duo < tmp:
                return quo + quo_part - 1, (duo + div - tmp) & M128, "B-corrected"
            return quo + quo_part, duo - tmp, "B"
        duo_lz = lz(duo)
        if div_lz <= duo_lz:
            return (quo + 1, duo - div, "loop-early1") if div <= duo else (quo, duo, "loop-early0")
        if n <= duo_lz:
            return quo + duo // div, duo % div, "loop-half"


def check(path):
    with open(path) as f:
        rows = re.findall(r'^    \("([A-Za-z0-9-]+)", ([0-9_]+), ([0-9_]+)\),$', f.read(), re.M)
    assert rows, "no rows found"
    for label, amount, ar in rows:
        branch = trifecta(int(amount) * GENESIS_AR, int(ar))[2]
        assert branch == label, f"({amount}, {ar}) takes {branch}, not {label}"
    print(f"{len(rows)} rows take the branch their label names: {sorted({r[0] for r in rows})}")


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "check":
        return check(sys.argv[2])
    samples = int(sys.argv[1]) if len(sys.argv) > 1 else 300_000
    random.seed(1)
    # What a deposit within the bounds meets, what a receiptless pool deposit can hold, and anything.
    populations = {
        "deposit within bounds, rate up to double genesis": lambda: (random.randint(1_000 * CKB, 1_000_000 * CKB), random.randint(GENESIS_AR, 2 * GENESIS_AR)),
        "below the minimum deposit, rate up to double genesis": lambda: (random.randint(0, 1_000 * CKB), random.randint(GENESIS_AR, 2 * GENESIS_AR)),
        "any amount, any rate from genesis": lambda: (random.getrandbits(random.randint(1, 64)), random.choice([random.randint(GENESIS_AR, M64), GENESIS_AR + random.getrandbits(random.randint(1, 40))])),
    }
    for name, draw in populations.items():
        count, first = collections.Counter(), {}
        for _ in range(samples):
            amount, ar = draw()
            duo = amount * GENESIS_AR
            quo, rem, branch = trifecta(duo, ar)
            assert (quo, rem) == (duo // ar, duo % ar), (amount, ar, branch)
            count[branch] += 1
            first.setdefault(branch, (amount, ar))
        print(name)
        for branch in sorted(count):
            print(f"  {branch}: {count[branch]} of {samples}, first {first[branch]}")


main()
