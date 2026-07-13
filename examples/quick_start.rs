use ternary_regex::*;

fn main() {
    // Exact match: [Pos, Zero, Neg]
    let pattern = TernaryPattern::exact(&[Ternary::Pos, Ternary::Zero, Ternary::Neg]);
    assert!(matches(
        &pattern,
        &[Ternary::Pos, Ternary::Zero, Ternary::Neg]
    ));
    assert!(!matches(
        &pattern,
        &[Ternary::Pos, Ternary::Zero, Ternary::Zero]
    ));

    // Wildcard: Pos, Anything, Neg
    let pattern = TernaryPattern::new(vec![
        PatternElem::Exact(Ternary::Pos),
        PatternElem::Any,
        PatternElem::Exact(Ternary::Neg),
    ]);
    assert!(matches(
        &pattern,
        &[Ternary::Pos, Ternary::Zero, Ternary::Neg]
    ));
    assert!(matches(
        &pattern,
        &[Ternary::Pos, Ternary::Pos, Ternary::Neg]
    ));

    // Find all matches in a stream.
    let input = vec![
        Ternary::Pos,
        Ternary::Neg,
        Ternary::Zero,
        Ternary::Pos,
        Ternary::Zero,
        Ternary::Neg,
    ];
    let positions = find_matches(&pattern, &input);
    println!("Matches at positions: {:?}", positions);
}
