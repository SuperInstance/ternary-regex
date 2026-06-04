//! # ternary-regex
//!
//! Pattern matching on ternary sequences (`-1`, `0`, `+1`). Provides NFA and DFA construction
//! from ternary patterns, NFA→DFA conversion with minimization, wildcard support, and
//! stream matching against ternary input.

#![forbid(unsafe_code)]

/// A ternary value: Negative (-1), Zero (0), or Positive (+1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ternary {
    Neg,
    Zero,
    Pos,
}

impl Ternary {
    pub fn to_i8(self) -> i8 {
        match self {
            Ternary::Neg => -1,
            Ternary::Zero => 0,
            Ternary::Pos => 1,
        }
    }
}

/// A single element in a ternary pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternElem {
    /// Match exactly this ternary value.
    Exact(Ternary),
    /// Match any ternary value (wildcard).
    Any,
    /// Match either of two values.
    Alt(Ternary, Ternary),
    /// Match anything except this value.
    Not(Ternary),
}

/// A compiled pattern: a sequence of PatternElems.
#[derive(Clone, Debug)]
pub struct TernaryPattern {
    pub elements: Vec<PatternElem>,
}

impl TernaryPattern {
    /// Create a pattern from a slice of PatternElems.
    pub fn new(elements: Vec<PatternElem>) -> Self {
        TernaryPattern { elements }
    }

    /// Create an exact-match pattern from a ternary sequence.
    pub fn exact(seq: &[Ternary]) -> Self {
        TernaryPattern {
            elements: seq.iter().map(|&t| PatternElem::Exact(t)).collect(),
        }
    }

    /// Check if an element matches a ternary value.
    pub fn elem_matches(elem: PatternElem, val: Ternary) -> bool {
        match elem {
            PatternElem::Exact(t) => t == val,
            PatternElem::Any => true,
            PatternElem::Alt(a, b) => val == a || val == b,
            PatternElem::Not(t) => val != t,
        }
    }
}

// ---- NFA ----

/// An NFA state ID.
type NfaState = usize;

/// A transition in the NFA: from state, on matching element, to state.
#[derive(Clone, Debug)]
struct NfaTransition {
    elem: PatternElem,
    target: NfaState,
}

/// A ternary NFA for pattern matching.
#[derive(Clone, Debug)]
pub struct TernaryNFA {
    /// Number of states.
    pub state_count: usize,
    /// Transitions: transitions[state] = list of (element, target).
    transitions: Vec<Vec<NfaTransition>>,
    /// Epsilon transitions.
    epsilon: Vec<Vec<NfaState>>,
    /// Start state.
    pub start: NfaState,
    /// Accept states.
    pub accept: Vec<bool>,
}

impl TernaryNFA {
    /// Create a new NFA with `n` states.
    pub fn new(n: usize) -> Self {
        TernaryNFA {
            state_count: n,
            transitions: vec![Vec::new(); n],
            epsilon: vec![Vec::new(); n],
            start: 0,
            accept: vec![false; n],
        }
    }

    /// Add a transition from `from` to `to` on `elem`.
    pub fn add_transition(&mut self, from: NfaState, elem: PatternElem, to: NfaState) {
        self.transitions[from].push(NfaTransition { elem, target: to });
    }

    /// Add an epsilon transition from `from` to `to`.
    pub fn add_epsilon(&mut self, from: NfaState, to: NfaState) {
        self.epsilon[from].push(to);
    }

    /// Set state `s` as accepting.
    pub fn set_accept(&mut self, s: NfaState) {
        self.accept[s] = true;
    }

    /// Compute epsilon closure of a set of states.
    pub fn epsilon_closure(&self, states: &[NfaState]) -> Vec<NfaState> {
        let mut closure = states.to_vec();
        let mut idx = 0;
        while idx < closure.len() {
            let s = closure[idx];
            for &t in &self.epsilon[s] {
                if !closure.contains(&t) {
                    closure.push(t);
                }
            }
            idx += 1;
        }
        closure.sort();
        closure.dedup();
        closure
    }

