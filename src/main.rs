/// This is the `bibadac` program to handle bibliographic data
/// written using the BibTeX/BibLaTeX formats.
///
/// The program contains 3 subcommands:
/// - `check`: check the validity of a BibTeX/BibLaTeX file
/// - `format`: format a BibTeX/BibLaTeX file
/// - `setup`: download pdfs that are mentionned in the file
///
use clap::{Args, Parser, Subcommand};
use std::io::Read;

use colored::Colorize;

use bibadac::arxiv_identifiers::ArxivId;
use bibadac::bibdb::LocalBibDb;
use bibadac::bibtex::BibFile;
use bibadac::format::{write_bibfile, FormatOptions};
use bibadac::linter::{Lint, LintMessage, LinterState};

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

fn windowed(s: &str, start: usize, end: usize, window_size: usize) -> (&str, &str, &str) {
    let new_start_attempt = start.saturating_sub(window_size);
    let new_end_attempt = end + window_size;
    let new_start = s
        .char_indices()
        .nth(new_start_attempt)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let new_end = s
        .char_indices()
        .nth(new_end_attempt)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    (&s[new_start..start], &s[start..end], &s[end..new_end])
}

#[derive(Debug, Clone, Parser)]
#[command(name = "bibadac")]
#[command(about = "A tool to handle bibliographic data")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCommand {
    #[command(
        about = "Check the validity of a BibTeX/BibLaTeX file",
        arg_required_else_help = true
    )]
    Check(CheckArgs),
    #[command(about = "Format a BibTeX/BibLaTeX file", arg_required_else_help = true)]
    Format(FormatArgs),
    #[command(
        about = "Download pdfs that are mentionned in the file",
        arg_required_else_help = true
    )]
    Setup(SetupArgs),
}

#[derive(Debug, Clone, Args)]
struct FileArgs {
    #[arg(
        short,
        long,
        help = "Read BibTeX from stdin, set to true in case no bibfiles are provided"
    )]
    stdin: bool,
    /// BibTeX/BibLaTeX files to read
    bib: Vec<std::path::PathBuf>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Config {
    check: CheckConfig,
    format: FormatConfig,
    setup: SetupConfig,
}

#[derive(Debug, Default, Clone, Args, Serialize, Deserialize)]
struct CheckConfig {
    #[arg(short, long, help = "Show only important errors")]
    concise: bool,
    #[arg(short, long, help = "Hide location of errors to symplify output")]
    executive_summary: bool,
    #[arg(short, long, help = "Output the errors in JSON format")]
    to_json: bool,
    #[arg(short, long, help = "Use a helper bibfile to check semantic errors")]
    file_db: Option<std::path::PathBuf>,
}

#[derive(Debug, Default, Clone, Args, Serialize, Deserialize)]
struct FormatConfig {
    #[arg(short, long, help = "Create a new file with the formatted content")]
    to_file: bool,
    #[arg(short, long, help = "Update the files *in place* (dangerous)")]
    in_place: bool,
    #[arg(short, long, help = "Autocomplete entries using an existing bibfile")]
    file_db: Option<std::path::PathBuf>,
    #[arg(short, long, help = "Remove the corresponding fields from the output")]
    remove_field: Vec<String>,
    #[arg(short, long, help = "Only keep the corresponding fields in the output")]
    keep_field: Vec<String>,
    #[arg(
        short,
        long,
        help = "Only keep entries containing one of the following fields"
    )]
    entry_field: Vec<String>,
    #[arg(short = 'l', long, help = "Order the fields alphabetically")]
    sort_fields: bool,
    #[arg(short = 'g', long, help = "Order the entries alphabetically")]
    sort_entries: bool,
}

