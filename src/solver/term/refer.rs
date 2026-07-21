use std::{
    collections::{HashSet, VecDeque},
    fmt,
    iter::Iterator,
};

use bigdecimal::BigDecimal;
use derive_more::{Debug, From};
use eyre::{Result, bail, ensure};
use itertools::Itertools;
use num::Zero;
use serde_derive::Serialize;
use trees::Node;

use utils::SubsetIterator;

use super::{Atom, ParamSubstitution, Symbol, TermBuf, TermPath, Truth, try_sym};

/// Upper bound on search nodes visited while matching one commutative operator.
/// A pathological AC term can't blow up: on exhaustion the matcher returns the
/// substitutions found so far instead of failing the whole match.
const MATCH_BUDGET: usize = 10_000;

pub trait Term {
    type RefType: Term;

    fn as_ref(&self) -> Self::RefType;

    fn parent(&self) -> Option<Self::RefType>;
    fn first_arg(&self) -> Option<Self::RefType>;
    fn last_arg(&self) -> Option<Self::RefType>;

    fn args_iter(&self) -> impl Iterator<Item = Self::RefType>;

    fn symbols(&self) -> HashSet<Symbol>;

    fn data(&self) -> &Atom;

    fn degree(&self) -> usize {
        self.as_ref().degree()
    }

    fn truth(&self) -> Truth;
}

#[derive(Clone, Copy, From)]
#[derive(Debug, Serialize)]
pub struct TermRef<'a>(&'a Node<Atom>);

impl<'a> PartialEq for TermRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.data() == other.data() &&
            self.degree() == other.degree() &&
            self.args_iter().zip(other.args_iter()).all(|(a, b)| a == b)
    }
}

impl<'a> Eq for TermRef<'a> {}

impl<'a> Term for TermRef<'a> {
    type RefType = Self;

    fn as_ref(&self) -> Self {
        *self
    }

    fn parent(&self) -> Option<Self> {
        self.0.parent().map(Self)
    }

    fn first_arg(&self) -> Option<Self::RefType> {
        self.0.front().map(TermRef)
    }

    fn last_arg(&self) -> Option<Self::RefType> {
        self.0.back().map(TermRef)
    }

    fn args_iter(&self) -> impl Iterator<Item = Self> {
        self.0.iter().map(Self)
    }

    fn symbols(&self) -> HashSet<Symbol> {
        self.0.bfs().iter.filter_map(|x| x.data.symbol()).collect()
    }

    fn data(&self) -> &Atom {
        self.0.data()
    }

    fn degree(&self) -> usize {
        self.0.degree()
    }

    fn truth(&self) -> Truth {
        self.data()
            .symbol()
            .map(|x| x.check_truth(*self))
            .unwrap_or(Truth::Unknown)
    }
}

impl<'a> TermRef<'a> {
    /// Returns `true` if both references point to the same node in memory.
    #[inline]
    pub fn same(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }

    #[inline]
    pub fn id(&self) -> TermPath {
        let mut current = *self;
        let mut path = vec![];

        while let Some(parent) = current.parent() {
            let id = parent
                .args_iter()
                .find_position(|x| x.same(&current))
                .map(|(num, _)| num)
                .expect("the parent must contains the child");
            path.push(id);
            current = parent;
        }
        path.reverse();
        path.into()
    }

    #[inline]
    pub fn bfs(&self) -> impl Iterator<Item = trees::bfs::Visit<&Atom>> {
        self.0.bfs().iter
    }

    #[inline]
    pub fn to_owned(&self) -> TermBuf {
        self.0.deep_clone().into()
    }