    /// Compute the set of states reachable from `states` on input `val`.
    pub fn move_on(&self, states: &[NfaState], val: Ternary) -> Vec<NfaState> {
        let mut result = Vec::new();
        for &s in states {
            for tr in &self.transitions[s] {
                if TernaryPattern::elem_matches(tr.elem, val) {
                    if !result.contains(&tr.target) {
                        result.push(tr.target);
                    }
                }
            }
        }
        result.sort();
        result.dedup();
        result
    }

    /// Test if the NFA accepts the given input sequence.
    pub fn accepts(&self, input: &[Ternary]) -> bool {
        let mut current = self.epsilon_closure(&[self.start]);
        for &val in input {
            let moved = self.move_on(&current, val);
            current = self.epsilon_closure(&moved);
            if current.is_empty() {
                return false;
            }
        }
        current.iter().any(|&s| self.accept[s])
    }

    /// Compile a TernaryPattern into an NFA using Thompson's construction.
    pub fn from_pattern(pattern: &TernaryPattern) -> Self {
        let n = pattern.elements.len();
        let mut nfa = TernaryNFA::new(n + 1);
        for (i, elem) in pattern.elements.iter().enumerate() {
            nfa.add_transition(i, *elem, i + 1);
        }
        nfa.set_accept(n);
        nfa
    }
}

// ---- DFA ----

/// A DFA state, represented as a sorted set of NFA states.
type DfaState = Vec<NfaState>;

/// A ternary DFA for pattern matching.
#[derive(Clone, Debug)]
pub struct TernaryDFA {
    /// DFA transitions: transitions[(state)][value] = target state index.
    /// State is represented as a sorted Vec<NfaState>.
    pub transitions: Vec<[usize; 3]>, // indexed by: 0=Neg, 1=Zero, 2=Pos
    /// Accept states.
    pub accept: Vec<bool>,
    /// Start state index.
    pub start: usize,
    /// State labels (sets of NFA states for each DFA state).
    pub state_labels: Vec<DfaState>,
}

/// Convert a ternary value to an index (0=Neg, 1=Zero, 2=Pos).
fn ternary_index(t: Ternary) -> usize {
    match t {
        Ternary::Neg => 0,
        Ternary::Zero => 1,
        Ternary::Pos => 2,
    }
}

impl TernaryDFA {
    /// Convert an NFA to a DFA using subset construction.
    pub fn from_nfa(nfa: &TernaryNFA) -> Self {
        let mut dfa = TernaryDFA {
            transitions: Vec::new(),
            accept: Vec::new(),
            start: 0,
            state_labels: Vec::new(),
        };

        let start_state = nfa.epsilon_closure(&[nfa.start]);
        dfa.state_labels.push(start_state.clone());
        dfa.start = 0;
        dfa.accept.push(start_state.iter().any(|&s| nfa.accept[s]));

        let all_ternary = [Ternary::Neg, Ternary::Zero, Ternary::Pos];
        let mut worklist = vec![0];

        while let Some(current_idx) = worklist.pop() {
            // Ensure transitions vec is large enough
            while dfa.transitions.len() <= current_idx {
                dfa.transitions.push([0; 3]);
            }

            let current_state = dfa.state_labels[current_idx].clone();

            for &val in &all_ternary {
                let moved = nfa.move_on(&current_state, val);
                let closure = nfa.epsilon_closure(&moved);

                if closure.is_empty() {
                    // Dead state
                    dfa.transitions[current_idx][ternary_index(val)] = usize::MAX;
                    continue;
                }

                if let Some(existing) = dfa.state_labels.iter().position(|s| *s == closure) {
                    dfa.transitions[current_idx][ternary_index(val)] = existing;
                } else {
                    let new_idx = dfa.state_labels.len();
                    dfa.state_labels.push(closure.clone());
                    dfa.accept.push(closure.iter().any(|&s| nfa.accept[s]));
                    dfa.transitions.push([0; 3]);
                    dfa.transitions[current_idx][ternary_index(val)] = new_idx;
                    worklist.push(new_idx);
                }
            }
        }

        dfa
    }

