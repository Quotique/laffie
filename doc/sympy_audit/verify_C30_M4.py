"""Verify D8-D12 (trig M4 batch) on SymPy 1.14.0."""

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


def d8() -> None:
    section("D8 — sin 3x = sin x", """
    sin 3x − sin x = 2 cos(2x) sin(x) = 0.
    Series:  x = π k          (sin x = 0)
             x = π/4 + π k / 2 (cos 2x = 0)
    Two distinct series.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(sp.sin(3*x), sp.sin(x))
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d9() -> None:
    section("D9 — sin x · cos 2x = 0", """
    Series:  x = π k          (sin x = 0)
             x = π/4 + π k / 2 (cos 2x = 0)
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.sin(x) * sp.cos(2*x)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d10() -> None:
    section("D10 — tg x + ctg x = 2", """
    1/(sin x cos x) = 2  →  sin(2x) = 1  →  x = π/4 + π k.
    ОДЗ: sin x ≠ 0, cos x ≠ 0; satisfied at every π/4 + π k.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(sp.tan(x) + sp.cot(x), 2)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d11() -> None:
    section("D11 — ЕГЭ-13 representative: 2 sin^2 x + sin 2x = 0 on [-π, π]", """
    2 sin x (sin x + cos x) = 0.
    sin x = 0  →  x ∈ {-π, 0, π}.
    sin x + cos x = 0  →  tan x = -1  →  x = -π/4 + π k  →  on [-π, π]: {-π/4, 3π/4}.
    Answer: {-π, -π/4, 0, 3π/4, π}.
    """)
    x = sp.Symbol("x", real=True)
    expr = 2*sp.sin(x)**2 + sp.sin(2*x)
    case("solve", lambda: print(sp.solve(expr, x)))
    case("solveset on [-pi, pi]",
         lambda: print(sp.solveset(expr, x, domain=sp.Interval(-sp.pi, sp.pi))))


def d12() -> None:
    section("D12 — (1 + cos x)(1 − tg(x/2)) = 0  — series loss via ОДЗ", """
    Factor 1: 1 + cos x = 0  →  cos x = -1  →  x = π + 2π k.
    Factor 2: 1 - tg(x/2) = 0  →  x/2 = π/4 + π k  →  x = π/2 + 2π k.
    But factor 2 has ОДЗ:  cos(x/2) ≠ 0  →  x ≠ π + 2π k.
    The series x = π + 2π k от factor 1 совпадает с дырой ОДЗ factor 2.
    So at x = π + 2π k:  expression = 0 · undefined = undefined → not a solution
    in the strict reading. Answer: x = π/2 + 2π k only.
    A laxer reading accepts x = π + 2π k since one factor is exactly 0.
    The sub-question: does sympy detect this nuance, or just return both series?
    """)
    x = sp.Symbol("x", real=True)
    expr = (1 + sp.cos(x)) * (1 - sp.tan(x/2))
    case("solve", lambda: print(sp.solve(expr, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(expr, x, domain=sp.S.Reals)))
    print()
    print("And after rewriting with simplify():")
    case("solve(simplify(expr))",
         lambda: print(sp.solve(sp.simplify(expr), x)))


def main() -> int:
    for fn in (d8, d9, d10, d11, d12):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
