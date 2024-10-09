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

use crate::arxiv_identifiers::ArxivId;
use crate::bibtex::{BibEntry, BibFile};
use std::fmt::{self, Debug, Formatter};
use tree_sitter::Node;

pub struct LinterState<'a> {
    pub revoked_dois: HashSet<&'a str>,
    pub arxiv_latest: HashMap<&'a str, usize>,
    pub doi_arxiv: HashMap<&'a str, &'a str>,
    pub arxiv_doi: HashMap<&'a str, &'a str>,
}

#[derive(Debug)]
pub enum LintMessage {
    EmptyKey,
    WeirdCharacters,
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

pub struct Lint<'a> {
    pub msg: LintMessage,
    loc: Vec<Node<'a>>,
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
        if value.contains(|c: char| !c.is_ascii_alphanumeric()) {
            return Some(LintMessage::WeirdCharacters);
        }
        if key == "author" && !value.contains(',') {
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
        None
    }

    pub fn lint_entry(&self, file: &BibFile<'a>, entry: BibEntry<'a>) -> Vec<LintMessage> {
        let mut messages = vec![];
        let fields = entry
            .fields
            .iter()
            .map(|field| (file.get_slice(field.name), file.get_slice(field.value)))
            .collect::<HashMap<_, _>>();
        for f in ["author", "title", "year"].iter() {
            if !fields.contains_key(f) {
                messages.push(LintMessage::MissingField(f.to_string()));
            }
        }
        for f in ["sha256"].iter() {
            if !fields.contains_key(f) {
                messages.push(LintMessage::MissingOptionalField(f.to_string()));
            }
        }
        if !fields.contains_key("url")
            && !fields.contains_key("doi")
            && !fields.contains_key("isbn")
            && !fields.contains_key("issn")
            && !fields.contains_key("arxiv")
            && !fields.contains_key("pmid")
        {
            messages.push(LintMessage::UncheckableEntry);
        }

        if !fields.contains_key("url")
            && !fields.contains_key("doi")
            && !fields.contains_key("isbn")
            && !fields.contains_key("issn")
            && !fields.contains_key("arxiv")
            && !fields.contains_key("pmid")
        {
            messages.push(LintMessage::UncheckableEntry);
        }

        let mut seen = HashSet::new();
        for f in entry.fields {
            let k = file.get_slice(f.name);
            if seen.contains(k) {
                messages.push(LintMessage::DuplicateKey(k.to_string()));
            }
            seen.insert(k);
        }
        messages.extend(fields.iter().filter_map(|(k, v)| self.lint_field(k, v)));

        messages
    }

    pub fn lint_file(&self, file: &BibFile<'a>, entries: Vec<BibEntry<'a>>) -> Vec<LintMessage> {
        let mut messages = vec![];
        let mut seen = HashSet::new();
        let mut doi_arxiv_sha256: HashMap<(&'a str, &'a str, &'a str), Vec<String>> =
            HashMap::new();

        // 1. accumulate errors for all the entries
        // 2. check for duplicate entries (same key)
        for entry in entries {
            let fields = entry
                .fields
                .iter()
                .map(|field| (file.get_slice(field.name), file.get_slice(field.value)))
                .collect::<HashMap<_, _>>();
            let key = entry.key.to_string();
            let doi = fields.get("doi").map(|s| *s).unwrap_or("");
            let arxiv = fields.get("arxiv").map(|s| *s).unwrap_or("");
            let sha256 = fields.get("sha256").map(|s| *s).unwrap_or("");
            doi_arxiv_sha256
                .entry((doi, arxiv, sha256))
                .or_default()
                .push(key.clone());

            seen.insert(key.clone());
            messages.extend(self.lint_entry(file, entry));
            if seen.contains(&key) {
                messages.push(LintMessage::DuplicateKey(key));
            }
        }

        // 3. check for duplicate entries (same DOI/ARXIV/SHA256 pair)
        for ((doi, arxiv, sha), entries) in doi_arxiv_sha256.into_iter() {
            if !doi.is_empty() && !arxiv.is_empty() && !sha.is_empty() && entries.len() > 1 {
                messages.push(LintMessage::DuplicateDoiArxivSha256(
                    doi.into(),
                    arxiv.into(),
                    sha.into(),
                ));
            }
        }
        // 4. outdated entries (arxiv versions)
        // For this, we need to be a bit smart:
        // - if the entry refers *only* to arxiv, then we can check outdatedness (but there may be
        // several versions of the same paper)
        // - if the entry refers to a *doi/url* and also an arxiv version, then it *SHOULD NOT* be
        // pinned
        messages
    }
}
