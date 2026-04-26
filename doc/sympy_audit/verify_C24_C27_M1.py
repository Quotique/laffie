"""Verify D1-D7 (parametric M1 family) on SymPy 1.14.0.

For each candidate:
  - print expected mathematical answer (worked out by hand)
  - run minimal reproducer
  - flag whether SymPy on 1.14.0 fails as the catalog claimed,
    silently produces incomplete output, or actually handles it.

Run:
    python3 doc/sympy_audit/verify_C24_C27_M1.py
"""

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


# ============================================================================
# D1.  (a^2 - 4) x = a - 2     (linear param)
# ============================================================================
def d1() -> None:
    section("D1 — parametric linear: (a^2 - 4) x = a - 2", """
    (a-2)(a+2) x = a-2
      a = 2:   0 = 0    →  x ∈ ℝ
      a = -2:  0 = -4   →  ∅
      else:    x = 1/(a+2)
    """)
    a, x = sp.symbols("a x")
    eq = sp.Eq((a**2 - 4) * x, a - 2)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset", lambda: print(sp.solveset(eq, x)))


# ============================================================================
# D2.  a x^2 + (a-1) x - 1 > 0   (parametric quadratic inequality)
# ============================================================================
def d2() -> None:
    section("D2 — parametric quadratic inequality", """
    Roots of a x^2 + (a-1) x - 1: factors as (a x - 1)(x + 1).
    So roots x = 1/a, x = -1 (when a != 0).
      a > 0:  x < -1 or x > 1/a
      a = 0:  -x - 1 > 0  →  x < -1
      a < 0:
        if 1/a < -1 (i.e. -1 < a < 0):  1/a < x < -1
        if 1/a = -1 (a = -1):           ∅ (parabola touches zero)
        if 1/a > -1 (a < -1):           -1 < x < 1/a
    """)
    a, x = sp.symbols("a x", real=True)
    expr = a * x**2 + (a - 1) * x - 1
    case("reduce_inequalities", lambda: print(sp.reduce_inequalities([expr > 0], [x])))
    case("solveset > 0",
         lambda: print(sp.solveset(expr > 0, x, domain=sp.S.Reals)))


# ============================================================================
# D3.  for which a does |x^2 - 4x + 3| = a have exactly 3 roots?
# ============================================================================
def d3() -> None:
    section("D3 — meta question: |x^2-4x+3| = a, exactly 3 roots", """
    f(x) = (x-1)(x-3), zeros at x=1,3, vertex at x=2 with f(2) = -1.
    |f(x)| = a:
      a < 0:  no roots
      a = 0:  2 roots (x=1, x=3)
      0 < a < 1:  4 roots
      a = 1:  3 roots (x=2 double from |f|=1 lower branch + x=2±sqrt(2))
      a > 1:  2 roots
    Answer: a = 1.
    """)
    a, x = sp.symbols("a x", real=True)
    eq = sp.Eq(sp.Abs(x**2 - 4 * x + 3), a)
    case("solve over x for symbolic a", lambda: print(sp.solve(eq, x)))
    print("  No SymPy API takes this meta-question directly. The closest is")
    print("  to count solutions per a — has to be done manually.")


# ============================================================================
# D4.  log_a(x^2 - 1) = log_a(2x + 2)   (log with parameter in base)
# ============================================================================
def d4() -> None:
    section("D4 — log with parameter in base: log_a(x^2-1) = log_a(2x+2)", """
    Domain on a:  a > 0, a != 1.
    Domain on x:  x^2 - 1 > 0  ∧  2x + 2 > 0
                  (x > 1 or x < -1)  ∧  x > -1
                  →  x > 1
    Equation: x^2 - 1 = 2x + 2  →  x^2 - 2x - 3 = 0  →  x = 3 or x = -1
    Filtered by domain: x = 3.
    """)
    a, x = sp.symbols("a x", positive=True, real=True)
    eq = sp.Eq(sp.log(x**2 - 1, a), sp.log(2 * x + 2, a))
    case("solve", lambda: print(sp.solve(eq, x)))
    print("  ОДЗ check: do roots satisfy x > 1 ?")
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


# ============================================================================
# D5.  {x + y = a, x^2 + y^2 = a}: for which a does it have a solution?
# ============================================================================
def d5() -> None:
    section("D5 — system: when does {x+y=a, x^2+y^2=a} have a solution?", """
    Substitute y = a - x:  x^2 + (a-x)^2 = a  →  2x^2 - 2ax + a^2 - a = 0
    Discriminant = 4a^2 - 8(a^2 - a) = -4a^2 + 8a = 4a(2 - a).
    Real solutions iff a(2 - a) >= 0 iff 0 <= a <= 2.
    """)
    a, x, y = sp.symbols("a x y", real=True)
    case("solve system",
         lambda: print(sp.solve([x + y - a, x**2 + y**2 - a], [x, y])))
    print("  After symbolic solve, do the formulas indicate the condition 0<=a<=2?")
    print("  (sympy will normally just return roots with sqrt(...) without flagging)")


# ============================================================================
# D6.  sin x + cos x = a, x in [0, pi]: for which a does it have a solution?
# ============================================================================
def d6() -> None:
    section("D6 — for which a is sin x + cos x = a solvable on [0, π]?", """
    sin x + cos x = sqrt(2) sin(x + π/4).
    On [0, π], x + π/4 ∈ [π/4, 5π/4],
    sin(.) ranges over [sin(5π/4), sin(π/2)] = [-sqrt(2)/2, 1].
    So sqrt(2)·sin(.) ranges over [-1, sqrt(2)].
    Solvable iff -1 <= a <= sqrt(2).
    """)
    a, x = sp.symbols("a x", real=True)
    case("solve sin(x)+cos(x) = a for x",
         lambda: print(sp.solve(sp.sin(x) + sp.cos(x) - a, x)))
    case("range of sin(x)+cos(x) on [0, pi] (manual: minimum/maximum)",
         lambda: print((sp.minimum(sp.sin(x) + sp.cos(x), x, sp.Interval(0, sp.pi)),
                        sp.maximum(sp.sin(x) + sp.cos(x), x, sp.Interval(0, sp.pi)))))


# ============================================================================
# D7.  x^4 - (a+1) x^2 + a = 0   (biquadratic with parameter)
# ============================================================================
def d7() -> None:
    section("D7 — biquadratic with parameter: x^4 - (a+1)x^2 + a = 0", """
    Let y = x^2. y^2 - (a+1) y + a = 0  →  (y - 1)(y - a) = 0  →  y = 1 or y = a.
      a < 0:  x = ±1 only (y = a has no real x)
      a = 0:  x = -1, 0, 1
      0 < a < 1 or a > 1:  x = ±1, ±sqrt(a)
      a = 1:  x = ±1 (double)
    Failure mode if any: x = ±sqrt(a) is only real when a >= 0; SymPy may
    return all four formal roots without the case-split.
    """)
    a, x = sp.symbols("a x")
    case("solve", lambda: print(sp.solve(x**4 - (a + 1) * x**2 + a, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(x**4 - (a + 1) * x**2 + a, x, domain=sp.S.Reals)))
    print("  Note: M5 = factorization. Manual factor: (x^2-1)(x^2-a) = 0.")
    case("factor", lambda: print(sp.factor(x**4 - (a + 1) * x**2 + a)))


def main() -> int:
    for fn in (d1, d2, d3, d4, d5, d6, d7):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