    /// Test if the DFA accepts the given input sequence.
    pub fn accepts(&self, input: &[Ternary]) -> bool {
        let mut current = self.start;
        for &val in input {
            let idx = ternary_index(val);
            if current >= self.transitions.len() || self.transitions[current][idx] == usize::MAX {
                return false;
            }
            current = self.transitions[current][idx];
        }
        current < self.accept.len() && self.accept[current]
    }

    /// Minimize the DFA using Hopcroft's algorithm (partition refinement).
    pub fn minimize(&self) -> TernaryDFA {
        let n = self.state_labels.len();
        if n <= 1 {
            return self.clone();
        }

        // Partition into accepting and non-accepting
        let mut partition: Vec<Vec<usize>> = Vec::new();
        let mut accept_set = Vec::new();
        let mut reject_set = Vec::new();
        for i in 0..n {
            if self.accept[i] {
                accept_set.push(i);
            } else {
                reject_set.push(i);
            }
        }
        if !accept_set.is_empty() {
            partition.push(accept_set);
        }
        if !reject_set.is_empty() {
            partition.push(reject_set);
        }

        // Find which partition a state belongs to
        let mut state_partition = vec![0usize; n];
        for (p_idx, part) in partition.iter().enumerate() {
            for &s in part {
                state_partition[s] = p_idx;
            }
        }

        // Refine partitions
        let mut changed = true;
        while changed {
            changed = false;
            let mut new_partition = Vec::new();
            for part in &partition {
                if part.len() <= 1 {
                    new_partition.push(part.clone());
                    continue;
                }

                // Split by transition signatures
                let mut groups: std::collections::HashMap<Vec<usize>, Vec<usize>> = std::collections::HashMap::new();
                for &s in part {
                    let sig: Vec<usize> = [Ternary::Neg, Ternary::Zero, Ternary::Pos].iter().map(|&val| {
                        let idx = ternary_index(val);
                        if s < self.transitions.len() && self.transitions[s][idx] != usize::MAX {
                            state_partition[self.transitions[s][idx]]
                        } else {
                            usize::MAX
                        }
                    }).collect();
                    groups.entry(sig).or_default().push(s);
                }

                if groups.len() > 1 {
                    changed = true;
                }
                for (_, group) in groups {
                    new_partition.push(group);
                }
            }

            partition = new_partition;
            for (p_idx, part) in partition.iter().enumerate() {
                for &s in part {
                    state_partition[s] = p_idx;
                }
            }
        }

        // Build minimized DFA
        let minimized_n = partition.len();
        let mut min_dfa = TernaryDFA {
            transitions: vec![[usize::MAX; 3]; minimized_n],
            accept: vec![false; minimized_n],
            start: state_partition[self.start],
            state_labels: Vec::new(),
        };

        // Pick representative from each partition
        for (p_idx, part) in partition.iter().enumerate() {
            let rep = part[0];
            min_dfa.accept[p_idx] = self.accept[rep];
            min_dfa.state_labels.push(part.clone());

            for &val in &[Ternary::Neg, Ternary::Zero, Ternary::Pos] {
                let idx = ternary_index(val);
                if rep < self.transitions.len() && self.transitions[rep][idx] != usize::MAX {
                    min_dfa.transitions[p_idx][idx] = state_partition[self.transitions[rep][idx]];
                }
            }
        }

        min_dfa
    }

    /// Find all matches of the pattern in the given ternary stream.
    /// Returns starting positions of all matches (prefix matching).
    pub fn find_all(&self, input: &[Ternary]) -> Vec<usize> {
        let mut matches = Vec::new();
        for start in 0..input.len() {
            // Try to match the pattern as a prefix of input[start..]
            let mut current = self.start;
            for (offset, &val) in input[start..].iter().enumerate() {
                let idx = ternary_index(val);
                if current >= self.transitions.len() || self.transitions[current][idx] == usize::MAX {
                    break;
                }
                current = self.transitions[current][idx];
                if current < self.accept.len() && self.accept[current] {
                    matches.push(start);
                    break;
                }
            }
        }
        matches
    }
}