    /// Returns all permutations of this node's children, each wrapped in
    /// a copy of the root symbol.
    pub fn args_permutations(&self) -> Vec<TermBuf> {
        let s = self.data().symbol().unwrap();
        let args: Vec<_> = self.args_iter().map(|a| a.to_owned()).collect();
        args.into_iter()
            .permutations(self.degree())
            .map(|perm| {
                let mut node = TermBuf::from(Atom::Symbol(s.clone()));
                for arg in perm {
                    node.term_mut().push_last_arg(arg);
                }
                node
            })
            .collect()
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = TermBuf> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().symbol().unwrap();
            let mut parts = vec![TermBuf::from(Atom::Symbol(s.clone())); count];
            for (id, child) in self.args_iter().enumerate() {
                parts[i.as_vec()[id]]
                    .term_mut()
                    .push_last_arg(child.to_owned());
            }

            for p in parts.iter_mut() {
                if p.term().degree() == 1 {
                    let mut child = p.term_mut().pop_first_arg().unwrap();
                    p.term_mut().swap(&mut child.term_mut());
                }
            }
            let mut result = TermBuf::from(Atom::Symbol(s.clone()));
            for p in parts.into_iter() {
                result.term_mut().push_last_arg(p);
            }
            result
        }))
    }

    fn parentheses(&self) -> bool {
        let parent_weight = self
            .parent()
            .and_then(|x| x.data().symbol())
            .and_then(|x| x.display_weight())
            .unwrap_or(u64::MAX);
        let parent_associative = self
            .parent()
            .and_then(|x| x.data().symbol())
            .map(|x| x.is_associative())
            .unwrap_or(false);
        let weight = self
            .data()
            .symbol()
            .and_then(|x| x.display_weight())
            .unwrap_or(u64::MIN);
        weight > parent_weight || (weight == parent_weight && !parent_associative)
    }

    pub fn contains(&self, target: &TermRef) -> bool {
        if *self == *target {
            return true;
        }

        for child in self.args_iter() {
            if child.contains(target) {
                return true;
            }
        }
        false
    }

    pub fn contains_params(&self) -> bool {
        self.bfs().any(|v| v.data.param().is_some())
    }
}

impl<'a> TermRef<'a> {
    #[inline]
    pub fn try_match(&self, pattern: TermRef) -> Result<Vec<ParamSubstitution>> {
        self.try_match_extend(pattern, Default::default())
    }

    pub fn find_matching_subterms(
        &self,
        pattern: TermRef,
    ) -> Vec<(Vec<ParamSubstitution>, TermPath)> {
        self.find_matching_subterms_extend(pattern, Default::default())
    }

    pub fn find_matching_subterms_extend(
        &self,
        pattern: TermRef,
        params: ParamSubstitution,
    ) -> Vec<(Vec<ParamSubstitution>, TermPath)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((*self, self.id()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = node
                .try_match_extend(pattern, params.clone())
                .map_err(|_| trace!(target: "pattern_match", "No match for {pattern} to {node}"))
            {
                result.push((mapping, pos));
            }

            for i in node.args_iter() {
                queue.push_back((i, i.id()));
            }
        }
        result
    }

