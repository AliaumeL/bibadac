/// This module serves as a thin wrapper
/// around the `tree-sitter-bibtex` parser
/// and provides nice APIs to interact with
/// such files.
use once_cell::sync::OnceCell;
use tree_sitter::{Language, Node, Parser, Tree, TreeCursor};
use tree_sitter_bibtex as bibparser;

fn language() -> &'static Language {
    static LANGUAGE: OnceCell<Language> = OnceCell::new();
    LANGUAGE.get_or_init(|| bibparser::language())
}

#[derive(Debug, Clone)]
pub struct BibFile<'a> {
    pub content: &'a str,
    pub tree: Tree,
}

#[derive(Debug, Clone)]
pub struct BibField<'a> {
    pub loc: Node<'a>,
    pub name: Node<'a>,
    pub value: Node<'a>,
}

#[derive(Debug, Clone)]
pub struct BibEntry<'a> {
    pub loc: Node<'a>,
    pub key: Node<'a>,
    pub entrytype: Node<'a>,
    pub fields: Vec<BibField<'a>>,
}

impl<'a> BibEntry<'a> {

    pub fn from_node(node : Node<'a>) -> Option<Self> {
        let mut e_cursor = node.walk();
        let mut f_cursor = node.walk();
        Self::from_node_fast(node, &mut e_cursor, &mut f_cursor)
    }

    fn from_node_fast(node: Node<'a>, e_cursor : &mut TreeCursor<'a>,
                                 f_cursor : &mut TreeCursor<'a>) -> Option<Self> {
        if !(node.kind() == "entry") {
            return None;
        }
        let mut key = None;
        let mut entrytype = None;
        let mut fields = vec![];
        // loop over children
        for entry_prop in node.children(e_cursor) {
            match entry_prop.kind() {
                "key_brace" => {
                    key = Some(entry_prop);
                }
                "entry_type" => {
                    entrytype = Some(entry_prop);
                }
                "field" => {
                    let mut field_name = None;
                    let mut field_value = None;
                    for field_prop in entry_prop.children(f_cursor) {
                        match field_prop.kind() {
                            "identifier" => {
                                field_name = Some(field_prop);
                            }
                            "value" => {
                                field_value = Some(field_prop);
                            }
                            _ => {}
                        }
                    }
                    if let (Some(field_name), Some(field_value)) = (field_name, field_value) {
                        fields.push(BibField {
                            loc: entry_prop,
                            name: field_name,
                            value: field_value,
                        });
                    }
                }
                _ => {}
            }
        }
        Some(BibEntry {
            loc: node,
            key: key?,
            entrytype: entrytype?,
            fields,
        })
    }
}

impl<'a> BibFile<'a> {
    pub fn new(content: &'a str) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .expect("Failed to load bibtex language");
        let tree = parser.parse(content, None).unwrap();
        Self { content, tree }
    }

    pub fn iterate(&self) -> impl Iterator<Item = Node> {
        let root = self.tree.root_node();
        DFSIterator {
            is_up: false,
            cursor: root.walk(),
        }
    }

    pub fn get_slice(&self, node: Node) -> &'a str {
        let start = node.start_byte();
        let end = node.end_byte();
        &self.content[start..end]
    }

    pub fn list_entries(&self) -> impl Iterator<Item = BibEntry> {
        // General shape
        // (document (entry ty: (entry_type) key: (key_brace) field: (field name: (identifier) value: (value (token (brace_word)))) field: (field name: (identifier) value: (value (token (brace_word))))) ...)
        // 1. iterate over entries (entry)
        // 2. for each entry, extract key, entrytype, fields
        let mut cursor = self.tree.root_node().walk();
        let mut e_cursor = self.tree.root_node().walk();
        let mut f_cursor = self.tree.root_node().walk();
        let mut entries = vec![];

        for main_block in self.tree.root_node().children(&mut cursor) {
            if let Some(entry) = BibEntry::from_node_fast(main_block, &mut e_cursor, &mut f_cursor) {
                entries.push(entry);
            }
        }

        entries.into_iter()
    }

}

struct DFSIterator<'a> {
    is_up: bool,
    cursor: TreeCursor<'a>,
}

impl<'a> Iterator for DFSIterator<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.is_up {
                if self.cursor.goto_next_sibling() {
                    self.is_up = false;
                    return Some(self.cursor.node());
                } else {
                    if self.cursor.goto_parent() {
                        continue;
                    } else {
                        return None;
                    }
                }
            }
            if self.cursor.goto_first_child() {
                return Some(self.cursor.node());
            } else {
                if self.cursor.goto_next_sibling() {
                    return Some(self.cursor.node());
                } else {
                    self.is_up = true;
                }
            }
        }
    }
}
