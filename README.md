# Bibadac

Bibtex linter and formatter written in Rust.

## Rationale

Writing papers requires citing papers. In practice for computer scientists and
mathematicians, this means interacting a lot with the [BibTeX] format. This
"format" is very flexible, and as a result, it is very easy to make mistakes. 
This tools is destined to the following people:

1. The *debuggers* who wants to understand why their bibliography does not
   compile, does strange things with unicode, etc. 
2. The *hoarders* who like to have their own collection of papers *somewhere*
   on their computer, but sometimes have to actually collaborate with others.
3. The *perfectionists* who want to have a consistent style in their
   bibliography, removing unused fields, etc.
4. The *superciters* people who would like to easily access the bibliography of
   a paper they are writing to precisely give theorem numbers or page numbers.
5. The *fearful* people who desire to know that none of the papers they refer
   to are *retracted*, or have a new version available.


## Features

This tool is still in development, but the following features are planned:

- [ ] Highlighting probable *syntax errors* in the BibTeX file, very fast.
- [ ] Pointing out *semantic errors* in the BibTeX file, such as missing
  fields, fields with wrong types, multiply defined keys, multiply defined
entries, etc.
- [ ] Pointing out *meta errors*, such as *not citing the latest arxiv version
  of a paper* or *citing a retracted paper*, or *citing an arxiv paper that has
been published since*.
- [ ] Formatting a BibTeX file according to a given style (e.g. removing unused
  fields, sorting entries, etc.)
- [ ] *Autocompleting* BibTeX entries by querying a database of papers (e.g.
  Google Scholar, DBLP, local bibtex files, etc.)
- [ ] *Downloading* papers from the internet based on the Bibtex entries and
  *pinning them* inside the BibTeX file.
- [ ] *Merging* several bibtex files together and resolving conflicts : duplicate entries,
    different versions of the same paper, different entry names for the same document, etc.

Note that all of these actions will take place in the *command line*. The goal
is not to have a graphical tool that people will use to manage their
bibliography, but a simple tool that can be integrated to any text editor or
continuous integration system to make their preferred tasks *automatic* (hence
less error-prone).

## Usage

There are three main commands to `bibadac`: 

- `bibadac check`: Check the validity of a BibTeX/BibLaTeX file
- `bibadac format`: Format a BibTeX/BibLaTeX file
- `bibadac setup`: Download pdfs that are mentionned in the file

For instance, the command `bibadac check mybib.bib` will 
print a report of the `mybib.bib` file, with all the errors and
warnings that were found. The exit code will be 0 if no error was found, 1 for
errors, and 101 in case of internal errors.

In order to format a BibTeX file, one can use the following command 
`bibadac format mybib.bib`. Note that by default, the formatted file is
printed. It is possible to modify the document *in-place* using
the option `--in-place`.

Finally, it is possible to ask the tool to download all the PDFs of the papers
cited in the BibTeX file, using `bibadac setup mybib.bib`. 
The command is called `setup` because it
"sets up" the reading environment, and depending on the options, it can only
use documents that you already have on your computer (avoiding using an
internet connection).

## Installation

The tool is not yet available on `crates.io`, but you can install it from the
source code. You will need to have `rust` installed on your computer. You can
install it by running the following command:

```bash
cargo install --git
```

This will install the `bibadac` binary in your `~/.cargo/bin` folder. Make sure
that this folder is in your `PATH` environment variable, or move the binary to
a folder that is in your `PATH`.

## Contributing

Any contribution is welcomed.

[BibTeX]: https://en.wikipedia.org/wiki/BibTeX

