# ternary-regex

Pattern matching on ternary sequences (`-1`, `0`, `+1`) with NFA, DFA, and minimization.

## Why This Exists

Regular expressions are one of computing's most powerful tools, but they're built for text. If you're searching ternary sensor streams, financial signals, or encoded genomic data for patterns like "+1 followed by anything then −1" or "not zero then zero," you're stuck converting to strings and back. This crate gives you a proper regex engine — Thompson's NFA construction, subset-conversion to DFA, and Hopcroft minimization — that operates directly on ternary alphabets. Wildcards, alternatives, and negated matches are first-class citizens.

## Core Concepts

- **`Ternary`** — The three-value alphabet: `Neg` (−1), `Zero` (0), `Pos` (+1).
- **`PatternElem`** — A single pattern element: `Exact(Ternary)`, `Any` (wildcard), `Alt(a, b)` (either of two values), `Not(t)` (anything except a value).
- **`TernaryPattern`** — A compiled sequence of `PatternElem`s.
- **`TernaryNFA`** — Nondeterministic finite automaton with epsilon transitions, built via Thompson's construction.
- **`TernaryDFA`** — Deterministic finite automaton with O(1) per-element matching, produced by subset construction. Supports Hopcroft minimization.

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-regex = "0.1"
```

```rust
use ternary_regex::*;

fn main() {
    // Exact match: [Pos, Zero, Neg]
    let pattern = TernaryPattern::exact(&[Ternary::Pos, Ternary::Zero, Ternary::Neg]);
    assert!(matches(&pattern, &[Ternary::Pos, Ternary::Zero, Ternary::Neg]));
    assert!(!matches(&pattern, &[Ternary::Pos, Ternary::Zero, Ternary::Zero]));

    // Wildcard: Pos, Anything, Neg
    let pattern = TernaryPattern::new(vec![
        PatternElem::Exact(Ternary::Pos),
        PatternElem::Any,
        PatternElem::Exact(Ternary::Neg),
    ]);
    assert!(matches(&pattern, &[Ternary::Pos, Ternary::Zero, Ternary::Neg]));
    assert!(matches(&pattern, &[Ternary::Pos, Ternary::Pos, Ternary::Neg]));

    // Find all matches in a stream
    let input = vec![Ternary::Pos, Ternary::Neg, Ternary::Zero, Ternary::Pos, Ternary::Neg];
    let positions = find_matches(&pattern, &input);
    println!("Matches at positions: {:?}", positions);
}
```

## API Overview

### Pattern Construction
- `TernaryPattern::exact(seq)` — Exact-match pattern from a ternary sequence
- `TernaryPattern::new(elements)` — Pattern from explicit `PatternElem`s
- `PatternElem::Exact(t)` / `Any` / `Alt(a, b)` / `Not(t)` — Pattern element variants

### Matching Functions
- `matches(pattern, input)` — Full match check (compiles pattern to minimized DFA)
- `find_matches(pattern, input)` — All starting positions of pattern matches in a stream
- `compile(pattern)` — Compile pattern to a minimized DFA for reuse

### NFA (low-level)
- `TernaryNFA::from_pattern(pattern)` — Thompson's construction from a pattern
- `TernaryNFA::new(n)` — Manual NFA with `n` states
- `nfa.add_transition(from, elem, to)` — Add a labeled transition
- `nfa.add_epsilon(from, to)` — Add an epsilon transition
- `nfa.accepts(input)` — Test acceptance

### DFA (low-level)
- `TernaryDFA::from_nfa(nfa)` — Subset construction NFA→DFA
- `dfa.minimize()` — Hopcroft minimization
- `dfa.accepts(input)` — O(n) acceptance test
- `dfa.find_all(input)` — Find all match positions in a stream

## How It Works

**Thompson's construction** builds an NFA with one state per pattern element plus one accept state. Each `PatternElem` becomes a transition that matches if `elem_matches(elem, value)` returns true. The NFA supports epsilon transitions for future extension with alternation and repetition.

**Subset construction** converts the NFA to a DFA by computing epsilon closures and tracking sets of NFA states as single DFA states. Each DFA state has exactly 3 outgoing transitions (one per ternary value), enabling O(1) lookups during matching.

**Hopcroft minimization** refines a partition of {accept, reject} states by comparing transition signatures, merging indistinguishable states until no further refinement is possible. The result is the unique minimal DFA for the pattern.

## Use Cases

1. **Sensor anomaly detection** — Match patterns like "positive spike, any value, negative spike" in real-time ternary sensor streams.
2. **Financial signal scanning** — Search market direction sequences for specific patterns (e.g., "up, up, not-down").
3. **Genomic motif finding** — After ternarizing genomic data (below baseline / at baseline / above baseline), search for regulatory motifs.
4. **Protocol parsing** — Match ternary-encoded message headers and delimiters in communication protocols.

## Ecosystem

- [`ternary-streaming`](https://github.com/user/ternary-streaming) — Streaming processing with pattern detection
- [`ternary-automata`](https://github.com/user/ternary-automata) — Cellular automata with ternary states
- [`ternary-signals`](https://github.com/user/ternary-signals) — Signal processing for ternary data

## License

MIT

## See Also
- **ternary-grammar** — related
- **ternary-language** — related
- **ternary-compiler** — related
- **ternary-codes** — related
- **ternary-diff** — related

