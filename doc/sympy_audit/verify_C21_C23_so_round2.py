"""Round 2 of StackExchange candidate verification on SymPy 1.14.0.

Goals:
  (a) Deepen the 3 promising candidates from round 1.
  (b) Probe a few candidates that round 1 dismissed too quickly.
"""

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


# ===== Round 1 deep dives =====


def deep_so19072700() -> None:
    section("SO#19072700 deep — rref drops parameter cases")
    nu = sp.Symbol("nu")
    lamb = sp.Symbol("lambda")
    A = sp.Matrix([
        [-3 * nu, 1, 0, 0],
        [3 * nu, -2 * nu - 1, 2, 0],
        [0, 2 * nu, -nu - lamb - 2, 3],
        [0, 0, nu + lamb, -3],
    ])
    case("rank() — should be < 4 generically and even smaller at degenerate parameters",
         lambda: print("rank =", A.rank()))
    case("rref includes 1/(nu**2*(lambda+nu)) etc — divides by these factors silently",
         lambda: print(A.rref()[0]))
    print()
    print("Substitute nu=0 BEFORE rref — what is rank?")
    A_at_0 = A.subs(nu, 0)
    print("A|nu=0 =", A_at_0.tolist())
    case("rank|nu=0", lambda: print(A_at_0.rank()))
    case("rref|nu=0", lambda: print(A_at_0.rref()))
    print()
    print("Substitute lambda=-nu BEFORE rref — what is rank?")
    A_at_lnu = A.subs(lamb, -nu)
    case("rank|lambda=-nu", lambda: print(A_at_lnu.rank()))
    case("rref|lambda=-nu", lambda: print(A_at_lnu.rref()))


def deep_so49817984() -> None:
    section("SO#49817984 deep — LambertW branches")
    y, x = sp.symbols("y x")
    cdf = (-y - 1) * sp.exp(-y) + 1
    sol = sp.solve(sp.Eq(cdf, x), y)
    print("solve returned:", sol)
    print()
    # Numerical check at x = 0.5: there should be two real preimages because
    # the original CDF is not monotonic on the full line (it is monotonic on
    # [-1, +oo), but the inverse via LambertW touches both branches W0 and W-1).
    # cdf(y) - x = 0 для x=0.5 — численная картина.
    import numpy as np
    ys = np.linspace(-3, 5, 9)
    for yv in ys:
        print(f"  cdf({yv:.2f}) = {(-yv - 1) * np.exp(-yv) + 1:.6f}")
    print()
    print("At x=0.95 the equation has two real roots — does sympy return both?")
    case("solve numerically with check", lambda: print([sp.nsolve(sp.Eq(cdf, 0.95), y, y0) for y0 in (-2.0, 0.5)]))
    print()
    print("LambertW branches: W(-1, z) is the secondary real branch")
    case("explicit second branch", lambda: print(sp.LambertW(-0.05 * sp.exp(-1), -1)))
    case("does sympy accept second branch in solve?",
         lambda: print(sp.solveset(sp.Eq(cdf, x), y, domain=sp.S.Reals)))


def deep_so59995637() -> None:
    section("SO#59995637 deep — parametric limit")
    x = sp.symbols("x", real=True)
    alpha = sp.symbols("alpha", real=True, positive=True, nonzero=True)
    expr = (x * sp.exp(x) - sp.exp(2 * sp.sqrt(1 + x**2))) / (
        sp.exp(alpha * x) + x**alpha
    )
    case("default limit", lambda: print(sp.limit(expr, x, sp.oo)))
    case("with alpha=2 (concrete)",
         lambda: print(sp.limit(expr.subs(alpha, 2), x, sp.oo)))
    case("with alpha=1/2",
         lambda: print(sp.limit(expr.subs(alpha, sp.Rational(1, 2)), x, sp.oo)))
    case("with alpha=3",
         lambda: print(sp.limit(expr.subs(alpha, 3), x, sp.oo)))
    print()
    print("Expected analysis:")
    print("  numerator: x*e^x - e^(2*sqrt(1+x^2)) ~ -e^(2x) for x→+∞")
    print("  denominator: e^(α x) + x^α ~ e^(α x) for x→+∞")
    print("  so ratio ~ -e^((2-α)x)")
    print("  limit = -∞ if α<2, -1 if α=2, 0 if α>2")
    print("  parametric — needs case-split on α.")


# ===== Round 2 fresh candidates =====


def c_so16794745() -> None:
    section("SO#16794745 — Freudenstein-type trig mixed-angle equation")
    print("https://stackoverflow.com/questions/16794745")
    x = sp.Symbol("x")
    fi = sp.Symbol("fi", real=True)
    k1, k2, k3 = sp.symbols("k1 k2 k3", real=True)
    eq = k1 * sp.cos(x) - k2 * sp.cos(fi) + k3 - sp.cos(x - fi)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset on Reals", lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def c_so76280419() -> None:
    section("SO#76280419 — integrate (x+b)^t, x — special cases t=0,1")
    print("https://stackoverflow.com/questions/76280419")
    x, b, t = sp.symbols("x b t")
    case("integrate", lambda: print(sp.integrate((x + b) ** t, x)))


def c_so77895974() -> None:
    section("SO#77895974 — cubic solve returns trig-Cardano with imaginary units")
    print("https://stackoverflow.com/questions/77895974")
    x = sp.Symbol("x")
    f = x**3 - 3 * x + 1
    case("solve", lambda: print(sp.solve(f)))
    case("solve, simplify each", lambda: print([sp.nsimplify(sp.simplify(r)) for r in sp.solve(f)]))


def c_so78986747() -> None:
    section("SO#78986747 — solve(x**2, x) misinterpretation")
    print("(skipped: user issued solve with wrong args; not a sympy bug)")


def c_so31070921() -> None:
    section("SO#31070921 — sympy mixed with numpy floats — out of scope?")
    print("https://stackoverflow.com/questions/31070921")
    print("Contains sin/cos with float arg via numpy; SymPy correctness vs float repr.")
    print("(typical float-vs-symbolic confusion — out of taxonomy)")


def main() -> int:
    deep_so19072700()
    deep_so49817984()
    deep_so59995637()
    c_so16794745()
    c_so76280419()
    c_so77895974()
    c_so78986747()
    c_so31070921()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