    pub fn try_match_extend(
        &self,
        pattern: TermRef,
        mut params: ParamSubstitution,
    ) -> Result<Vec<ParamSubstitution>> {
        trace!(target: "pattern_match", "Pattern: {pattern}, traget: {self}, mapping: {params}");

        match (&pattern.data(), &self.data()) {
            (Atom::Symbol(sym), Atom::Symbol(t_sym)) => {
                ensure!(sym == t_sym, "Expect symbol {sym}, found: {t_sym}");
                let mut result = vec![];

                let has_placeholder = pattern
                    .args_iter()
                    .any(|a| a.data().placeholder().is_some());

                if sym.is_commutative() && !has_placeholder {
                    // Lazy backtracking multiset matcher: assign each pattern arg a
                    // child (commutative) or a non-empty subset (associative), with
                    // pruning, dedup and a budget instead of materializing every
                    // permutation / partition.
                    self.match_commutative(pattern, params, &mut result);
                } else if sym.is_associative() && sym.is_commutative() {
                    // Placeholder spread (`..`): positional subset path.
                    for (num, parts) in self.subsets(pattern.degree()).enumerate() {
                        ensure!(num < 1025, "Subsets of operation is too large");

                        let mut loc_result = vec![params.clone()];
                        parts
                            .term()
                            .try_match_args(pattern, &mut loc_result)
                            .expect("must match");
                        result.append(&mut loc_result);
                    }
                } else if sym.is_commutative() {
                    for perm in self.args_permutations() {
                        let mut loc_result = vec![params.clone()];
                        if perm.term().try_match_args(pattern, &mut loc_result).is_ok() &&
                            !loc_result.is_empty()
                        {
                            result.append(&mut loc_result);
                        }
                    }
                } else {
                    result.push(params);
                    self.try_match_args(pattern, &mut result)?;
                }
                ensure!(!result.is_empty(), "No mapping found");
                Ok(result)
            }
            // try map (-1)*param on (-number)
            (Atom::Symbol(mul), Atom::Number(neg)) if mul == "*" && neg < &BigDecimal::zero() => {
                TermBuf::symbol("*")
                    .arg(TermBuf::number(-1))
                    .arg(TermBuf::number(neg.abs()))
                    .term()
                    .try_match_extend(pattern, params)
            }
            (Atom::Symbol(p_id), _) => {
                bail!("Expect symbol id: {p_id}, found target: {:?}", &self.data())
            }
            (Atom::Param(p), _) => {
                if let Some(node) = params.params.get(p) {
                    // Non-linear param: a repeated ?p must bind to the same term.
                    ensure!(node.term() == *self, "non-linear param {p} mismatch");
                } else {
                    params.params.insert(p.clone(), self.to_owned());
                }
                Ok(vec![params])
            }
            (Atom::Number(value), Atom::Number(other_value)) if value == other_value => {
                Ok(vec![params])
            }
            (Atom::Number(value), Atom::Number(other_value)) => {
                bail!("Expect Number {value}, found {other_value}",)
            }
            (Atom::Number(_), _) => bail!("Expect Number, found: {:?}", self.data()),
            (Atom::Variable(value), Atom::Variable(other_value)) if value == other_value => {
                Ok(vec![params])
            }
            (Atom::Variable(value), Atom::Variable(other_value)) => {
                bail!("Expect Variable {value}, found {other_value}")
            }
            (Atom::Variable(_), _) => bail!("Expect Variable, found: {:?}", self.data()),
            (Atom::ArgList(_), _) => bail!("Mapping placeholder"),
        }
    }

    fn try_match_args(&self, pattern: TermRef, result: &mut Vec<ParamSubstitution>) -> Result<()> {
        let placeholder = pattern
            .args_iter()
            .enumerate()
            .find_map(|(pos, x)| x.data().placeholder().map(|p| (pos, p)));
        let args_delta = self.degree() as i64 - pattern.degree() as i64;
        ensure!(
            placeholder.is_some() && args_delta >= -1 || placeholder.is_none() && args_delta == 0,
            "Argument size missmatch: {} {}",
            pattern.degree(),
            self.degree()
        );

        for (p, t) in pattern.args_iter().zip(
            self.args_iter()
                .enumerate()
                .filter(|(num, _)| {
                    // Skip placeholder all placeholder args but first.
                    if let Some((pos, _)) = placeholder {
                        *num <= pos || *num > 1 + pos + args_delta as usize
                    } else {
                        true
                    }
                })
                .map(|(_, x)| x),
        ) {
            if p.data().placeholder().is_some() {
                continue;
            }

            let mut new_result = vec![];
            for r in result.drain(..) {
                if let Ok(mut p) = t.try_match_extend(p, r) {
                    trace!(target: "pattern_match", "New mapping: [{}]", p.iter().format(", "));
                    new_result.append(&mut p);
                }
            }
            *result = new_result;
        }

        if let Some((pos, ph)) = placeholder {
            let mapping: Vec<_> = self
                .args_iter()
                .enumerate()
                .filter(|(num, _)| *num >= pos && *num < 1 + pos + args_delta as usize)
                .map(|(_, x)| x.to_owned())
                .collect();

            for i in result.iter_mut() {
                i.arglists.insert(ph, mapping.clone());
            }
        }

        Ok(())
    }

