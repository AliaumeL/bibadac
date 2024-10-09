/// This file is responsible for formatting the bibtex
/// entries into a "nice" representation.

/// With proper indentation and *aligned* equal signs in each entry.
/// Also, line breaks are taken into account.
///
/// 3. The fields are sorted alphabetically.
/// 4. The entry type and fields are always in lowercase
/// 5. The *author field* is formatted using the
///    Name, Firstname convention.
/// 6. All the garbage is placed *below* the entry.
///
/// ---
///
/// Next: the formatter takes as input extra fields
/// and can *fill* the missing fields using this extra
/// information (if unambiguous).
///
use crate::bibtex::{BibEntry, BibFile};
use std::collections::HashMap;
use crate::author_format::format_authors;
use crate::bibdb::{BibDb,PreBibEntry};

#[derive(Clone)]
pub struct FormatOptions<T> {
    pub indent: usize,
    pub min_field_length: Option<usize>,
    pub sort_fields: bool,
    pub sort_entries: bool,
    pub format_author: bool,
    pub whitelist: Option<Vec<String>>,
    pub blacklist: Option<Vec<String>>,
    pub database: T,
}

/// Hints are of the form
/// "doi", "name" -> ["alice"]
/// "doi", "title" -> ["title1", "title2"]
/// "eprint", "abstract" -> ["..."]
pub struct FormatHints {
    pub hints : HashMap<(String, String), Vec<String>>,
}

impl<T> FormatOptions<T> 
{
    pub fn new(db : T) -> Self {
        Self {
            indent: 2,
            min_field_length: None,
            sort_fields: true,
            sort_entries: false,
            whitelist: None,
            blacklist: Some(vec!["publisher".into(), 
                                 "editor".into(),
                                 "issue_date".into(),
                                 "numpages".into(),
                                 "address".into(),
                                 "month".into(),
                                 "series".into(),
                                 "annote".into(),
                                 "keyword".into(),
                                 "urn".into()
                                 ]),
            format_author: true,
            database: db,
        }
    }
}

pub fn write_bibfield<T,K>(_bib: &BibFile, name: &str,  value: &str, options: &FormatOptions<K>, out: &mut T)
where
    T: std::io::Write,
    K: BibDb
{
    let lines: Vec<_> = value.split('\n').collect();
    let subsequent_indent = options.indent + 4 + options.min_field_length.unwrap_or(0);
    write!(
        out,
        "{:indent$}{:<width$} = {value}",
        "",
        name.to_lowercase(),
        value = lines[0],
        indent = options.indent,
        width = options.min_field_length.unwrap_or(0),
    )
    .unwrap();
    for line in lines[1..].iter() {
        write!(out, "\n{:indent$}{}", "", line.trim(), indent = subsequent_indent).unwrap();
    }
    write!(out, ",\n").unwrap();
}

pub fn write_bibentry<T,K>(bib: &BibFile, entry: &BibEntry, options: &FormatOptions<K>, out: &mut T)
where
    T: std::io::Write,
    K: BibDb
{
    let key = bib.get_slice(entry.key);
    let entrytype = bib.get_slice(entry.entrytype);
    let prebib = PreBibEntry{ properties : entry.fields.iter().map(|f| {
                    (bib.get_slice(f.name).to_lowercase()
                     , bib.get_slice(f.value).into())
                 }).collect::<HashMap<String,String>>()
    };
    let mut compl  = options.database.complete(&prebib);
    compl.properties.retain(|k,_| {
        !prebib.properties.contains_key(k)
    });

    write!(out, "{}{{{key},\n", entrytype.to_lowercase(), key = key).unwrap();

    let mut fields = entry.fields.clone();
    if options.sort_fields {
        fields.sort_by_key(|field| bib.get_slice(field.name).to_lowercase());
    }

    for field in fields {
        if let Some(blacklist) = &options.blacklist {
            if blacklist.contains(&bib.get_slice(field.name).to_lowercase()) {
                continue;
            }
        }
        if options.format_author && bib.get_slice(field.name) == "author" {
            let authors = bib.get_slice(field.value);
            let mut formatted_authors = "{".to_string();
            formatted_authors += &format_authors(&authors[1..authors.len()-1]);
            formatted_authors += "}";
            write_bibfield(bib, "author", &formatted_authors, options, out);
        } else {
            write_bibfield(bib, bib.get_slice(field.name), bib.get_slice(field.value), options, out);
        }
    }

    if compl.properties.len() > 1 {
        writeln!(out).unwrap();
    }
    for (name, value) in compl.properties {
        write_bibfield(bib, &name, &value, options, out);
    }

    write!(out, "}}\n").unwrap();
}

pub fn write_bibfile<T,K>(bib: &BibFile, options: &FormatOptions<K>, out: &mut T)
where
    T: std::io::Write,
    K: BibDb
{
    if options.sort_entries {
        let mut cursor = bib.tree.root_node().walk();
        for entry in bib.tree.root_node().children(&mut cursor) {
            if let Some(_) = BibEntry::from_node(entry) {
            } else {
                let slice = bib.get_slice(entry);
                write!(out, "{}", slice).unwrap();
            }
        }
        let mut entries = bib.list_entries().collect::<Vec<_>>();
        entries.sort_by_key(|e| {
            let year = e.fields.iter().find_map(|f| {
                if bib.get_slice(f.name) == "year" {
                    let ctn = bib.get_slice(f.value);
                    let first_char = ctn.chars().nth(0)?;
                    if !first_char.is_digit(10) {
                        let ctn2 = &ctn[1..std::cmp::max(1,ctn.len() - 1)];
                        Some(i32::from_str_radix(ctn2, 10).unwrap_or(0))
                    } else {
                        Some(i32::from_str_radix(ctn, 10).unwrap_or(0))
                    }
                } else {
                    None
                }
            }).unwrap_or(0);
            -year
        });
        for entry in entries {
            write_bibentry(bib, &entry, options, out);
            write!(out, "\n").unwrap();
        }
    } else {
        let mut cursor = bib.tree.root_node().walk();
        for entry in bib.tree.root_node().children(&mut cursor) {
            if let Some(entry) = BibEntry::from_node(entry) {
                write_bibentry(bib, &entry, options, out);
                write!(out, "\n").unwrap();
            } else {
                let slice = bib.get_slice(entry);
                write!(out, "{}", slice).unwrap();
            }
        }
    }
}

