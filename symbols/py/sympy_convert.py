"""Conversion utilities between Laffie Term trees and sympy expressions.

This file is embedded into the binary at compile time and injected into the
Python environment as `SYMPY_CONVERT_SRC`. Usage from .sym files:
    exec(SYMPY_CONVERT_SRC)
    # now `to_sympy(node, x_sym)` and `from_sympy(expr, var_name, Term)` are available
"""

import sympy


def to_sympy(node, var_map, kinds=None):
    """Convert a Laffie Term tree to a sympy expression.

    Args:
        node: Laffie Term node.
        var_map: dict mapping variable name (str) -> sympy.Symbol.
                 Variables not in the map are created as new symbols.
        kinds: optional dict, populated name -> 'var'|'param' so that
               `from_sympy` can restore the original atom kind (sympy collapses
               both Variable and Param into a bare Symbol).
    """
    if node.is_number:
        v = node.value
        return sympy.Integer(v) if isinstance(v, int) else sympy.Rational(v).limit_denominator()

    if node.is_variable:
        if kinds is not None:
            kinds[node.name] = 'var'
        return var_map.get(node.name, sympy.Symbol(node.name))

    if node.is_param:
        if kinds is not None:
            kinds[node.name] = 'param'
        return sympy.Symbol(node.name)

    if not node.is_symbol:
        return None

    s = node.symbol
    args = [to_sympy(c, var_map, kinds) for c in node.children]
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
    if s == '==' and len(args) == 2:
        return sympy.Eq(args[0], args[1])
    if s == 'sqrt' and len(args) == 1:
        return sympy.sqrt(args[0])
    if s == 'abs' and len(args) == 1:
        return sympy.Abs(args[0])

    return None


def from_sympy(expr, var_name, Term, kinds=None):
    """Convert a sympy expression back to a Laffie Term tree.

    Args:
        expr: sympy expression.
        var_name: name of the main variable (str). Kept for signature
            compatibility; atom kind is now driven by `kinds`.
        Term: the Laffie Term class (injected in calculator context).
        kinds: optional dict name -> 'var'|'param' produced by `to_sympy`.
            Restores the original atom kind across the roundtrip (sympy
            collapses Variable and Param into a bare Symbol). When absent, or
            for a name sympy introduced itself, the symbol defaults to Variable
            — the find-target convention.
    """
    if isinstance(expr, sympy.Integer):
        return Term.number(int(expr))

    if isinstance(expr, sympy.Rational) and not isinstance(expr, sympy.Integer):
        return Term('/', [Term.number(int(expr.p)), Term.number(int(expr.q))])

    if isinstance(expr, sympy.Symbol):
        name = str(expr)
        if kinds is not None and kinds.get(name) == 'param':
            return Term.param(name)
        return Term.variable(name)

    if isinstance(expr, sympy.Add):
        children = [from_sympy(a, var_name, Term, kinds) for a in expr.args]
        if None in children:
            return None
        return Term('+', children)

    if isinstance(expr, sympy.Mul):
        children = [from_sympy(a, var_name, Term, kinds) for a in expr.args]
        if None in children:
            return None
        return Term('*', children)

    if isinstance(expr, sympy.Pow):
        base, exp = expr.args
        # Normalise Pow(x, -1) → 1/x and Pow(x, -n) → 1/x^n so that the
        # resulting Laffie term uses the canonical division form rather than
        # a negative-exponent power. Negative-exponent powers are not parser
        # syntax in .pbl files and force structural mismatches against
        # answers written with division.
        if isinstance(exp, sympy.Integer) and int(exp) < 0:
            base_term = from_sympy(base, var_name, Term, kinds)
            if base_term is None:
                return None
            n = -int(exp)
            denom = base_term if n == 1 else Term('^', [base_term, Term.number(n)])
            return Term('/', [Term.number(1), denom])
        base_term = from_sympy(base, var_name, Term, kinds)
        exp_term = from_sympy(exp, var_name, Term, kinds)
        if base_term is None or exp_term is None:
            return None
        return Term('^', [base_term, exp_term])

    # Negative number (sympy represents -3 as Mul(-1, 3) sometimes, but
    # also as Integer(-3) which is handled above)
    if expr.is_number:
        if expr.is_integer:
            return Term.number(int(expr))
        if expr.is_rational:
            return Term('/', [Term.number(int(expr.p)), Term.number(int(expr.q))])

    if isinstance(expr, sympy.Abs):
        inner = from_sympy(expr.args[0], var_name, Term, kinds)
        if inner is None:
            return None
        return Term('abs', [inner])

    if isinstance(expr, sympy.Equality):
        l = from_sympy(expr.lhs, var_name, Term, kinds)
        r = from_sympy(expr.rhs, var_name, Term, kinds)
        if l is None or r is None:
            return None
        return Term('==', [l, r])

    if isinstance(expr, sympy.Unequality):
        l = from_sympy(expr.lhs, var_name, Term, kinds)
        r = from_sympy(expr.rhs, var_name, Term, kinds)
        if l is None or r is None:
            return None
        return Term('!=', [l, r])

    if isinstance(expr, (sympy.StrictGreaterThan, sympy.GreaterThan,
                         sympy.StrictLessThan, sympy.LessThan)):
        op_map = {
            sympy.StrictGreaterThan: '>',
            sympy.GreaterThan: '>=',
            sympy.StrictLessThan: '<',
            sympy.LessThan: '<=',
        }
        op = op_map[type(expr)]
        l = from_sympy(expr.lhs, var_name, Term, kinds)
        r = from_sympy(expr.rhs, var_name, Term, kinds)
        if l is None or r is None:
            return None
        return Term(op, [l, r])

    if isinstance(expr, sympy.And):
        children = [from_sympy(a, var_name, Term, kinds) for a in expr.args]
        if None in children:
            return None
        return Term('&&', children)

    if isinstance(expr, sympy.Or):
        children = [from_sympy(a, var_name, Term, kinds) for a in expr.args]
        if None in children:
            return None
        return Term('||', children)

    return None
