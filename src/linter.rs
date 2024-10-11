/// This is the file that contains
/// the linter rules for the `bibadac` program.
///
/// It does not *per-se* contains any parsing logic.
/// But it uses the `Node` abstraction of TreeSitter
/// to point precise locations for the errors.
///
/// field level lint warnings:
/// - empty key (location: key)
/// - using weird characters (location: field value)
/// - author writing is not "Last, First" (location: field_value)
/// - using "arxiv" as a DOI (bad practice) (location: field_value)
/// - using "http" as a DOI (bad practice) (location: field_value)
///
/// entry level lint warnings:
/// - missing important fields (author, title, year) (location: entry)
/// - uncheckable entry (no url, nor doi, nor isbn, nor issn, nor arxiv, nor pmid) (location: entry)
/// - missing optional fields (sha256) (location: entry)
/// - duplicate field name (location: Vec<field_key>)
///
/// file level lint warnings:
/// - duplicate entries (same key) (location: Vec<entry>)
/// - duplicate entries (same DOI/ARXIV/SHA256 pair) (location: Vec<entry>)
/// - outdated entries  (arxiv versions) (location: Vec<entry>)
/// - published equivalents (arxiv -> doi / doi -> arxiv) (location: Vec<entry>)
/// - revoked entries   (doi revoked) (location: Vec<entry>)
///
///
/// To do these checks we need to:
///
/// 1. have access to the list of DOIs/ARXIVs/SHA256s of the entries
/// 2. be given a list of "published equivalents" (arxiv -> doi / doi -> arxiv)
/// 3. be given a list of "revoked entries" (doi revoked)
/// 4. be given a list of "outdated entries" (arxiv versions)
///
/// Also, in the state, we need to be able to have access to the location of every key
/// every field_value, and every entry. This means that the abstraction of BibEntry / BibFile
/// should keepd this information available.
///
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::author_format::check_authors;
use crate::bibtex::tree_sitter::Node;
use crate::bibtex::{BibEntry, BibFile};
use std::fmt::{self, Debug, Formatter};

pub struct LinterState<'a> {
    pub revoked_dois: HashSet<&'a str>,
    pub arxiv_latest: HashMap<&'a str, usize>,
    pub doi_arxiv: HashMap<&'a str, &'a str>,
    pub arxiv_doi: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LintMessage {
    SyntaxError(String),
    EmptyKey,
    WeirdCharacters(String),
    AuthorFormat,
    ArxivAsDoi,
    HttpDoi,
    MissingField(String),
    UncheckableEntry,
    MissingOptionalField(String),
    DuplicateFieldName(String),
    DuplicateKey(String),
    DuplicateDoiArxivSha256(String, String, String),
    OutdatedEntry,
    PublishedEquivalent,
    RevokedEntry,
}

impl LintMessage {
    pub fn is_crucial(&self) -> bool {
        match self {
            LintMessage::SyntaxError(_) => true,
            LintMessage::EmptyKey => true,
            LintMessage::WeirdCharacters(_) => false,
            LintMessage::AuthorFormat => false,
            LintMessage::ArxivAsDoi => false,
            LintMessage::HttpDoi => false,
            LintMessage::MissingField(_) => true,
            LintMessage::UncheckableEntry => false,
            LintMessage::MissingOptionalField(_) => false,
            LintMessage::DuplicateFieldName(_) => true,
            LintMessage::DuplicateKey(_) => true,
            LintMessage::DuplicateDoiArxivSha256(_, _, _) => true,
            LintMessage::OutdatedEntry => false,
            LintMessage::PublishedEquivalent => false,
            LintMessage::RevokedEntry => false,
        }
    }
}

/// A message, and the *reason* why it was triggered
pub struct Lint<'a> {
    pub msg: LintMessage,
    pub loc: Vec<Node<'a>>,
}

impl Debug for Lint<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.msg, self.loc)
    }
}

impl<'a> LinterState<'a> {
    fn lint_field(&self, key: &str, value: &str) -> Option<LintMessage> {
        if value.is_empty() {
            return Some(LintMessage::EmptyKey);
        }
        if key == "author" && !check_authors(value) {
            return Some(LintMessage::AuthorFormat);
        }
        if key == "doi" && value.contains("arXiv") {
            return Some(LintMessage::ArxivAsDoi);
        }
        if key == "doi" && value.starts_with("http") {
            return Some(LintMessage::HttpDoi);
        }
        if key == "doi" && self.revoked_dois.contains(value) {
            return Some(LintMessage::RevokedEntry);
        }
        // we allow "{", "}", and ","
        if key != "doi"
            && key != "eprint"
            && key != "url"
            && value.contains(|c: char| c != '\n' && (c.is_control() || c == '\\'))
        {
            return Some(LintMessage::WeirdCharacters(value.to_string()));
        }
        None
    }

