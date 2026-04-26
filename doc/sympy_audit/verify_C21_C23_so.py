"""Verify shortlisted StackExchange candidates on SymPy 1.14.0.

Reproduces each user-reported failure as faithfully as possible from the
question text. Output goes to stdout; we then triage which survive on 1.14.0.

For each candidate:
  - print URL + title
  - run reproducer
  - compare to user-claimed expected vs SymPy 1.14.0 actual

Run from repo root:
    python3 doc/sympy_audit/verify_C21_C23_so.py
"""

from __future__ import annotations

import sys
import traceback

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
    except Exception as exc:  # pragma: no cover - we want to see them
        print(f"EXCEPTION: {type(exc).__name__}: {exc}")


def c_so16096164() -> None:
    section("SO#16096164 — solve gives incorrect answer (rational eq in p)")
    print("https://stackoverflow.com/questions/16096164")
    print("User: solve(num/denom = p) returns wrong/missing roots vs MATLAB.")

    p, WAA, WAa, Waa = sp.symbols("p WAA WAa Waa")
    num = p**2 * WAA + p * (1 - p) * WAa
    denom = p**2 * WAA + 2 * p * (1 - p) * WAa + (1 - p) ** 2 * Waa
    eq = sp.Eq(num / denom, p)
    case("solve full rational equation", lambda: print(sp.solve(eq, p)))
    case("solve numerator multiplied out (manual)",
         lambda: print(sp.solve(p * denom - num, p)))


def c_so69608662() -> None:
    section("SO#69608662 — linsolve produces wrong result")
    print("https://stackoverflow.com/questions/69608662")
    import numpy as np

    A = np.array([
        [1, 1, 0, 1, 1, -3, -1, 1],
        [0, 0, 0, -1, -1, 0, -1, -2],
        [0, 0, 0, 0, 0, 1, 1, 1],
    ])
    b = np.array([[0], [0], [0]])
    A_s = sp.Matrix(A)
    b_s = sp.Matrix(b)
    syms = list(sp.symbols("a b c d e f g h"))
    case("linsolve", lambda: print(sp.linsolve((A_s, b_s), syms)))
    case("solve", lambda: print(sp.solve(A_s @ sp.Matrix(syms) - b_s, syms)))


def c_so19072700() -> None:
    section("SO#19072700 — rref returns identity for singular symbolic matrix")
    print("https://stackoverflow.com/questions/19072700")
    nu = sp.Symbol("nu")
    lamb = sp.Symbol("lambda")
    A = sp.Matrix([
        [-3 * nu, 1, 0, 0],
        [3 * nu, -2 * nu - 1, 2, 0],
        [0, 2 * nu, -nu - lamb - 2, 3],
        [0, 0, nu + lamb, -3],
    ])
    case("det (should be 0)", lambda: print(sp.simplify(A.det())))
    case("rref", lambda: print(A.rref()))
    case("nullspace (should be non-empty)", lambda: print(A.nullspace()))


def c_so73988957() -> None:
    section("SO#73988957 — nonlinsolve gives extra solution")
    print("https://stackoverflow.com/questions/73988957")
    a, v_s, v_e, t_m, t_e, d_e = sp.symbols("a v_s v_e t_m t_e d_e")
    v_s = 15
    v_e = 20
    a = 1
    d_e = sp.Rational(212, 1) + sp.Rational(1, 2)  # 212.5
    v_m = v_s - t_m * a
    Eq1 = v_m + a * (t_e - t_m) - v_e
    Eq2 = t_e * v_m + (v_s - v_m) * t_m / 2 + (v_e - v_m) * (t_e - t_m) / 2 - d_e
    case("nonlinsolve", lambda: print(sp.nonlinsolve([Eq1, Eq2], [t_m, t_e])))
    case("solve", lambda: print(sp.solve([Eq1, Eq2], [t_m, t_e])))


