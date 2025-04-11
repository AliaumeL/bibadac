/// This is a minimal library
/// file to write list of authors
/// in the "BibTeX" format, that is
/// "Author1, Author1 and Author2 and Author3, Author3"
///
/// This provides a way to check validity of a given string
/// and can also be used to *format* the list of authors
/// correctly.

pub fn format_authors(authors: &str) -> String {
    authors
        .split(" and ")
        .map(|author| {
            if author.contains(",") {
                return author.to_string();
            }
            let parts = author.trim().split_whitespace().collect::<Vec<&str>>();
            if parts.len() == 1 {
                parts[0].into()
            } else {
                let new_first = parts[parts.len() - 1].to_string() + ",";
                vec![&new_first.as_str()]
                    .into_iter()
                    .chain(parts[0..parts.len() - 1].iter())
                    .cloned()
                    .collect::<Vec<&str>>()
                    .join(" ")
            }
        })
        .collect::<Vec<String>>()
        .join(" and ")
}

pub fn check_authors(authors: &str) -> bool {
    let authors = authors.split(" and ");
    for author in authors {
        let parts = author.trim().split_whitespace().collect::<Vec<&str>>();
        if parts.len() == 1 {
            continue;
        }
        if parts.len() >= 2 {
            if !parts[0].ends_with(",") {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_authors() {
        let authors = "Author1 and Author2 and Author3";
        assert_eq!(format_authors(authors), "Author1 and Author2 and Author3");
        let authors = "Author1 and A B and Author3";
        assert_eq!(format_authors(authors), "Author1 and B, A and Author3");
        let authors = "Author1 and A, B and Author3";
        assert_eq!(format_authors(authors), "Author1 and A, B and Author3");
        let authors = "A B C and D E F and G H I";
        assert_eq!(format_authors(authors), "C, A B and F, D E and I, G H");
        let authors = "Michael Kaminski and Nissim Francez";
        assert_eq!(
            format_authors(authors),
            "Kaminski, Michael and Francez, Nissim"
        );
        let authors = "DONALD E. KNUTH and PETER B. BENDIX";
        assert_eq!(
            format_authors(authors),
            "KNUTH, DONALD E. and BENDIX, PETER B."
        );
    }

    #[test]
    fn test_check_authors() {
        let authors = "Author1 and A B C and Author3";
        assert_eq!(check_authors(authors), false);
        let authors = "Author1 and A, B C and Author3";
        assert_eq!(check_authors(authors), true);
        let authors = "Author1 and A , B C and Author3";
        assert_eq!(check_authors(authors), false);
    }
}