    /// Backtracking matcher for a commutative (optionally associative) operator
    /// without a placeholder. Collects deduplicated substitutions into `out`.
    fn match_commutative(
        &self,
        pattern: TermRef,
        params: ParamSubstitution,
        out: &mut Vec<ParamSubstitution>,
    ) {
        // `self` is the target symbol node; its symbol equals the pattern's
        // (checked by the caller), so both the operator and its associativity
        // come straight from it.
        let sym = self.data().symbol().expect("commutative match on a symbol");
        let assoc = sym.is_associative();

        let children: Vec<TermBuf> = self.args_iter().map(|c| c.to_owned()).collect();
        let pat_args: Vec<TermRef> = pattern.args_iter().collect();

        // Associative lets one pattern arg absorb several children; plain
        // commutative is a bijection.
        if assoc {
            if pat_args.is_empty() || pat_args.len() > children.len() {
                return;
            }
        } else if pat_args.len() != children.len() {
            return;
        }

        // Duplicate substitutions can only arise from equal children: the
        // bijection path skips equal neighbours, so dedup is needed only for the
        // associative subset path when a value repeats. Children are sorted.
        let needs_dedup = assoc && children.windows(2).any(|w| w[0] == w[1]);

        let mut matcher = CommutativeMatcher {
            used: vec![false; children.len()],
            budget: MATCH_BUDGET,
            seen: HashSet::new(),
            pat_args,
            children,
            sym,
            assoc,
            needs_dedup,
        };
        matcher.assign(0, params, out);
    }
}

/// Shared state for one commutative match: the immutable pattern/target context
/// plus the mutable search bookkeeping, so the recursion carries only
/// `(arg_idx, params, out)`.
struct CommutativeMatcher<'p> {
    pat_args:    Vec<TermRef<'p>>,
    children:    Vec<TermBuf>,
    sym:         Symbol,
    assoc:       bool,
    needs_dedup: bool,
    used:        Vec<bool>,
    budget:      usize,
    seen:        HashSet<String>,
}

impl CommutativeMatcher<'_> {
    /// Place pattern arg `arg_idx` (and the rest, recursively) against the
    /// still unused target children, backtracking on failure.
    fn assign(
        &mut self,
        arg_idx: usize,
        params: ParamSubstitution,
        out: &mut Vec<ParamSubstitution>,
    ) {
        if self.budget == 0 {
            return;
        }

        if arg_idx == self.pat_args.len() {
            if self.used.iter().all(|u| *u) &&
                (!self.needs_dedup || self.seen.insert(canon_key(&params)))
            {
                out.push(params);
            }
            return;
        }

        let arg = self.pat_args[arg_idx];

        if !self.assoc {
            // Each pattern arg binds exactly one still-unused child.
            for child_idx in 0..self.children.len() {
                if self.budget == 0 {
                    return;
                }
                if self.used[child_idx] {
                    continue;
                }
                // Children are sorted: skip a duplicate candidate at this level.
                if child_idx > 0 &&
                    !self.used[child_idx - 1] &&
                    self.children[child_idx] == self.children[child_idx - 1]
                {
                    continue;
                }
                self.budget = self.budget.saturating_sub(1);
                if !head_compatible(arg, self.children[child_idx].term()) {
                    continue;
                }
                if let Ok(sub_maps) = self.children[child_idx]
                    .term()
                    .try_match_extend(arg, params.clone())
                {
                    self.used[child_idx] = true;
                    for sub_map in sub_maps {
                        self.assign(arg_idx + 1, sub_map, out);
                    }
                    self.used[child_idx] = false;
                }
            }
            return;
        }

        // Associative: this pattern arg absorbs a non-empty subset of unused
        // children.
        let unused: Vec<usize> = (0..self.children.len())
            .filter(|i| !self.used[*i])
            .collect();
        let remaining_args = self.pat_args.len() - arg_idx;
        // Leave at least one child for each of the remaining pattern args.
        let max_size = unused.len() + 1 - remaining_args;
        let bits = unused.len();
        let bound: u64 = if bits >= 63 { u64::MAX } else { 1u64 << bits };

        let mut mask: u64 = 1;
        while mask < bound {
            if self.budget == 0 {
                return;
            }
            let subset: Vec<usize> = unused
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1u64 << bit) != 0)
                .map(|(_, &idx)| idx)
                .collect();
            mask += 1;

            if subset.is_empty() || subset.len() > max_size {
                continue;
            }
            self.budget = self.budget.saturating_sub(1);

            let group = if subset.len() == 1 {
                self.children[subset[0]].clone()
            } else {
                let mut g = TermBuf::from(Atom::Symbol(self.sym.clone()));
                for &idx in &subset {
                    g.term_mut().push_last_arg(self.children[idx].clone());
                }
                g
            };

            if !head_compatible(arg, group.term()) {
                continue;
            }
            if let Ok(sub_maps) = group.term().try_match_extend(arg, params.clone()) {
                for &idx in &subset {
                    self.used[idx] = true;
                }
                for sub_map in sub_maps {
                    self.assign(arg_idx + 1, sub_map, out);
                }
                for &idx in &subset {
                    self.used[idx] = false;
                }
            }
        }
    }
}

