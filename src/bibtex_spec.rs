/// This file checks compliancy with the BibTeX specification.
///
use std::collections::HashSet;
use std::sync::OnceLock;

pub const BIBTEX_ENTRY_TYPES: [&str; 24] = [
    "article",
    "book",
    "booklet",
    "conference",
    "inbook",
    "incollection",
    "inproceedings",
    "manual",
    "mastersthesis",
    "misc",
    "phdthesis",
    "proceedings",
    "techreport",
    "unpublished",
    "patent",
    "bookinbook",
    "suppbook",
    "suppcollection",
    "suppperiodical",
    "mvbook",
    "mvcollection",
    "mvproceedings",
    "talk",
    "mapping",
];

pub const BIBTEX_FIELDS: [&str; 28] = [
    "address",
    "annote",
    "author",
    "booktitle",
    "chapter",
    "crossref",
    "edition",
    "editor",
    "howpublished",
    "institution",
    "journal",
    "key",
    "month",
    "note",
    "number",
    "organization",
    "pages",
    "publisher",
    "school",
    "series",
    "title",
    "type",
    "volume",
    "year",
    "eprint",
    "archiveprefix",
    "primaryclass",
    "keywords",
];

struct NFA<T> {
    final_states: Vec<T>,
    transitions: Vec<(T, Option<char>, T)>,
    initial_states: Vec<T>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

// runs first A and at any time can switch to B,
// but can never switch back to A
fn non_deterministic_duplicate<T>(a: NFA<T>) -> NFA<Either<T, T>>
where
    T: Clone + Eq + std::hash::Hash,
{
    let mut transitions = Vec::new();
    let mut final_states = Vec::new();

    // the two automata are the same,
    // one can switch between them at any time
    // -> either following a transition (reading a wrong letter)
    // -> or "staying in the same state" (not reading a letter)
    for (from, c, to) in a.transitions.iter() {
        transitions.push((Either::Left(from.clone()), *c, Either::Left(to.clone())));
        transitions.push((Either::Right(from.clone()), *c, Either::Right(to.clone())));
        transitions.push((Either::Left(from.clone()), None, Either::Right(to.clone())));
        transitions.push((
            Either::Left(from.clone()),
            None,
            Either::Right(from.clone()),
        ));
    }

    // We also add transitions that "read two letters"
    // at once (i.e., we can decide to invent a letter)
    for (u, la, v) in a.transitions.iter() {
        for (w, lb, x) in a.transitions.iter() {
            if v != w {
                continue;
            }
            transitions.push((Either::Left(u.clone()), *la, Either::Right(x.clone())));
            transitions.push((Either::Left(u.clone()), *lb, Either::Right(x.clone())));
        }
    }

    for state in a.final_states {
        final_states.push(Either::Right(state));
    }

    NFA {
        final_states,
        transitions,
        initial_states: a
            .initial_states
            .into_iter()
            .map(|s| Either::Left(s))
            .collect(),
    }
}

fn assigning_automaton(strings: Vec<&str>) -> NFA<(usize, usize)> {
    // states are (string index, character index)
    // transitions are (state, character, state)
    // whenever
    // (string[..i] + character) = string[..j]
    // we compute it in the MOST NAIVE WAY POSSIBLE
    //
    // Final states are of the form
    // (i, j) where j == len(strings[i])
    // Initial states are of the form
    // (i, 0) for all i
    let mut transitions = Vec::new();
    let final_states = strings
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.len()))
        .collect();
    let initial_states = strings.iter().enumerate().map(|(i, _)| (i, 0)).collect();

    // iterate over (i,k) (j,l) pairs where k < len(s[i]) and l < len(s[j])
    // and characters c in used_chars
    // if s[i][..k] == s[j][..l] and s[i][k+1] == c then we add a transition
    // from (i,k) to (j,l+1)
    for (i, s) in strings.iter().enumerate() {
        for (j, t) in strings.iter().enumerate() {
            for (k, c) in s.chars().enumerate() {
                for (l, d) in t.chars().enumerate() {
                    if k == l && c == d && s[..k] == t[..l] {
                        transitions.push(((i, k), Some(c.clone()), (j, l + 1)));
                    }
                }
            }
        }
    }

    NFA {
        final_states,
        transitions,
        initial_states,
    }
}