#[derive(Debug, Clone, Args, Default, Serialize, Deserialize)]
struct SetupConfig {
    #[arg(short, long, help = "Save bibentries to a file")]
    to_file: Option<std::path::PathBuf>,
    #[arg(short = 'o', long, help = "Print the bibentries")]
    no_output: bool,
    #[arg(short, long, help = "Download the pdfs")]
    documents: bool,
    #[arg(short, long, help = "Directory to save the pdfs")]
    working_directory: Option<std::path::PathBuf>,
    #[arg(short = 'p', long, help = "Do not show progress of the downloads")]
    no_progress: bool,
    #[arg(short = 'm', long, help = "Be polite when talking to CrossRef APIs")]
    polite_email: Option<String>,
    #[arg(short = 'a', long, help = "Directly import from arxiv")]
    arxiv: Vec<String>,
    #[arg(short = 'd', long, help = "Directly import from doi")]
    doi: Vec<String>,
}

#[derive(Debug, Clone, Args)]
struct CheckArgs {
    #[clap(flatten)]
    files: FileArgs,
    #[clap(flatten)]
    config: CheckConfig,
}

#[derive(Debug, Clone, Args)]
struct FormatArgs {
    #[clap(flatten)]
    files: FileArgs,
    #[clap(flatten)]
    config: FormatConfig,
}

#[derive(Debug, Clone, Args)]
struct SetupArgs {
    #[clap(flatten)]
    files: FileArgs,
    #[clap(flatten)]
    config: SetupConfig,
}

#[derive(Debug, Clone)]
struct InputFile {
    name: std::path::PathBuf,
    content: String,
}

trait InputFiles {
    fn list_files(&self) -> Vec<InputFile>;
}

