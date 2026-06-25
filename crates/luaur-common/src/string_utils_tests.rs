//! Inline port of `luau/tests/StringUtils.test.cpp`
//! (`TEST_SUITE("StringUtilsTest")`). Tests `editDistance` (Damerau-Levenshtein
//! with adjacent transpositions). The C++ function is byte-based, so multi-byte
//! UTF-8 characters contribute their byte length to the distance — the "unicode"
//! cases assert exactly that.

#![cfg(test)]

use crate::functions::edit_distance::editDistance;

fn ed(a: &str, b: &str) -> usize {
    editDistance(a.as_bytes(), b.as_bytes())
}

// A 1M-iteration throughput benchmark, not a correctness check — it exceeds the
// nextest slow-timeout on CI. `#[ignore]` keeps it out of the default suite; run
// it explicitly with `cargo test -- --ignored benchmark_levenshtein_distance`.
#[test]
#[ignore = "throughput benchmark, not a correctness test (exceeds the suite timeout)"]
fn benchmark_levenshtein_distance() {
    let count = 1_000_000;
    let a = "Intercalate";
    let b = "Interchangeable";

    for _ in 0..count {
        editDistance(a.as_bytes(), b.as_bytes());
    }
}

/// Run `editDistance` over every prefix pair of `a`/`b` and compare to the
/// reference matrix (`compareLevenshtein` in C++). `a`/`b` here are ASCII, so
/// byte prefixes equal character prefixes.
fn compare_levenshtein(distances: &[&[usize]], a: &str, b: &str) {
    for x in 0..=a.len() {
        for y in 0..=b.len() {
            let actual = editDistance(&a.as_bytes()[..x], &b.as_bytes()[..y]);
            let expected = distances[x][y];
            assert_eq!(
                actual,
                expected,
                "distance of {:?} and {:?}: expected {expected}, got {actual}",
                &a[..x],
                &b[..y]
            );
        }
    }
}

#[test]
fn levenshtein_distance_kitten_sitting() {
    let distances: &[&[usize]] = &[
        &[0, 1, 2, 3, 4, 5, 6, 7],
        &[1, 1, 2, 3, 4, 5, 6, 7],
        &[2, 2, 1, 2, 3, 4, 5, 6],
        &[3, 3, 2, 1, 2, 3, 4, 5],
        &[4, 4, 3, 2, 1, 2, 3, 4],
        &[5, 5, 4, 3, 2, 2, 3, 4],
        &[6, 6, 5, 4, 3, 3, 2, 3],
    ];
    compare_levenshtein(distances, "kitten", "sitting");
}

#[test]
fn levenshtein_distance_saturday_sunday() {
    let distances: &[&[usize]] = &[
        &[0, 1, 2, 3, 4, 5, 6],
        &[1, 0, 1, 2, 3, 4, 5],
        &[2, 1, 1, 2, 3, 3, 4],
        &[3, 2, 2, 2, 3, 4, 4],
        &[4, 3, 2, 3, 3, 4, 5],
        &[5, 4, 3, 3, 4, 4, 5],
        &[6, 5, 4, 4, 3, 4, 5],
        &[7, 6, 5, 5, 4, 3, 4],
        &[8, 7, 6, 6, 5, 4, 3],
    ];
    compare_levenshtein(distances, "saturday", "sunday");
}

#[test]
fn edit_distance_is_agnostic_of_argument_ordering() {
    assert_eq!(ed("blox", "block"), ed("block", "blox"));
}

#[test]
fn are_we_using_distance_with_adjacent_transpositions_and_not_optimal_string_alignment() {
    assert_eq!(ed("CA", "ABC"), 2);
}

#[test]
fn edit_distance_supports_unicode() {
    // ASCII character
    assert_eq!(ed("A block", "X block"), 1);
    // UTF-8 2/3/4-byte characters cost their byte length (the function is byte-based)
    assert_eq!(ed("A block", "À block"), 2);
    assert_eq!(ed("A block", "⪻ block"), 3);
    assert_eq!(ed("A block", "𒋄 block"), 4);
    // UTF-8 extreme (Zalgo) characters
    assert_eq!(ed("A block", "R̴̨̢̟̚ŏ̶̳̳͚́ͅb̶̡̻̞̐̿ͅl̸̼͝ợ̷̜͓̒̏͜͝ẍ̴̝̦̟̰́̒́̌ block"), 85);
}
