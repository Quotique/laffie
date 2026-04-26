"""Verify D13-D17 (ОДЗ M3 batch) on SymPy 1.14.0."""

from __future__ import annotations

import sympy as sp


def section(title: str, expected: str) -> None:
    print()
    print("=" * 78)
    print(title)
    print("=" * 78)
    print("Expected:")
    for line in expected.strip().splitlines():
        print("  " + line)
    print()


def case(label: str, fn) -> None:
    print(f"--- {label}")
    try:
        fn()
    except Exception as exc:
        print(f"EXCEPTION: {type(exc).__name__}: {exc}")
    print()


def d13() -> None:
    section("D13 — irrational: sqrt(x+3) - sqrt(x-1) = 1", """
    ОДЗ: x >= 1.
    Squaring: x = 13/4. Substitution: 5/2 - 3/2 = 1 ✓.
    Answer: x = 13/4.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(sp.sqrt(x + 3) - sp.sqrt(x - 1), 1)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d14() -> None:
    section("D14 — parametric irrational: sqrt(x-a)+sqrt(x+a) = sqrt(2x)", """
    ОДЗ: x >= |a|, x >= 0.
    Square once: 2x + 2 sqrt(x^2 - a^2) = 2x  →  x^2 - a^2 = 0  →  x = |a|.
    Check: at x = |a|, one root is 0, the other is sqrt(2|a|).
      a >= 0: x = a;
      a < 0:  x = -a.
    """)
    a, x = sp.symbols("a x", real=True)
    eq = sp.Eq(sp.sqrt(x - a) + sp.sqrt(x + a), sp.sqrt(2 * x))
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset", lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d15() -> None:
    section("D15 — log: log_2(x^2-5x+6) + log_2(x-1) = log_2(x-3) + 2", """
    ОДЗ: x^2-5x+6 > 0  ∧  x-1 > 0  ∧  x-3 > 0  →  x > 3.
    log_2((x-2)(x-3)(x-1)) = log_2(4(x-3))
    (x-3)·[(x-2)(x-1) - 4] = 0
      x = 3 (exluded by ОДЗ)
      x^2 - 3x - 2 = 0  →  x = (3 ± sqrt(17))/2
    Filter: only (3 + sqrt(17))/2 ≈ 3.56 satisfies x > 3.
    """)
    x = sp.Symbol("x", real=True, positive=True)
    eq = sp.Eq(sp.log(x**2 - 5*x + 6, 2) + sp.log(x - 1, 2),
               sp.log(x - 3, 2) + 2)
    case("solve (positive only)", lambda: print(sp.solve(eq, x)))
    # Also try with default symbol
    y = sp.Symbol("y")
    eq2 = sp.Eq(sp.log(y**2 - 5*y + 6, 2) + sp.log(y - 1, 2),
                sp.log(y - 3, 2) + 2)
    case("solve (no assumptions)", lambda: print(sp.solve(eq2, y)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq2, y, domain=sp.S.Reals)))


def d16() -> None:
    section("D16 — false root after cancellation: (x^2-9)/(x-3) = 6", """
    ОДЗ: x != 3.
    (x-3)(x+3)/(x-3) = 6  =>  x+3 = 6  =>  x = 3.
    But x = 3 fails ОДЗ. Answer: ∅.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq((x**2 - 9) / (x - 3), 6)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d17() -> None:
    section("D17 — variable-base log inequality: log_x(x^2 - 2x) > 1", """
    ОДЗ on log_x: x > 0, x != 1.
    ОДЗ on argument: x^2 - 2x > 0 → x < 0 or x > 2 → with x > 0: x > 2.
    For x > 1: log_x f > 1 ⟺ f > x → x^2 - 2x > x → x^2 - 3x > 0 → x > 3.
    For 0 < x < 1: ruled out by argument ОДЗ on (2, +∞).
    Answer: x > 3.
    """)
    x = sp.Symbol("x", real=True, positive=True)
    expr = sp.log(x**2 - 2*x) / sp.log(x)  # log_x(...) = ln(...)/ln(x)
    case("solveset on log_x f > 1",
         lambda: print(sp.solveset(expr > 1, x, domain=sp.Interval(0, sp.oo))))
    case("reduce_inequalities",
         lambda: print(sp.reduce_inequalities([expr > 1], [x])))


def main() -> int:
    for fn in (d13, d14, d15, d16, d17):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
