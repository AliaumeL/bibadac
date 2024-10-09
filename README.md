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

Note that all of these actions will take place in the *command line*. The goal
is not to have a graphical tool that people will use to manage their
bibliography, but a simple tool that can be integrated to any text editor or
continuous integration system to make their preferred tasks *automatic* (hence
less error-prone).

## Usage

The basic command is to check the validity of a BibTeX file.

```bash
bibadac check mybib.bib
```

This will print a report of the `mybib.bib` file, with all the errors and
warnings that were found. The exit code will be 0 if no error was found, 1 for
errors, and 101 in case of internal errors.

It is possible to ask the program to generate a report in a different format
and to specify an output file instead of printing it to the standard output.

```bash
bibadac check mybib.bib --format json --output report.json
```

In order to format a BibTeX file, one can use the following command:

```bash
bibadac format mybib.bib
```

Note that by default, the formatting will be done in place, but it is possible
to specify an output file. This is typically a good idea if you are afraid of
losing your data when tinkering with the tool.

```bash
bibadac format mybib.bib --output mybib_formatted.bib
```

Finally, it is possible to ask the tool to download all the PDFs of the papers
cited in the BibTeX file. This is done with the following command, which places
them in the current folder by default. The command is called `setup` because it
"sets up" the reading environment, and depending on the options, it can only
use documents that you already have on your computer (avoiding using an
internet connection).

```bash
bibadac setup mybib.bib
```


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


## Configuration 

You may want to configure the tool to your needs. Typically, 
there are several things that you may want to change:

1. The *strictness* of the linting rules, i.e. what is considered an error and
   what is considered a warning.
2. The *ability to access internet*, when trying to download papers,
   autocomplete entries, or check version numbers.
3. The *formatting rules* that you want to apply to your BibTeX file
   (e.g. removing unused fields, sorting entries, etc.)

This can be done directly using command line options, or by creating
a configuration file in the current directory call `bibadac.toml`.

This part of the documentation is not yet written.

[BibTeX]: https://en.wikipedia.org/wiki/BibTeX
