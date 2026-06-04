# Future Integration: ternary-regex

## Current State
Implements pattern matching on ternary sequences: `TernaryPattern` with exact/wildcard/alt/not matching, NFA construction from patterns, NFA→DFA conversion with minimization, epsilon transitions, and stream matching against ternary input.

## Integration Opportunities

### With ternary-protocol
Protocol message validation. Each message type is a `TernaryPattern`. Incoming messages are matched against the DFA — invalid messages are rejected at the pattern level before reaching application logic. `PatternElem::Alt(Ternary::Neg, Ternary::Pos)` matches any non-zero trit — useful for "this field must be active." The DFA-compiled patterns run in O(n) time per message with zero backtracking.

### With ternary-sensor
Sensor stream pattern detection. A sequence of `TernaryClass` readings (Low/Normal/High) forms a ternary stream. `TernaryPattern::exact(&[High, High, Low])` detects the anomaly signature "two consecutive highs followed by a low." The NFA matches patterns in real-time as sensor readings arrive, triggering alerts when anomalous patterns are detected.

### With ternary-automata
`TernaryNFA` is structurally identical to a cellular automaton rule table. A regex pattern over ternary sequences IS a CA rule applied to a 1D tape. The DFA minimization from `minimize()` could optimize CA rule tables — find the minimal rule that produces equivalent behavior.

## Potential in Mature Systems
In PLATO, every construct's communication contract is specified as a `TernaryPattern`. Incoming messages are matched against the contract DFA — if no accept state is reached, the message violates the contract. This is compile-time verification of inter-construct communication at the protocol level. On ESP32, the minimized DFA compiles to a jump table — constant-time per trit, no branching, no backtracking.

## Cross-Pollination Ideas
**Music × Regex:** Ternary rhythmic pattern matching. A drum pattern is a ternary sequence (0=rest, 1=tap, 2=accent). `TernaryPattern` with wildcards matches rhythmic motifs across compositions. `PatternElem::Not(Ternary::Zero)` matches "any hit regardless of accent." This enables music information retrieval by rhythmic pattern. Connects to `ternary-music` and `agent-rhythm-rs`.

**Language × Regex:** The `ternary-language` tokenizer produces ternary token sequences. `ternary-regex` matches grammatical patterns in these sequences — e.g., "Negative followed by Any followed by Positive" detects hedging language. The NFA tracks parse state as tokens stream in.

## Dependencies for Next Steps
- Integration with `ternary-protocol` for contract-based message validation
- Pattern learning: extract common ternary patterns from room state histories
- 2D pattern matching for spatial room configurations