/// Cheap pre-filter: reject a candidate whose head atom can't possibly match
/// the pattern arg, before the full recursive `try_match_extend`.
fn head_compatible(pattern: TermRef, candidate: TermRef) -> bool {
    match (pattern.data(), candidate.data()) {
        (Atom::Param(_) | Atom::ArgList(_), _) => true,
        (Atom::Symbol(f), Atom::Symbol(g)) => f == g,
        // A `*` pattern may still match a negative number literal ((-1)*|n|).
        (Atom::Symbol(f), Atom::Number(n)) => f == "*" && n < &BigDecimal::zero(),
        (Atom::Symbol(_), _) => false,
        (Atom::Number(a), Atom::Number(b)) => a == b,
        (Atom::Number(_), _) => false,
        (Atom::Variable(a), Atom::Variable(b)) => a == b,
        (Atom::Variable(_), _) => false,
    }
}

/// Order-independent key of a substitution's param bindings, for deduplication.
fn canon_key(params: &ParamSubstitution) -> String {
    let mut parts: Vec<String> = params
        .params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    parts.sort();
    parts.join(";")
}

impl<'a> fmt::Display for TermRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.parentheses() {
            write!(f, "(")?;
        }

        let mul_sym_str = try_sym("*")
            .map(|x| x.to_string())
            .unwrap_or("*".to_owned());

        match self.data() {
            Atom::Symbol(symbol) => {
                let s = match symbol.display_weight() {
                    Some(_) if self.degree() < 2 => {
                        format!("{symbol}{}", self.args_iter().format(", "))
                    }
                    Some(_) => self.args_iter().join(&symbol.to_string()),
                    // Prefix notation by default
                    None if self.degree() > 0 => {
                        format!("{symbol}({})", self.args_iter().format(", "))
                    }
                    None => format!("{symbol}"),
                }
                .replace(&format!("-1{mul_sym_str}"), "-")
                .replace("+-", "-");
                write!(f, "{s}")?
            }
            Atom::Number(num) => write!(f, "{num}")?,
            _ => write!(f, "{}", self.data())?,
        }

        if self.parentheses() {
            write!(f, ")")
        } else {
            Ok(())
        }
    }
}

