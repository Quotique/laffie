"""Verify D21-D22 (system M5 batch) on SymPy 1.14.0."""

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


def d21() -> None:
    section("D21 — symmetric system: {x+y+xy = 11, x^2 y + x y^2 = 30}", """
    Let s = x+y, p = xy.
    Eq1: s + p = 11.   Eq2: p · s = 30.
    s, p are roots of t^2 - 11 t + 30 = 0  →  t = 5 or t = 6.
    Case A: s=5, p=6  →  {x,y} = {2,3}.
    Case B: s=6, p=5  →  {x,y} = {1,5}.
    Answer: (x,y) ∈ {(2,3), (3,2), (1,5), (5,1)}.
    """)
    x, y = sp.symbols("x y")
    eqs = [x + y + x*y - 11, x**2*y + x*y**2 - 30]
    case("solve", lambda: print(sp.solve(eqs, [x, y])))
    case("nonlinsolve", lambda: print(sp.nonlinsolve(eqs, [x, y])))


def d22() -> None:
    section("D22 — homogeneous: {x^2 - 3xy + 2y^2 = 0, x^2 + y^2 = 10}", """
    First eq factors:  (x - y)(x - 2y) = 0  →  x = y  or  x = 2y.
    Case A (x = y):  2y^2 = 10  →  y = ±sqrt(5),   x = ±sqrt(5).
    Case B (x = 2y): 5y^2 = 10  →  y = ±sqrt(2),   x = ±2 sqrt(2).
    Answer: (±sqrt(5), ±sqrt(5)), (±2 sqrt(2), ±sqrt(2)).
    """)
    x, y = sp.symbols("x y")
    eqs = [x**2 - 3*x*y + 2*y**2, x**2 + y**2 - 10]
    case("solve", lambda: print(sp.solve(eqs, [x, y])))
    case("nonlinsolve", lambda: print(sp.nonlinsolve(eqs, [x, y])))


def main() -> int:
    for fn in (d21, d22):
        try:
            fn()
        except Exception:
            import traceback
            traceback.print_exc()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
