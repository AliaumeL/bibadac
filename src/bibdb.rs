/// This file is responsible for 
/// fetching database information 
/// on different websites/local files
/// to help the linter and formatter
/// to do their job.

use std::collections::HashMap;


#[derive(Clone, Debug, PartialEq)]
pub struct PreBibEntry {
    pub properties : HashMap<String, String>,
}

impl PreBibEntry {

    fn is_extension_of(&self, other : &PreBibEntry) -> bool {
        other.properties
            .iter()
            .all(|(k,v)| {
                if k != "title" && k != "sha256" && k != "doi" && k != "eprint" && k != "url" {
                    return true;
                }
                if let Some(v2) = self.properties.get(k) {
                    v == v2
                } else {
                    false
                }
            })
    }

    fn merge(&mut self, other : &PreBibEntry) {
        other.properties
             .iter()
             .for_each(|(k,v)| {
                 self.properties.entry(k.to_string()).or_insert(v.to_string());
             });
    }
}

pub trait BibDb {
    fn get_doi(&self, doi : &str) -> Option<PreBibEntry>;
    fn get_eprint(&self, eprint : &str) -> Option<PreBibEntry>;
    fn complete(&self, partial : &PreBibEntry) -> PreBibEntry;
}


pub struct LocalBibDb {
    pub entries : Vec<PreBibEntry>,
}

impl Default for LocalBibDb {
    fn default() -> Self {
        LocalBibDb { entries: vec![] }
    }
}

impl LocalBibDb {
    pub fn new() -> Self {
        LocalBibDb::default()
    }

    pub fn import_bibtex(mut self, ctn : &str) -> Self {
        use crate::bibtex::BibFile;
        let file = BibFile::new(ctn);
        let new_entries : Vec<PreBibEntry> = file.list_entries()
            .into_iter()
            .map(|e| {
                PreBibEntry {
                    properties:
                        e.fields
                         .into_iter()
                         .map(|f| {
                             (file.get_slice(f.name).into(), file.get_slice(f.value).into())
                         })
                        .collect()
                }
            }).collect();
        self.entries.extend(new_entries);
        self
    }
}


impl BibDb for &mut LocalBibDb {
    fn get_doi(&self, doi : &str) -> Option<PreBibEntry> {
        self.entries.iter().find(|e| {
            if let Some(d) = e.properties.get("doi") {
                d == doi
            } else {
                false
            }
        }).cloned()
    }

    fn get_eprint(&self, eprint : &str) -> Option<PreBibEntry> {
        self.entries.iter().find(|e| {
            if let Some(d) = e.properties.get("eprint") {
                d == eprint
            } else {
                false
            }
        }).cloned()
    }

    fn complete(&self, partial : &PreBibEntry) -> PreBibEntry {
        let mut output = partial.clone();
        for entry in self.entries.iter() {
            if entry.is_extension_of(partial) {
                output.merge(entry)
            }
        }
        output
    }
}