/// Matches a term against a symbol pattern, binding children to variables.
///
/// Each argument position can be:
/// - `ident` — binds one child to the variable
/// - `"symbol"(ident)` — checks that the child is the named symbol, then binds
///   the entire node to the variable
///
/// Returns `Option<(bindings...)>`.
///
/// # Examples
///
/// ```ignore
/// let (lhs, rhs) = match_term!(term, "in"(lhs, rhs))?;
/// let (u, s) = match_term!(term, "in"(u, "set"(s)))?;
/// ```
macro_rules! match_term {
    ($term:expr, $name:literal ( $($rest:tt)* )) => {{
        use $crate::term::Term as _;
        let __t = $term;
        (|| -> Option<_> {
            if !__t.data().is_symbol_name($name) { return None; }
            let mut __it = __t.args_iter();
            match_term!(@arms __it [] $($rest)*)
        })()
    }};

    (@arms $it:ident [$($acc:ident)*] $v:ident , $($rest:tt)*) => {{
        let $v = $it.next()?;
        match_term!(@arms $it [$($acc)* $v] $($rest)*)
    }};
    (@arms $it:ident [$($acc:ident)*] $v:ident) => {{
        let $v = $it.next()?;
        Some(($($acc,)* $v,))
    }};

    (@arms $it:ident [$($acc:ident)*] $name:literal ( $v:ident ) , $($rest:tt)*) => {{
        let $v = $it.next()?;
        if !$v.data().is_symbol_name($name) { return None; }
        match_term!(@arms $it [$($acc)* $v] $($rest)*)
    }};
    (@arms $it:ident [$($acc:ident)*] $name:literal ( $v:ident )) => {{
        let $v = $it.next()?;
        if !$v.data().is_symbol_name($name) { return None; }
        Some(($($acc,)* $v,))
    }};

    (@arms $it:ident [$($acc:ident)*] $name:literal , $($rest:tt)*) => {{
        let __a = $it.next()?;
        if !__a.data().is_symbol_name($name) { return None; }
        match_term!(@arms $it [$($acc)*] $($rest)*)
    }};
    (@arms $it:ident [$($acc:ident)*] $name:literal) => {{
        let __a = $it.next()?;
        if !__a.data().is_symbol_name($name) { return None; }
        Some(($($acc,)*))
    }};
}
pub(crate) use match_term;

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::term::{Substitute, Term, term_with_params, term_with_vars};

    #[test]
    fn match_term_nullary_literal() {
        let t = term_with_vars("x is known");
        let (l,) = match_term!(t.term(), "is"(l, "known")).expect("should match");
        assert_eq!(l.data(), term_with_vars("x").term().data());

        let other = term_with_vars("x is variable");
        assert!(match_term!(other.term(), "is"(l, "known")).is_none());
    }

    #[test]
    fn subterm_contains_direct_term() {
        let term = term_with_vars("x");
        assert!(term.term().contains(&term.term()));
    }

    #[test]
    fn subterm_does_not_contain_absent_term() {
        let term = term_with_vars("set(1, 2, 3) is known");
        let missing = term_with_vars("4");
        assert!(!term.term().contains(&missing.term()));
    }

    #[test]
    fn subterm_contains_in_multi_arity() {
        let term = term_with_vars("set(3, 5, 7) is known");
        let target = term_with_vars("5");
        assert!(term.term().contains(&target.term()));
    }

    #[test]
    fn subterm_contains_complex_term() {
        let term = term_with_vars("a*x^2 + b*x + c == 0");
        let subterm = term_with_vars("a*x^2");
        assert!(term.term().contains(&subterm.term()));
    }

    #[test]
    fn symbol_display_test() {
        for (term, display) in &[
            ("a + b + c", "a+b+c"),
            ("a*(b+c)", "a*(b+c)"),
            ("a*b + c", "a*b+c"),
            ("a*b/2 + c", "(a*b)/2+c"),
            ("a + b - c", "a+b-c"),
            ("x == -3", "x==-3"),
            ("-(-x + 2)", "-(-x+2)"),
            ("-(-1)", "--1"),
            ("118*x^2 + 1389x - 1507 == 0", "118*x^2+1389*x-1507==0"),
            ("(-3)*(x+2)", "-3*(x+2)"),
        ] {
            let term = term_with_params(term);

            assert_eq!(term.to_string(), *display);
        }
    }

    #[test]
    fn simple_param_map_test() {
        let term = term_with_vars("x + 1 == 0");
        let pattern = term_with_params("a + b == 0");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: x, b: 1 }, { a: 1, b: x }");
    }

    #[test]
    fn param_mapping_minus_sign_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a + b == 0");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: x, b: -5 }, { a: 5, b: -x }");
    }

    #[test]
    fn param_mapping_minus_sign_2_test() {
        let term = term_with_vars("-x - 5 == 0");
        let pattern = term_with_params("-a - b == 0");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: x, b: 5 }, { a: 5, b: x }");
    }

    #[test]
    fn same_param_map_test() {
        let term = term_with_vars("x + 1 == x");
        let pattern = term_with_params("a + 1 == a");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: x }");
    }

    #[test]
    fn subtree_param_map_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 2*x^2, b: x-1 }");
    }

    #[test]
    fn substitute_test() {
        let term = term_with_vars("2*x^2 + 4 == x - 1");
        let pattern = term_with_params("a + 4 == b");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        assert_eq!(maps.len(), 1);

        let mut test = term_with_params("a + 1");
        test.substitute(&maps[0]);
        insta::assert_snapshot!(test, @"2*x^2+1");
    }

    #[test]
    fn placeholder_test() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3, 5, 7) is known");

        let maps = term
            .term()
            .try_match(pattern.term())
            .map_err(|e| println!("Error: {e}"))
            .unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 3, ..1: [5, 7] }");
    }

    #[test]
    fn placeholder_empty_arglist() {
        let pattern = term_with_params("set(a, ..) is known");
        let term = term_with_vars("set(3) is known");

        let maps = term.term().try_match(pattern.term()).unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 3, ..1: [] }");
    }

    #[test]
    fn match_term_binary() {
        let term = term_with_vars("x + 1");
        let result = match_term!(term.term(), "+"(a, b));
        assert!(result.is_some());
        let (a, b) = result.unwrap();
        assert_eq!(a.to_string(), "x");
        assert_eq!(b.to_string(), "1");
    }

    #[test]
    fn match_term_wrong_symbol() {
        let term = term_with_vars("x + 1");
        let result = match_term!(term.term(), "*"(a, b));
        assert!(result.is_none());
    }

    #[test]
    fn match_term_not_a_symbol() {
        let term = term_with_vars("42");
        let result = match_term!(term.term(), "+"(a, b));
        assert!(result.is_none());
    }

    #[test]
    fn match_term_too_few_children() {
        let term = term_with_vars("set(42)");
        let result = match_term!(term.term(), "set"(a, b));
        assert!(result.is_none());
    }

    #[test]
    fn match_term_unary() {
        let term = term_with_vars("set(42)");
        let result = match_term!(term.term(), "set"(a));
        assert!(result.is_some());
        let (a,) = result.unwrap();
        assert_eq!(a.to_string(), "42");
    }

    #[test]
    fn match_term_nested_symbol() {
        let term = term_with_vars("x in set(1, 2, 3)");
        let result = match_term!(term.term(), "in"(lhs, "set"(s)));
        assert!(result.is_some());
        let (lhs, s) = result.unwrap();
        assert_eq!(lhs.to_string(), "x");
        assert!(s.data().is_symbol_name("set"));
        assert_eq!(s.degree(), 3);
    }

    #[test]
    fn match_term_nested_wrong_inner() {
        let term = term_with_vars("x in empty_set");
        let result = match_term!(term.term(), "in"(lhs, "set"(s)));
        assert!(result.is_none());
    }

    #[test]
    fn match_term_triple() {
        let term = term_with_vars("1 + 2 + 3");
        let result = match_term!(term.term(), "+"(a, b, c));
        assert!(result.is_some());
        let (a, b, c) = result.unwrap();
        assert_eq!(a.to_string(), "1");
        assert_eq!(b.to_string(), "2");
        assert_eq!(c.to_string(), "3");
    }

    #[test]
    fn ac_subset_grouping() {
        // AC pattern with fewer args than target: each pattern var absorbs a
        // non-empty subset (singleton unwrapped, multi wrapped in the symbol).
        let pattern = term_with_params("a + b");
        let target = term_with_vars("1 + 2 + 3");
        let maps = target.term().try_match(pattern.term()).unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ a: 1, b: 2+3 }, { a: 2, b: 1+3 }, { a: 1+2, b: 3 }, { a: 3, b: 1+2 }, { a: 1+3, b: 2 }, { a: 2+3, b: 1 }");
    }

    #[test]
    fn ac_many_children_no_cliff() {
        // 8 children against a 3-arg pattern: used to blow the 1025-partition
        // cliff and fail the whole match. Now it returns (budgeted) matches.
        let pattern = term_with_params("a + b + c");
        let target = term_with_vars("1 + 2 + 3 + 4 + 5 + 6 + 7 + 8");
        let maps = target.term().try_match(pattern.term());
        assert!(maps.is_ok(), "large AC match must not error");
        assert!(
            !maps.unwrap().is_empty(),
            "should find at least one grouping"
        );
    }

    #[test]
    fn commutative_placeholder_find() {
        let pattern = term_with_params("find(x, ..)");

        // find(a, b) → find(x, ..): two substitutions
        let goal = term_with_params("find(a, b)");
        let maps = goal.term().try_match(pattern.term()).unwrap();
        insta::assert_snapshot!(maps.iter().format(", "), @"{ x: a, ..1: [b] }, { x: b, ..1: [a] }");

        // find(a) → find(x, ..): one substitution, arglist empty
        let goal_single = term_with_params("find(a)");
        let maps_single = goal_single.term().try_match(pattern.term()).unwrap();
        insta::assert_snapshot!(maps_single.iter().format(", "), @"{ x: a, ..1: [] }");
    }
}
