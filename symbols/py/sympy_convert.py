"""Conversion utilities between Laffie Term trees and sympy expressions.

Usage from .sym files:
    exec(open('symbols/py/sympy_convert.py').read())
    # now `to_sympy(node, x_sym)` and `from_sympy(expr, var_name, Term)` are available
"""

import sympy


def to_sympy(node, var_map):
    """Convert a Laffie Term tree to a sympy expression.

    Args:
        node: Laffie Term node.
        var_map: dict mapping variable name (str) -> sympy.Symbol.
                 Variables not in the map are created as new symbols.
    """
    if node.is_number:
        v = node.value
        return sympy.Integer(v) if isinstance(v, int) else sympy.Rational(v).limit_denominator()

    if node.is_variable:
        return var_map.get(node.name, sympy.Symbol(node.name))

    if node.is_param:
        return sympy.Symbol(node.name)

    if not node.is_symbol:
        return None

    s = node.symbol
    args = [to_sympy(c, var_map) for c in node.children]
    if None in args:
        return None

    if s == '+':
        return sympy.Add(*args)
    if s == '*':
        return sympy.Mul(*args)
    if s == '^':
        return sympy.Pow(args[0], args[1])
    if s == '/':
        return args[0] / args[1]
    if s == 'neg' and len(args) == 1:
        return -args[0]

    return None


def from_sympy(expr, var_name, Term):
    """Convert a sympy expression back to a Laffie Term tree.

    Args:
        expr: sympy expression.
        var_name: name of the main variable (str).
        Term: the Laffie Term class (injected in calculator context).
    """
    if isinstance(expr, sympy.Integer):
        return Term.number(int(expr))

    if isinstance(expr, sympy.Rational) and not isinstance(expr, sympy.Integer):
        return Term('/', [Term.number(int(expr.p)), Term.number(int(expr.q))])

    if isinstance(expr, sympy.Symbol):
        return Term.variable(str(expr))

    if isinstance(expr, sympy.Add):
        children = [from_sympy(a, var_name, Term) for a in expr.args]
        if None in children:
            return None
        return Term('+', children)

    if isinstance(expr, sympy.Mul):
        children = [from_sympy(a, var_name, Term) for a in expr.args]
        if None in children:
            return None
        return Term('*', children)

    if isinstance(expr, sympy.Pow):
        base, exp = expr.args
        return Term('^', [from_sympy(base, var_name, Term),
                          from_sympy(exp, var_name, Term)])

    # Negative number (sympy represents -3 as Mul(-1, 3) sometimes, but
    # also as Integer(-3) which is handled above)
    if expr.is_number:
        if expr.is_integer:
            return Term.number(int(expr))
        if expr.is_rational:
            return Term('/', [Term.number(int(expr.p)), Term.number(int(expr.q))])

    return None