def c_so79113424() -> None:
    section("SO#79113424 — limit of atan at -oo returns nan")
    print("https://stackoverflow.com/questions/79113424")
    x = sp.Symbol("x", real=True)
    expr = sp.atan((1 - x) / (x + 3))
    case("limit x -> -oo (expected -pi/4)",
         lambda: print(sp.limit(expr, x, -sp.oo)))
    case("limit x -> +oo (expected -pi/4)",
         lambda: print(sp.limit(expr, x, sp.oo)))


def c_so59995637() -> None:
    section("SO#59995637 — parametric limit returns wrong sign")
    print("https://stackoverflow.com/questions/59995637")
    x = sp.symbols("x", real=True)
    alpha = sp.symbols("alpha", real=True, positive=True, nonzero=True)
    expr = (x * sp.exp(x) - sp.exp(2 * sp.sqrt(1 + x**2))) / (
        sp.exp(alpha * x) + x**alpha
    )
    case("limit x -> oo with alpha > 0",
         lambda: print(sp.limit(expr, x, sp.oo)))


def c_so49817984() -> None:
    section("SO#49817984 — solve drops a LambertW branch")
    print("https://stackoverflow.com/questions/49817984")
    y, x = sp.symbols("y x")
    cdf = (-y - 1) * sp.exp(-y) + 1
    case("solve(cdf == x, y) — does it return both branches?",
         lambda: print(sp.solve(sp.Eq(cdf, x), y)))


def c_so33923802() -> None:
    section("SO#33923802 — sympy thinks real-only expression is complex")
    print("https://stackoverflow.com/questions/33923802")
    d, g, t = sp.symbols("Delta Gamma t", real=True)
    hbar = sp.symbols("hbar", positive=True, real=True)
    dg = sp.sqrt(d**2 + g**2)
    expr = sp.exp(sp.I * t * dg / hbar)
    case("Abs(exp(I*t*dg/hbar)) — should simplify to 1",
         lambda: print(sp.simplify(sp.Abs(expr))))


def c_so73598109() -> None:
    section("SO#73598109 — invert + subs gives x=0 (wrong)")
    print("https://stackoverflow.com/questions/73598109")
    x, y_prime = sp.symbols("x y_prime", positive=True)
    c = 10
    import math
    f = math.log(1 + c) - c / (1 + c)
    y = 1 / f * sp.log(1 + c * x) / x
    eqn = sp.Eq(y_prime, y)
    sol = sp.solve(eqn, x, rational=False)
    print("solutions:", sol)
    if sol:
        case("subs(y_prime, 1.5) on first solution",
             lambda: print(sol[0].subs(y_prime, 1.5)))


def c_so24062112() -> None:
    section("SO#24062112 — Abs(exp(I)) not simplified to 1")
    print("https://stackoverflow.com/questions/24062112")
    case("simplify(Abs(exp(I))) — expected 1",
         lambda: print(sp.simplify(sp.Abs(sp.exp(sp.I)))))


def c_so49607478() -> None:
    section("SO#49607478 — limit involving normal CDF gives wrong answer")
    print("https://stackoverflow.com/questions/49607478")
    x, y = sp.symbols("x y")
    from sympy.stats import Normal, cdf
    N = Normal("N", 0, 1)
    case("limit involving N(x) — see body for exact problem (skipped here)",
         lambda: print("user did not paste minimal repro; skipping"))


def c_so44099570() -> None:
    section("SO#44099570 — dsolve harmonic motion: out of scope (ODE)")
    print("https://stackoverflow.com/questions/44099570 — skipped (dsolve)")


def main() -> int:
    for fn in [
        c_so16096164,
        c_so69608662,
        c_so19072700,
        c_so73988957,
        c_so79113424,
        c_so59995637,
        c_so49817984,
        c_so33923802,
        c_so73598109,
        c_so24062112,
    ]:
        try:
            fn()
        except Exception:
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
