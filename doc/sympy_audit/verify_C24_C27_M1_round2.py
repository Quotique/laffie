"""Re-verify D4 (without positivity), and probe D3/D5 meta-question failure modes."""

from __future__ import annotations

import sympy as sp


def section(title: str) -> None:
    print()
    print("=" * 78)
    print(title)
    print("=" * 78)


def case(label: str, fn) -> None:
    print(f"\n--- {label}")
    try:
        fn()
    except Exception as exc:
        print(f"EXCEPTION: {type(exc).__name__}: {exc}")


# D4 — without positivity constraint
def d4_unconstrained() -> None:
    section("D4 (unconstrained) — log_a(x^2-1) = log_a(2x+2)")
    a, x = sp.symbols("a x")
    eq = sp.Eq(sp.log(x**2 - 1, a), sp.log(2 * x + 2, a))
    case("solve, no assumptions", lambda: print(sp.solve(eq, x)))
    case("solve with x real", lambda: print(sp.solve(eq.subs(a, 2), sp.Symbol("x", real=True))))

    # Reformulate equation using ln/ln form to bypass log_a parsing if needed
    case("solve with concrete a=2",
         lambda: print(sp.solve(sp.Eq(sp.log(x**2 - 1, 2), sp.log(2 * x + 2, 2)), x)))
    case("solve dropping the log entirely (algebraic core)",
         lambda: print(sp.solve(sp.Eq(x**2 - 1, 2 * x + 2), x)))


def d3_deeper() -> None:
    section("D3 deeper — count of roots of |x^2-4x+3| = a vs a")
    x = sp.Symbol("x", real=True)
    a = sp.Symbol("a", real=True)
    print("Test by enumerating: how many real roots for each a in {-1, 0, 0.5, 1, 2}?")
    for av in (-1, 0, sp.Rational(1, 2), 1, 2):
        eq = sp.Eq(sp.Abs(x**2 - 4 * x + 3), av)
        sol = sp.solveset(eq, x, domain=sp.S.Reals)
        print(f"  a = {av}:  {sol}")
    print()
    print("Trying solveset with symbolic a:")
    case("solveset", lambda: print(
        sp.solveset(sp.Eq(sp.Abs(x**2 - 4 * x + 3), a), x, domain=sp.S.Reals)))
    print()
    print("Counting solutions as function of a — no public sympy API.")
    print("Conclusion: this is R6 (no mode for 'count of solutions vs param').")


def d5_deeper() -> None:
    section("D5 deeper — meta condition for solvability of system")
    a, x, y = sp.symbols("a x y", real=True)
    sols = sp.solve([x + y - a, x**2 + y**2 - a], [x, y])
    print("Symbolic solutions:", sols)
    print()
    print("Sympy emits sqrt(-a(a-2)) — implicit condition a*(a-2) <= 0 for real.")
    print("But there is no API that surfaces 'for which a is the system solvable?'.")
    print("Direct attempt:")
    case("solveset on the discriminant",
         lambda: print(sp.solveset(-a * (a - 2) >= 0, a, domain=sp.S.Reals)))
    print()
    print("Conclusion: condition a in [0,2] requires manual extraction of the")
    print("discriminant; no API for 'parameter set such that real roots exist'.")
    print("Same R6 flavor as D3.")


def d6_deeper() -> None:
    section("D6 deeper — sin x + cos x = a on [0, pi]")
    a, x = sp.symbols("a x", real=True)
    print("solve drops period and gives Weierstrass-substituted answer:")
    case("solve", lambda: print(sp.solve(sp.sin(x) + sp.cos(x) - a, x)))
    case("solveset",
         lambda: print(sp.solveset(sp.sin(x) + sp.cos(x) - a, x, domain=sp.S.Reals)))
    print()
    print("solveset on a restricted interval — direct phrasing:")
    case("solveset on [0, pi]",
         lambda: print(sp.solveset(sp.sin(x) + sp.cos(x) - a, x,
                                   domain=sp.Interval(0, sp.pi))))
    print()
    print("min/max on [0, pi] — these work correctly:")
    case("minimum",
         lambda: print(sp.minimum(sp.sin(x) + sp.cos(x), x, sp.Interval(0, sp.pi))))
    case("maximum",
         lambda: print(sp.maximum(sp.sin(x) + sp.cos(x), x, sp.Interval(0, sp.pi))))


def d7_deeper() -> None:
    section("D7 deeper — biquadratic, focus on Intersection unevaluated")
    a, x = sp.symbols("a x")
    case("solveset over Reals",
         lambda: print(sp.solveset(x**4 - (a + 1) * x**2 + a, x, domain=sp.S.Reals)))
    print()
    print("Manual case-split for concrete signs of a:")
    case("a = -1",
         lambda: print(sp.solveset((x**4 - (-1 + 1) * x**2 + -1).subs(a, -1), x,
                                   domain=sp.S.Reals)))
    case("a = 1/4",
         lambda: print(sp.solveset(x**4 - (sp.Rational(1, 4) + 1) * x**2 + sp.Rational(1, 4),
                                   x, domain=sp.S.Reals)))
    case("a = 4",
         lambda: print(sp.solveset(x**4 - (4 + 1) * x**2 + 4, x, domain=sp.S.Reals)))
    print()
    print("So sympy can solve each concrete a, but cannot express the")
    print("piecewise structure parametrically.")


def main() -> int:
    d4_unconstrained()
    d3_deeper()
    d5_deeper()
    d6_deeper()
    d7_deeper()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
