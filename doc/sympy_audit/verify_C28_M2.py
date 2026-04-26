"""Verify D18-D20 (absolute-value M2 family) on SymPy 1.14.0."""

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


def d18() -> None:
    section("D18 — двойной модуль: |x-1| + |x+2| = 5", """
    Геометрически: сумма расстояний x до 1 и x до -2 равна 5.
    Расстояние между точками = 3, сумма 5 > 3 — два решения снаружи отрезка.
      x >= 1:    2x+1 = 5  →  x = 2
      x <= -2:  -2x-1 = 5  →  x = -3
      -2<x<1:    3      ≠ 5  →  ∅
    Answer: x ∈ {-3, 2}.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(sp.Abs(x - 1) + sp.Abs(x + 2), 5)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))
    case("reduce_abs_inequalities (relational form)",
         lambda: print(sp.reduce_abs_inequality(sp.Abs(x - 1) + sp.Abs(x + 2) - 5, '==', x)))


def d19() -> None:
    section("D19 — вложенный модуль: ||x-2| - 3| = 1", """
    Let y = |x-2|. |y - 3| = 1 → y = 2 or y = 4.
      |x-2| = 2:  x = 0 or x = 4
      |x-2| = 4:  x = -2 or x = 6
    Answer: x ∈ {-2, 0, 4, 6}.
    """)
    x = sp.Symbol("x", real=True)
    eq = sp.Eq(sp.Abs(sp.Abs(x - 2) - 3), 1)
    case("solve", lambda: print(sp.solve(eq, x)))
    case("solveset over Reals",
         lambda: print(sp.solveset(eq, x, domain=sp.S.Reals)))


def d20() -> None:
    section("D20 — meta: при каких a |x-1| + |x+1| < a имеет решения?", """
    f(x) = |x-1| + |x+1|.
    f(x) >= 2 везде (минимум 2 достигается на [-1, 1]).
    Неравенство f(x) < a имеет решение iff a > 2.
    Answer: a > 2.
    """)
    a, x = sp.symbols("a x", real=True)
    f = sp.Abs(x - 1) + sp.Abs(x + 1)
    case("reduce_inequalities([f < a], [x])",
         lambda: print(sp.reduce_inequalities([f < a], [x])))
    case("solveset(f < a, x, S.Reals)",
         lambda: print(sp.solveset(f < a, x, domain=sp.S.Reals)))
    case("minimum of f on Reals",
         lambda: print(sp.minimum(f, x)))
    print("  As in C27/D5: condition on parameter requires manual extraction")
    print("  via minimum().  No API for 'for which a is f(x) < a satisfiable'.")


def main() -> int:
    for fn in (d18, d19, d20):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