    pub fn lint_entry(&self, file: &BibFile<'a>, entry: BibEntry<'a>) -> Vec<Lint<'a>> {
        let mut messages = vec![];
        let fields = entry
            .fields
            .iter()
            .map(|field| {
                (
                    file.get_slice(field.name),
                    file.get_braceless_slice(field.value),
                )
            })
            .collect::<HashMap<_, _>>();
        for f in ["author", "title", "year"].iter() {
            if !fields.contains_key(f) {
                messages.push(Lint {
                    msg: LintMessage::MissingField(f.to_string()),
                    loc: vec![entry.loc],
                });
            }
        }
        for f in ["sha256"].iter() {
            if !fields.contains_key(f) {
                messages.push(Lint {
                    msg: LintMessage::MissingOptionalField(f.to_string()),
                    loc: vec![entry.loc],
                });
            }
        }
        if !fields.contains_key("url")
            && !fields.contains_key("doi")
            && !fields.contains_key("isbn")
            && !fields.contains_key("issn")
            && !fields.contains_key("eprint")
            && !fields.contains_key("pmid")
        {
            messages.push(Lint {
                msg: LintMessage::UncheckableEntry,
                loc: vec![entry.loc],
            });
        }

        let mut defined_keys = HashMap::new();
        for f in entry.fields.iter() {
            let k = file.get_slice(f.name);
            defined_keys.entry(k).or_insert(vec![]).push(f.loc);
        }
        for (k, locs) in defined_keys {
            if locs.len() > 1 {
                messages.push(Lint {
                    msg: LintMessage::DuplicateFieldName(k.to_string()),
                    loc: locs,
                });
            }
        }
        messages.extend(entry.fields.iter().filter_map(|f| {
            let keystr = file.get_slice(f.name);
            let valuestr = file.get_braceless_slice(f.value);
            let msg = self.lint_field(keystr, valuestr)?;
            Some(Lint {
                msg,
                loc: vec![f.loc],
            })
        }));

        messages
    }

    pub fn lint_file(&self, file: &'a BibFile<'a>, entries: Vec<BibEntry<'a>>) -> Vec<Lint<'a>> {
        let mut messages = vec![];
        let mut used_keys: HashMap<&str, Vec<Node<'a>>> = HashMap::new();
        let mut doi_arxiv_sha256: HashMap<(&'a str, &'a str, &'a str), Vec<Node<'a>>> =
            HashMap::new();

        // 0. check for syntax errors in the file
        // (list error nodes as "syntax errors")
        for node in file.iterate() {
            if node.kind() == "ERROR" {
                messages.push(Lint {
                    msg: LintMessage::SyntaxError(file.get_slice(node).to_string()),
                    loc: vec![node],
                });
            }
        }

        // 1. accumulate errors for all the entries
        // 2. check for duplicate entries (same key)
        for entry in entries {
            let fields = entry
                .fields
                .iter()
                .map(|field| {
                    (
                        file.get_slice(field.name),
                        file.get_braceless_slice(field.value),
                    )
                })
                .collect::<HashMap<_, _>>();
            let key = file.get_slice(entry.key);
            let doi = fields.get("doi").map(|s| *s).unwrap_or("");
            let arxiv = fields.get("arxiv").map(|s| *s).unwrap_or("");
            let sha256 = fields.get("sha256").map(|s| *s).unwrap_or("");
            doi_arxiv_sha256
                .entry((doi, arxiv, sha256))
                .or_default()
                .push(entry.loc);

            used_keys.entry(key).or_insert(vec![]).push(entry.loc);
            messages.extend(self.lint_entry(file, entry));
        }

        for (key, locs) in used_keys {
            if locs.len() > 1 {
                messages.push(Lint {
                    msg: LintMessage::DuplicateKey(key.to_string()),
                    loc: locs,
                });
            }
        }

        // 3. check for duplicate entries (same DOI/ARXIV/SHA256 pair)
        for ((doi, arxiv, sha), entries) in doi_arxiv_sha256.into_iter() {
            if !(doi.is_empty() && arxiv.is_empty() && sha.is_empty()) && entries.len() > 1 {
                messages.push(Lint {
                    msg: LintMessage::DuplicateDoiArxivSha256(doi.into(), arxiv.into(), sha.into()),
                    loc: entries,
                });
            }
        }
        // 4. outdated entries (arxiv versions)
        // - if the entry refers *only* to arxiv, then we can check outdatedness (but there may be
        // several versions of the same paper)
        // - if the entry refers to a *doi/url* and also an arxiv version, then it *SHOULD NOT* be
        // pinned

        // a. take all arxiv entries, remove those that have a DOI associated
        //    (print an error if the arxiv version is not pinned)
        // b. check if the arxiv version is outdated for the rest of the entries

        // 5. published equivalents (arxiv -> doi / doi -> arxiv)
        // TODO.

        messages
    }
}