impl InputFiles for FileArgs {
    fn list_files(&self) -> Vec<InputFile> {
        let use_stdin = self.stdin || self.bib.is_empty();
        self.bib
            .iter()
            .filter_map(|name| {
                if !name.exists() {
                    eprintln!("File {:?} does not exist", name);
                    return None;
                }
                let content = std::fs::read_to_string(name).expect("Could not read input file");
                Some(InputFile {
                    name: name.clone(),
                    content,
                })
            })
            .chain(if use_stdin {
                let mut content = String::new();
                std::io::stdin()
                    .read_to_string(&mut content)
                    .expect("Could not read stdin");
                vec![InputFile {
                    name: "stdin".into(),
                    content,
                }]
            } else {
                vec![]
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportEntry {
    file: String,
    errors: Vec<JsonReportLint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportLoc {
    line: usize,
    column: usize,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportLint {
    msg: LintMessage,
    loc: Vec<JsonReportLoc>,
}

fn print_json_lints(lints: Vec<(&InputFile, &BibFile, Vec<Lint>)>) {
    let mut out = std::io::stdout();
    let json_report = lints
        .iter()
        .map(|(bib, _, lints)| JsonReportEntry {
            file: bib.name.to_string_lossy().to_string(),
            errors: lints
                .iter()
                .map(|l| JsonReportLint {
                    msg: l.msg.clone(),
                    loc: l
                        .loc
                        .iter()
                        .map(|n| JsonReportLoc {
                            line: n.start_position().row + 1,
                            column: n.start_position().column + 1,
                            start_byte: n.start_byte(),
                            end_byte: n.end_byte(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(&mut out, &json_report).expect("Could not write json report");
}

fn print_bib_lint(bibtex: &BibFile, bib: &InputFile, l: &Lint) {
    println!(
        "{}\n<{:?}:L{}:C{}>\n{:?}",
        "Error".red(),
        bib.name,
        l.loc[0].start_position().row + 1,
        l.loc[0].start_position().column + 1,
        l.msg
    );
    println!(
        "{}",
        l.loc
            .iter()
            .map(|n| {
                let s = bibtex.get_slice(*n);
                s.lines()
                    .take(3)
                    .zip(1..)
                    .map(|(l, i)| format!("{:>4}| {}", i + n.start_position().row, l))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n...\n")
            .blue()
    );
    if let LintMessage::SyntaxError(_) = l.msg {
        // print a bit before and a bit after
        // using colors to highlight the error
        let start = l.loc[0].start_byte();
        let end = l.loc[0].end_byte();
        let (before, error, after) = windowed(&bibtex.content, start, end, 20);

        print!("{}", before);
        print!("{}", error.red());
        print!("{}", after);
    }
    println!();
}

fn main() {
    let args = Cli::parse();

    match args.command {
        SubCommand::Check(cargs) => {
            use std::collections::HashSet;

            let mut linter = LinterState::default();

            let mut start_bib = String::new();
            if let Some(path) = cargs.config.file_db {
                start_bib =
                    std::fs::read_to_string(path).expect("Could not read the helper bibfile");
            }

            let bibtex = BibFile::new(&start_bib);
            let eprints = bibtex
                .list_entries()
                .flat_map(|entry| {
                    entry
                        .fields
                        .into_iter()
                        .filter(|f| bibtex.get_slice(f.name) == "eprint")
                        .map(|f| bibtex.get_braceless_slice(f.value))
                        .filter_map(|e| ArxivId::try_from(e).ok())
                })
                .collect::<HashSet<_>>();
            for eprint in eprints {
                if let Some(v) = eprint.version {
                    linter
                        .arxiv_latest
                        .entry(eprint.id)
                        .and_modify(|u| *u = std::cmp::max(*u, v))
                        .or_insert(v);
                }
            }

            let files = cargs.files.list_files();
            let inputs = files
                .iter()
                .map(|f| {
                    let bibtex = BibFile::new(&f.content);
                    (f, bibtex)
                })
                .collect::<Vec<_>>();
            let mut lints = vec![];
            for (bib, tex) in inputs.iter() {
                if !cargs.config.concise {
                    lints.push((
                        *bib,
                        tex,
                        linter.lint_file(&tex, tex.list_entries().collect()),
                    ));
                } else {
                    lints.push((
                        *bib,
                        tex,
                        linter
                            .lint_file(&tex, tex.list_entries().collect())
                            .into_iter()
                            .filter(|l| l.msg.is_crucial())
                            .collect(),
                    ));
                }
            }

            if cargs.config.to_json {
                print_json_lints(lints);
                return;
            }

            // 1. print the number of errors for every input
            for (bib, _, lints) in lints.iter() {
                if lints.len() == 0 {
                    println!("{} \t\t {:?}", "[OK]".green(), bib.name);
                } else {
                    let err = if lints.len() > 1 { "errors" } else { "error" };
                    println!("{} {} {} \t {:?}", "[KO]".red(), lints.len(), err, bib.name);
                }
            }

            // 2. do not print the errors for each file if verbose
            if cargs.config.executive_summary {
                return;
            }

            for (bib, bibtex, lints) in lints.iter() {
                for l in lints {
                    print_bib_lint(bibtex, bib, l);
                }
            }
        }
        SubCommand::Format(cargs) => {
            let mut db = LocalBibDb::new();
            if let Some(path) = cargs.config.file_db {
                let start_bib =
                    std::fs::read_to_string(path).expect("Could not read the helper bibfile");
                db = db.import_bibtex(&start_bib);
            }

            let inputs = cargs.files.list_files();

            let mut format_options = FormatOptions::new(&mut db);
            if !cargs.config.remove_field.is_empty() {
                format_options.blacklist = Some(cargs.config.remove_field);
            }
            if !cargs.config.keep_field.is_empty() {
                format_options.whitelist = Some(cargs.config.keep_field);
            }
            if !cargs.config.entry_field.is_empty() {
                format_options.field_filter = Some(cargs.config.entry_field);
            }

            format_options.sort_fields = cargs.config.sort_fields;
            format_options.sort_entries = cargs.config.sort_entries;

            for bib in inputs {
                let bibtex = BibFile::new(&bib.content);
                let max_field_length = bibtex
                    .list_entries()
                    .map(|entry| {
                        entry
                            .fields
                            .iter()
                            .map(|field| bibtex.get_slice(field.name).len())
                            .max()
                            .unwrap_or(0)
                    })
                    .max()
                    .unwrap_or(0);
                format_options.min_field_length = Some(max_field_length);
                use std::io::Write;
                if cargs.config.to_file {
                    let newpath = bib.name.with_extension("new.bib");
                    let mut out =
                        std::fs::File::create(newpath).expect("Could not create the output file");
                    write!(
                        out,
                        "{}",
                        bibadac::format::BibFormat {
                            bib: &bibtex,
                            options: &format_options
                        }
                    )
                    .expect("Could not write to the output file");
                } else if cargs.config.in_place {
                    let mut out =
                        std::fs::File::create(&bib.name).expect("Could not create the output file");
                    write!(
                        out,
                        "{}",
                        bibadac::format::BibFormat {
                            bib: &bibtex,
                            options: &format_options
                        }
                    )
                    .expect("Could not write to the output file");
                } else {
                    write!(
                        std::io::stdout(),
                        "{}",
                        bibadac::format::BibFormat {
                            bib: &bibtex,
                            options: &format_options
                        }
                    )
                    .expect("Could not write to the output file");
                }
            }
        }
        SubCommand::Setup(cargs) => {
            use bibadac::setup::SetupConfig;
            let files = cargs.files.list_files();

            let mut config = SetupConfig::default();
            config.progress = !cargs.config.no_progress;
            config.download_pdf = cargs.config.documents;
            config.polite_email = cargs.config.polite_email;
            if let Some(path) = &cargs.config.working_directory {
                config.working_directory = path.clone();
            } else {
                config.working_directory =
                    std::env::current_dir().expect("Could not get the current directory");
            }

            if let Some(database) = &cargs.config.to_file {
                config.import_bibfile(database);
            }

            let mut dois: HashSet<String> = HashSet::new();
            let mut eprints: HashSet<String> = HashSet::new();
            let mut sha256s: HashSet<String> = HashSet::new();

            for bib in files {
                let bibtex = BibFile::new(&bib.content);
                for entry in bibtex.list_entries() {
                    for field in entry.fields.iter() {
                        let key = bibtex.get_slice(field.name);
                        let value = bibtex.get_braceless_slice(field.value);
                        match key {
                            "doi" => {
                                dois.insert(value.to_string());
                            }
                            "eprint" => {
                                eprints.insert(value.to_string());
                                // add the "non pinned" version of the eprint
                                if let Ok(e) = ArxivId::try_from(value) {
                                    eprints.insert(e.id.to_string());
                                }
                            }
                            "sha256" => {
                                sha256s.insert(value.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }

            for arxiv in &cargs.config.arxiv {
                eprints.insert(arxiv.to_string());
            }

            for doi in &cargs.config.doi {
                dois.insert(doi.to_string());
            }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("Unable to create the asynchronous runtime");

            rt.block_on(async {
                let response = config.run(dois, eprints, sha256s).await;
                if !cargs.config.no_output {
                    for (_, result) in response.entries.iter() {
                        if let Some(entry) = result {
                            println!("{}", entry);
                        }
                    }
                    for (_, result) in response.pdfs.iter() {
                        if let Some(pdf) = result {
                            println!("{}", pdf.entry);
                        }
                    }
                }
                if let Some(path) = &cargs.config.to_file {
                    use std::io::Write;
                    // create the file if it does not exist already
                    // otherwise *append* to it
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(path)
                        .expect("Could not open the output file");
                    let mut file = std::io::BufWriter::new(file);
                    for (_, result) in response.entries.iter() {
                        if let Some(entry) = result {
                            writeln!(file, "{}", entry)
                                .expect("Could not write to the output file");
                        }
                    }
                    for (_, result) in response.pdfs.iter() {
                        if let Some(pdf) = result {
                            writeln!(file, "{}", pdf.entry)
                                .expect("Could not write to the output file");
                        }
                    }
                }
                if !cargs.config.no_progress {
                    for (key, res) in response.entries.iter() {
                        if res.is_none() {
                            println!("[ERR] Could not find entry for {}", key);
                        }
                    }
                    for (key, res) in response.pdfs.iter() {
                        if res.is_none() {
                            println!("{} Could not find pdf for {}", "[ERR]".red(), key.yellow());
                        }
                    }
                }
            });
        }
    }
}
