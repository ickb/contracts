"""Report which instructions of the deployed iCKB binaries a test run executed.

Usage (run with python3 -I):
  coverage.py <release_dir> <symbols_dir> [--functions] [--runs] [--annotate <binary> <regex>] <trace files...>

<release_dir> holds the deployed binaries and <symbols_dir> their relinked copies with symbols
(<name>.sym), both as run.sh leaves them. A trace file has one "<program fnv64> <pc> <length>"
line per instruction the instrumented VM decoded, see README.md.

For each binary it prints the instructions executed, then the never-executed ones in three groups:
functions never entered, runs that end in a panic or an allocation failure, and the rest.
--functions lists every function not fully executed, --runs prints the rest group as disassembly,
--annotate prints the functions matching <regex> with "--" on each never-executed instruction.
"""
import collections
import re
import subprocess
import sys

BINARIES = ["ickb_logic", "limit_order", "owned_owner"]
# Calls that never return: a run that reaches one of these stops the script.
STOPS = re.compile(r"panic|unwrap_failed|expect_failed|handle_alloc_error|_fail|capacity_overflow|__rust_alloc_error_handler|rdl_oom|rt_error")
CALL = re.compile(r"(jalr|jal|call)\b")


def fnv64(data):
    h = 0xCBF29CE484222325
    for x in data:
        h = ((h ^ x) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def functions(path):
    """[(name, [(address, text)])] from the symbolized binary, local labels folded into their function."""
    out = subprocess.run(["llvm-objdump", "-d", "--no-show-raw-insn", "-C", path], capture_output=True, text=True, check=True).stdout
    funcs, cur = [], None
    for line in out.splitlines():
        m = re.match(r"^([0-9a-f]{16}) <(.*)>:$", line)
        if m:
            if not m.group(2).startswith(".L"):
                cur = (m.group(2), [])
                funcs.append(cur)
            continue
        m = re.match(r"^\s+([0-9a-f]+):\s+(.*)$", line)
        # `unimp` pads the end of a function after a call that never returns.
        if m and cur is not None and not m.group(2).startswith("unimp"):
            cur[1].append((int(m.group(1), 16), m.group(2).strip()))
    return funcs


def main():
    args = sys.argv[1:]
    release, symbols, args = args[0], args[1], args[2:]
    show_functions = "--functions" in args
    show_runs = "--runs" in args
    annotate = None
    if "--annotate" in args:
        i = args.index("--annotate")
        annotate = (args[i + 1], re.compile(args[i + 2]))
        del args[i:i + 3]
    traces = [a for a in args if not a.startswith("--")]

    ids = {}
    for name in BINARIES:
        with open(f"{release}/{name}", "rb") as f:
            ids[fnv64(f.read())] = name
    executed = collections.defaultdict(set)
    for path in traces:
        with open(path) as f:
            for line in f:
                p = line.split()
                # A process killed mid-write can leave one short line.
                if len(p) == 3 and int(p[0], 16) in ids:
                    pc = int(p[1], 16)
                    executed[ids[int(p[0], 16)]].update(range(pc, pc + int(p[2])))

    for name in BINARIES:
        funcs = functions(f"{symbols}/{name}.sym")
        cov = executed[name]
        where = {a: (fi, i) for fi, (_, ins) in enumerate(funcs) for i, (a, _) in enumerate(ins)}
        # Every traced address must be an instruction of this binary, or the trace is not its own.
        starts = set(where)
        stray = sum(1 for a in cov if a not in starts and not any(a - d in starts for d in (1, 2, 3)))
        assert stray == 0, f"{name}: {stray} traced addresses are not instructions of this binary"

        def stops(fi, i, depth=0):
            """Whether straight-line execution from here reaches a call that never returns."""
            ins = funcs[fi][1]
            while i < len(ins):
                text = ins[i][1]
                if CALL.match(text) and STOPS.search(text):
                    return True
                m = re.match(r"j\s+0x([0-9a-f]+)", text)
                if m:
                    target = int(m.group(1), 16)
                    return target in where and depth < 6 and stops(*where[target], depth + 1)
                if re.match(r"(ret|jr|jalr|jal|call|b[a-z]+)\b", text):
                    return False
                i += 1
            return False

        total = sum(len(ins) for _, ins in funcs)
        hit = sum(1 for _, ins in funcs for a, _ in ins if a in cov)
        groups = collections.Counter()
        rest = []
        for fi, (fname, ins) in enumerate(funcs):
            if not any(a in cov for a, _ in ins):
                groups["in functions never entered"] += len(ins)
                continue
            i = 0
            while i < len(ins):
                if ins[i][0] in cov:
                    i += 1
                    continue
                # One straight-line piece: up to the next executed instruction or transfer of control.
                j = i
                while j + 1 < len(ins) and ins[j + 1][0] not in cov and not re.match(r"(j|ret|jr)\b", ins[j][1]) and not (CALL.match(ins[j][1]) and STOPS.search(ins[j][1])):
                    j += 1
                if stops(fi, i):
                    groups["on paths that end in a panic or an allocation failure"] += j - i + 1
                else:
                    groups["in the rest"] += j - i + 1
                    rest.append((fname, ins[i:j + 1]))
                i = j + 1
        print(f"{name}: {hit} of {total} instructions executed ({100 * hit / total:.1f}%), {sum(1 for _, ins in funcs if any(a in cov for a, _ in ins))} of {len(funcs)} functions entered")
        for group in ["in functions never entered", "on paths that end in a panic or an allocation failure", "in the rest"]:
            print(f"  never executed {group}: {groups[group]}")
        if show_functions:
            for fname, ins in funcs:
                h = sum(1 for a, _ in ins if a in cov)
                if h < len(ins):
                    print(f"  {h:5d}/{len(ins):5d} {fname[:140]}")
        if show_runs:
            last = None
            for fname, run in rest:
                if fname != last:
                    print(f"\n  ### {fname[:140]}")
                    last = fname
                for a, text in run:
                    print(f"     {a:x}: " + re.sub(r"<([^>]{60})[^>]*>", r"<\1..>", text))
                print("     .")
        if annotate and annotate[0] == name:
            for fname, ins in funcs:
                if annotate[1].search(fname):
                    print(f"\n  ### {fname[:140]}")
                    for a, text in ins:
                        print(("     " if a in cov else "  -- ") + f"{a:x}: " + re.sub(r"<([^>]{60})[^>]*>", r"<\1..>", text))


main()