/// Convenience: compile a pattern to a minimized DFA.
pub fn compile(pattern: &TernaryPattern) -> TernaryDFA {
    let nfa = TernaryNFA::from_pattern(pattern);
    let dfa = TernaryDFA::from_nfa(&nfa);
    dfa.minimize()
}

/// Convenience: match a pattern against a ternary sequence.
pub fn matches(pattern: &TernaryPattern, input: &[Ternary]) -> bool {
    let dfa = compile(pattern);
    dfa.accepts(input)
}

/// Convenience: find all occurrences of a pattern in a ternary stream.
pub fn find_matches(pattern: &TernaryPattern, input: &[Ternary]) -> Vec<usize> {
    let dfa = compile(pattern);
    dfa.find_all(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(v: i8) -> Ternary {
        match v {
            -1 => Ternary::Neg,
            0 => Ternary::Zero,
            _ => Ternary::Pos,
        }
    }

    fn ts(vals: &[i8]) -> Vec<Ternary> {
        vals.iter().map(|&v| t(v)).collect()
    }

    #[test]
    fn test_pattern_exact_match() {
        let pattern = TernaryPattern::exact(&ts(&[1, 0, -1]));
        assert!(matches(&pattern, &ts(&[1, 0, -1])));
        assert!(!matches(&pattern, &ts(&[1, 0, 0])));
    }

    #[test]
    fn test_pattern_wildcard() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Exact(Ternary::Pos),
            PatternElem::Any,
            PatternElem::Exact(Ternary::Neg),
        ]);
        assert!(matches(&pattern, &ts(&[1, 0, -1])));
        assert!(matches(&pattern, &ts(&[1, 1, -1])));
        assert!(matches(&pattern, &ts(&[1, -1, -1])));
        assert!(!matches(&pattern, &ts(&[0, 0, -1])));
    }

    #[test]
    fn test_pattern_alt() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Alt(Ternary::Pos, Ternary::Zero),
            PatternElem::Exact(Ternary::Neg),
        ]);
        assert!(matches(&pattern, &ts(&[1, -1])));
        assert!(matches(&pattern, &ts(&[0, -1])));
        assert!(!matches(&pattern, &ts(&[-1, -1])));
    }

    #[test]
    fn test_pattern_not() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Not(Ternary::Neg),
            PatternElem::Exact(Ternary::Pos),
        ]);
        assert!(matches(&pattern, &ts(&[1, 1])));
        assert!(matches(&pattern, &ts(&[0, 1])));
        assert!(!matches(&pattern, &ts(&[-1, 1])));
    }

    #[test]
    fn test_nfa_basic() {
        let mut nfa = TernaryNFA::new(3);
        nfa.add_transition(0, PatternElem::Exact(Ternary::Pos), 1);
        nfa.add_transition(1, PatternElem::Exact(Ternary::Neg), 2);
        nfa.set_accept(2);
        assert!(nfa.accepts(&ts(&[1, -1])));
        assert!(!nfa.accepts(&ts(&[1, 0])));
        assert!(!nfa.accepts(&ts(&[1, -1, 0])));
    }

    #[test]
    fn test_nfa_epsilon() {
        let mut nfa = TernaryNFA::new(3);
        nfa.add_transition(0, PatternElem::Exact(Ternary::Pos), 1);
        nfa.add_epsilon(1, 2);
        nfa.set_accept(2);
        // "Pos" should reach state 2 via epsilon
        assert!(nfa.accepts(&ts(&[1])));
    }

    #[test]
    fn test_nfa_from_pattern() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Exact(Ternary::Pos),
            PatternElem::Any,
            PatternElem::Exact(Ternary::Neg),
        ]);
        let nfa = TernaryNFA::from_pattern(&pattern);
        assert!(nfa.accepts(&ts(&[1, 0, -1])));
        assert!(nfa.accepts(&ts(&[1, 1, -1])));
        assert!(!nfa.accepts(&ts(&[0, 0, -1])));
    }

    #[test]
    fn test_dfa_from_nfa() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Exact(Ternary::Pos),
            PatternElem::Exact(Ternary::Zero),
        ]);
        let nfa = TernaryNFA::from_pattern(&pattern);
        let dfa = TernaryDFA::from_nfa(&nfa);
        assert!(dfa.accepts(&ts(&[1, 0])));
        assert!(!dfa.accepts(&ts(&[1, 1])));
        assert!(!dfa.accepts(&ts(&[0, 0])));
    }

    #[test]
    fn test_dfa_minimize() {
        // Create an NFA with potential for redundant states
        let pattern = TernaryPattern::new(vec![
            PatternElem::Any,
            PatternElem::Any,
        ]);
        let nfa = TernaryNFA::from_pattern(&pattern);
        let dfa = TernaryDFA::from_nfa(&nfa);
        let min = dfa.minimize();
        // Minimized DFA should accept the same inputs
        assert!(min.accepts(&ts(&[1, 0])));
        assert!(min.accepts(&ts(&[-1, -1])));
        assert!(!min.accepts(&ts(&[1])));
        assert!(!min.accepts(&ts(&[1, 0, -1])));
    }

    #[test]
    fn test_dfa_minimization_reduces_states() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Exact(Ternary::Pos),
            PatternElem::Exact(Ternary::Neg),
        ]);
        let nfa = TernaryNFA::from_pattern(&pattern);
        let dfa = TernaryDFA::from_nfa(&nfa);
        let min = dfa.minimize();
        // The minimal DFA should have fewer or equal states
        assert!(min.transitions.len() <= dfa.transitions.len());
    }

    #[test]
    fn test_find_all_matches() {
        let pattern = TernaryPattern::exact(&ts(&[1, -1]));
        let dfa = compile(&pattern);
        let input = ts(&[1, -1, 0, 1, -1, 1, 0]);
        let found = dfa.find_all(&input);
        assert_eq!(found, vec![0, 3]);
    }

    #[test]
    fn test_find_all_no_matches() {
        let pattern = TernaryPattern::exact(&ts(&[1, 1]));
        let found = find_matches(&pattern, &ts(&[-1, 0, -1, 0]));
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_all_overlapping() {
        let pattern = TernaryPattern::exact(&ts(&[1]));
        let found = find_matches(&pattern, &ts(&[1, 1, 1]));
        assert_eq!(found, vec![0, 1, 2]);
    }

    #[test]
    fn test_empty_pattern() {
        let pattern = TernaryPattern::new(vec![]);
        let dfa = compile(&pattern);
        assert!(dfa.accepts(&ts(&[])));
        assert!(!dfa.accepts(&ts(&[1])));
    }

    #[test]
    fn test_convenience_matches() {
        let pattern = TernaryPattern::exact(&ts(&[0, 1, -1]));
        assert!(matches(&pattern, &ts(&[0, 1, -1])));
        assert!(!matches(&pattern, &ts(&[0, 1, 0])));
    }

    #[test]
    fn test_nfa_epsilon_closure() {
        let mut nfa = TernaryNFA::new(4);
        nfa.add_epsilon(0, 1);
        nfa.add_epsilon(1, 2);
        nfa.add_epsilon(2, 3);
        let closure = nfa.epsilon_closure(&[0]);
        assert_eq!(closure, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_dfa_single_element_pattern() {
        let pattern = TernaryPattern::exact(&ts(&[1]));
        let found = find_matches(&pattern, &ts(&[0, 1, 0, 1, 0]));
        assert_eq!(found, vec![1, 3]);
    }

    #[test]
    fn test_wildcard_stream_match() {
        let pattern = TernaryPattern::new(vec![
            PatternElem::Exact(Ternary::Pos),
            PatternElem::Any,
            PatternElem::Any,
            PatternElem::Exact(Ternary::Neg),
        ]);
        assert!(matches(&pattern, &ts(&[1, 0, 0, -1])));
        assert!(matches(&pattern, &ts(&[1, 1, -1, -1])));
        assert!(!matches(&pattern, &ts(&[1, 0, 0, 0])));
    }
}
