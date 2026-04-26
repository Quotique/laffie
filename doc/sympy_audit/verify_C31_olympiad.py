"""Verify D23-D29 (olympiad and Демидович batch) on SymPy 1.14.0."""

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


def d23() -> None:
    section("D23 — nested radical: x = sqrt(2 - sqrt(2 + x))", """
    ОДЗ: 0 <= x <= sqrt(2).
    Squaring twice: x^4 - 4 x^2 - x + 2 = 0.
    Numerical: only one real root in ОДЗ: x ≈ 0.62.
    All other roots from squaring are extraneous.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(x, sp.sqrt(2 - sp.sqrt(2 + x)))
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))
    # Polynomial form for reference
    print("After squaring twice (used as oracle):")
    case("solveset on x^4 - 4x^2 - x + 2 = 0 (Reals)",
         lambda: print(sp.solveset(x**4 - 4*x**2 - x + 2, x, domain=sp.S.Reals)))


def d24() -> None:
    section("D24 — cyclotomic Phi_14(x) = 0 in Reals", """
    Phi_14(x) = x^6 - x^5 + x^4 - x^3 + x^2 - x + 1.
    Its roots are primitive 14th roots of unity — all complex.
    Real roots: none.  Answer: ∅.
    """)
    x = sp.Symbol("x", real=True)
    expr = x**6 - x**5 + x**4 - x**3 + x**2 - x + 1
    case("solve", lambda: print(sp.solve(expr, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(expr, x, domain=sp.S.Reals)))
    print("(complex roots)")
    case("solveset over Complexes",
         lambda: print(sp.solveset(expr, x, domain=sp.S.Complexes)))


def d25() -> None:
    section("D25 — prove gcd(21n + 4, 14n + 3) = 1 for all n in N", """
    Euclid:
      21n+4 = 1·(14n+3) + (7n+1)
      14n+3 = 2·(7n+1) + 1
      7n+1  = (7n+1)·1 + 0
    So gcd = 1 for every integer n.
    """)
    n = sp.Symbol("n", integer=True)
    case("igcd(21*n+4, 14*n+3) for symbolic n",
         lambda: print(sp.igcd(21*n + 4, 14*n + 3)))
    case("gcd(21*n+4, 14*n+3) symbolic",
         lambda: print(sp.gcd(21*n + 4, 14*n + 3)))
    print("Numerical sanity check at n=0,1,...,5:")
    for nv in range(6):
        print(f"  n={nv}: gcd({21*nv+4}, {14*nv+3}) = {sp.igcd(21*nv+4, 14*nv+3)}")
    print()
    print("No public sympy API to PROVE this for all n. R6 (no prove mode).")


def d26() -> None:
    section("D26 — system {x+y+z = 6, x^3+y^3+z^3 = 3 x y z} in Reals", """
    Identity: x^3+y^3+z^3 - 3xyz = (x+y+z)·(...).
    Either x+y+z = 0 (excluded by Eq1), or x = y = z.
    With x+y+z = 6 → x = y = z = 2.
    """)
    x, y, z = sp.symbols("x y z", real=True)
    eqs = [x + y + z - 6, x**3 + y**3 + z**3 - 3*x*y*z]
    case("solve", lambda: print(sp.solve(eqs, [x, y, z])))
    case("nonlinsolve", lambda: print(sp.nonlinsolve(eqs, [x, y, z])))


def d27() -> None:
    section("D27 — sign of sqrt(x^2-4)/(x-2)", """
    ОДЗ: x <= -2 or x > 2.
    For x > 2: sqrt(x^2-4)/(x-2) = sqrt(x+2)/sqrt(x-2) > 0.
    For x < -2: numerator > 0, denominator < 0  →  expression < 0.
    Sign-aware simplification needed.  No single closed-form simplification.
    """)
    x = sp.Symbol("x", real=True)
    expr = sp.sqrt(x**2 - 4) / (x - 2)
    case("simplify(expr)", lambda: print(sp.simplify(expr)))
    case("simplify with assumption x > 2",
         lambda: print(sp.simplify(expr.subs(x, sp.Symbol("x", positive=True) + 3))))
    print("Sign in branches:")
    for xv in (-3, -2, -1, 0, 2.1, 3, 5):
        try:
            v = expr.subs(x, xv)
            v = sp.nsimplify(v) if v.is_number else v
            print(f"  x = {xv}: expr = {v}")
        except Exception as exc:
            print(f"  x = {xv}: {exc}")


def d28() -> None:
    section("D28 — irrational inequality: sqrt(x + 2) > x", """
    ОДЗ: x >= -2.
    For x < 0: LHS >= 0 > x ✓.
    For x >= 0: square gives x^2 - x - 2 < 0 → -1 < x < 2 → 0 <= x < 2.
    Total: -2 <= x < 2.
    """)
    x = sp.Symbol("x", real=True)
    expr = sp.sqrt(x + 2) - x
    case("solveset(sqrt(x+2) > x, S.Reals)",
         lambda: print(sp.solveset(sp.sqrt(x + 2) > x, x, domain=sp.S.Reals)))
    case("reduce_inequalities([sqrt(x+2) > x], [x])",
         lambda: print(sp.reduce_inequalities([sp.sqrt(x + 2) > x], [x])))


def d29() -> None:
    section("D29 — method of intervals with hole: (x^2-5x+6)/(x^2-4) >= 0", """
    Factor: (x-2)(x-3) / ((x-2)(x+2)).
    Cancels to (x-3)/(x+2), with hole at x = 2 (excluded by ОДЗ).
    (x-3)/(x+2) >= 0  →  x < -2 or x >= 3.
    Hole x=2 falls in the negative interval, so excluding it changes nothing
    in this case.
    Answer: (-∞, -2) ∪ [3, +∞).
    """)
    x = sp.Symbol("x", real=True)
    expr = (x**2 - 5*x + 6) / (x**2 - 4)
    case("solveset(expr >= 0, S.Reals)",
         lambda: print(sp.solveset(expr >= 0, x, domain=sp.S.Reals)))
    case("reduce_inequalities([expr >= 0], [x])",
         lambda: print(sp.reduce_inequalities([expr >= 0], [x])))


def main() -> int:
    for fn in (d23, d24, d25, d26, d27, d28, d29):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
