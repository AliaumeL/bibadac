/// This file is responsible for formatting the bibtex
/// entries into a "nice" representation.
///
/// 1. entries are separated by a blank line
/// 2. each entry is formatted as follows:
/// ```
/// @<entry_type>{<key>,
///    <field1> = {<value1>},
///    <field2> = {<value2>},
///    ...
///  }
/// ```
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
use crate::bibtex::{BibEntry, BibField, BibFile};

pub struct FormatOptions {
    pub indent: usize,
    pub sort_fields: bool,
    pub sort_entries: bool,
    pub format_author: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 4,
            sort_fields: true,
            sort_entries: false,
            format_author: true,
        }
    }
}

pub fn write_bibfield<T>(bib: &BibFile, field: &BibField, options: &FormatOptions, out: &mut T)
where
    T: std::io::Write,
{
    let name = bib.get_slice(field.name);
    let value = bib.get_slice(field.value);
    let lines: Vec<_> = value.split('\n').collect();
    write!(
        out,
        "{:indent$} = {value}",
        name,
        value = lines[0],
        indent = options.indent,
    )
    .unwrap();
    for line in lines[1..].iter() {
        write!(out, "\n{:indent$}{}", line, indent = options.indent + 3).unwrap();
    }
    write!(out, ",\n").unwrap();
}

pub fn write_bibentry<T>(bib: &BibFile, entry: &BibEntry, options: &FormatOptions, out: &mut T)
where
    T: std::io::Write,
{
    let key = bib.get_slice(entry.key);
    let entrytype = bib.get_slice(entry.entrytype);

    write!(out, "@{}{{{key},\n", entrytype.to_lowercase(), key = key).unwrap();

    let mut fields = entry.fields.clone();
    if options.sort_fields {
        fields.sort_by_key(|field| bib.get_slice(field.name).to_lowercase());
    }

    for field in fields {
        write_bibfield(bib, &field, options, out);
    }

    write!(out, "}}\n").unwrap();
}

pub fn write_bibfile<T>(bib: &BibFile, options: &FormatOptions, out: &mut T)
where
    T: std::io::Write,
{
    // FIXME: keep the junk between entries
    // We should rather be doing the following
    //
    // "reparse" the file -> for every junk, write it *as-is*
    // for every entry, format it.
    //
    // This does not account for the errors *inside* the entries.
    for entry in bib.list_entries() {
        write_bibentry(bib, &entry, options, out);
        write!(out, "\n").unwrap();
    }
}