fn run_transition<T>(from: T, c: Option<char>, to: T, state: T, d: char) -> Option<T>
where
    T: Eq + Clone + std::hash::Hash,
{
    if state != from {
        return None;
    }
    match c {
        Some(e) if e == d => Some(to),
        None => Some(to),
        _ => None,
    }
}

fn run_automaton<T>(a: &NFA<T>, s: &str) -> HashSet<T>
where
    T: Eq + Clone + std::hash::Hash + std::fmt::Debug,
{
    let mut current_states: HashSet<T> = a.initial_states.iter().cloned().collect();
    for c in s.chars() {
        current_states = current_states
            .iter()
            .flat_map(|state| {
                a.transitions
                    .iter()
                    .filter_map(|(from, d, to)| {
                        run_transition(from.clone(), *d, to.clone(), state.clone(), c)
                    })
                    .collect::<HashSet<T>>()
            })
            .collect();
    }
    current_states
        .into_iter()
        .filter(|s| a.final_states.contains(s))
        .collect()
}

fn field_typo_automaton() -> &'static NFA<Either<(usize, usize), (usize, usize)>> {
    static INIT: OnceLock<NFA<Either<(usize, usize), (usize, usize)>>> = OnceLock::new();
    INIT.get_or_init(|| {
        non_deterministic_duplicate(assigning_automaton(
            BIBTEX_FIELDS.iter().map(|s| *s).collect(),
        ))
    })
}

fn entry_typo_automaton() -> &'static NFA<Either<(usize, usize), (usize, usize)>> {
    static INIT: OnceLock<NFA<Either<(usize, usize), (usize, usize)>>> = OnceLock::new();
    INIT.get_or_init(|| {
        non_deterministic_duplicate(assigning_automaton(
            BIBTEX_ENTRY_TYPES.iter().map(|s| *s).collect(),
        ))
    })
}

/// Check if the field is *close* to a bibtex field
/// (i.e. the field is a typo of a bibtex field).
/// We look at edit distance of 1.
/// This is done by constructing the levenshtein automaton
/// and checking if the field is accepted by the automaton.
pub fn field_typo(s: &str) -> Vec<&'static str> {
    let nfa = field_typo_automaton();
    let states = run_automaton(nfa, s);
    states
        .iter()
        .map(|s| match s {
            Either::Left(s) => panic!(
                "should not happen {} {}",
                BIBTEX_FIELDS[s.0],
                &BIBTEX_FIELDS[s.0][..s.1]
            ),
            Either::Right(s) => BIBTEX_FIELDS[s.0],
        })
        .collect()
}

pub fn entry_typo(s: &str) -> Vec<&'static str> {
    let nfa = entry_typo_automaton();
    let states = run_automaton(nfa, s);
    states
        .iter()
        .map(|s| match s {
            Either::Left(_) => panic!("should not happen"),
            Either::Right(s) => BIBTEX_ENTRY_TYPES[s.0],
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_word_automaton_two_words() {
        let a = assigning_automaton(vec!["hello", "world"]);
        assert_eq!(
            run_automaton(&a, "hello"),
            vec![(0, 5)].into_iter().collect()
        );
        assert_eq!(
            run_automaton(&a, "world"),
            vec![(1, 5)].into_iter().collect()
        );
        assert_eq!(run_automaton(&a, "hell").len(), 0);
    }

    #[test]
    fn test_word_automaton_two_prefixes() {
        let a = assigning_automaton(vec!["hello", "hell"]);
        assert_eq!(
            run_automaton(&a, "hello"),
            vec![(0, 5)].into_iter().collect()
        );
        assert_eq!(
            run_automaton(&a, "hell"),
            vec![(1, 4)].into_iter().collect()
        );
        assert_eq!(run_automaton(&a, "hel").len(), 0);
    }

    #[test]
    fn test_one_letter_typo() {
        assert_eq!(field_typo("author"), vec!["author"]);
        // assert_eq!(field_typo("auhtor"), vec!["author"]);
        assert_eq!(field_typo("autho"), vec!["author"]);
        assert_eq!(field_typo("authr"), vec!["author"]);
        assert_eq!(field_typo("auth").len(), 0);
    }

    #[test]
    fn test_ambiguous_completion() {
        assert_eq!(entry_typo("mvbook"), vec!["mvbook"]);
        let mut res = entry_typo("mbook");
        res.sort();
        assert_eq!(res, vec!["book", "mvbook"]);
    }
}
