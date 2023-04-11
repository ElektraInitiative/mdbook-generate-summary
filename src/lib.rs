use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    str::FromStr,
    vec,
};

use anyhow::Error;
use mdbook::{
    book::{Book, Link, SectionNumber, Summary, SummaryItem},
    preprocess::{Preprocessor, PreprocessorContext},
    MDBook,
};

/// Possible configuration options when running the preprocessor
struct Config {
    /// Use the first line of the file and parse '# <chapter_name>' if set. Defaults to false.
    get_chapter_name_from_file: bool,
    /// The file to use for chapters with children. Defaults to 'README'.
    /// Do not include the file extension as it will be '.md' anyways.
    chapter_file_name: String,
    /// Creates empty file with name chapter_file_name if it is missing in a directory. Defaults to
    /// false.
    /// When false the preprocessor panics if the file is <chapter_file_name>.md is missing in a
    /// directory.
    create_missing_chapter_files: bool,
    /// If a create_missing_chapter_files is false, but the file is missing the implementations
    /// panics by default.
    /// Set this to true to instead use ignore the missing file.
    ignore_missing_chapter_files: bool,
}

impl From<&toml::map::Map<String, toml::value::Value>> for Config {
    fn from(value: &toml::map::Map<String, toml::value::Value>) -> Self {
        Self {
            get_chapter_name_from_file: value
                .get("get_chapter_name_from_file")
                .map_or(false, |val| val.as_bool().unwrap()),
            chapter_file_name: value
                .get("chapter_file_name")
                .map_or("README".to_owned(), |val| val.as_str().unwrap().to_owned()),
            create_missing_chapter_files: value
                .get("create_missing_chapter_files")
                .map_or(false, |val| val.as_bool().unwrap()),
            ignore_missing_chapter_files: value
                .get("ignore_missing_chapter_files")
                .map_or(false, |val| val.as_bool().unwrap()),
        }
    }
}

#[derive(Debug, Default)]
pub struct GenerateSummary;

impl GenerateSummary {
    pub fn new() -> GenerateSummary {
        GenerateSummary
    }
}

impl Preprocessor for GenerateSummary {
    fn name(&self) -> &str {
        "generate-summary"
    }

    fn run(&self, ctx: &PreprocessorContext, _: Book) -> Result<Book, Error> {
        let config = Config::from(ctx.config.get_preprocessor(self.name()).unwrap());

        let book_dir = &ctx.root.join(&ctx.config.book.src);

        // Create summary using books src directory
        let summary = Summary {
            title: Option::None,
            prefix_chapters: vec![],
            numbered_chapters: generate_chapters(book_dir, Option::None, &config),
            suffix_chapters: vec![],
        };

        Ok(MDBook::load_with_config_and_summary(&ctx.root, ctx.config.clone(), summary)?.book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

/// Create summary items out of the provided directory. If the section is `None` it means we are in
/// the src dir.
fn generate_chapters(
    dir_path: &PathBuf,
    section: Option<&SectionNumber>,
    config: &Config,
) -> Vec<SummaryItem> {
    let mut entries = get_markdown_files_and_directories(dir_path);

    // Sort by filename
    entries.sort_by_key(|a| a.file_name());

    entries
        .into_iter()
        .map(|entry| {
            let path = entry.path();
            let filename = path.file_stem().unwrap().to_str().unwrap().to_owned();
            (entry, filename)
        })
        .filter(|(entry, filename)| {
            if section.is_none() && filename == "SUMMARY" {
                // Do not keep 'SUMMARY.md' when in src file as we are the ones generating it
                return false;
            }
            entry.file_type().unwrap().is_dir() || filename != &config.chapter_file_name
        })
        .enumerate()
        .map(|(i, (entry, filename))| {
            let mut section = section.cloned().unwrap_or_default();
            section.push((i + 1) as u32);

            let path = entry.path();
            let (path_to_chapter_content, nested_items) = if entry.file_type().unwrap().is_file() {
                (Some(path), vec![])
            } else {
                (
                    get_path_to_directory_content(&path, config),
                    generate_chapters(&path, Some(&section), config),
                )
            };

            let link = Link {
                name: get_chapter_name(&path_to_chapter_content, config, filename),
                location: path_to_chapter_content,
                nested_items,
                number: Some(section),
            };
            SummaryItem::Link(link)
        })
        .collect()
}

/// Build the path to the file to be used as the directory's content.
/// If `config.create_missing_chapter_files` is true and the chapter file is missing create it.
/// If `config.ignore_missing_chapter_files` is true and the chapter file is missing return [`Option::None`].
///
/// # Panics
/// If the content file is missing and both `config.create_missing_chapter_files` and `config.ignore_missing_chapter_files` are false.
fn get_path_to_directory_content(path: &Path, config: &Config) -> Option<PathBuf> {
    let mut chapter_content = path.to_path_buf();
    chapter_content.push(PathBuf::from_str(&format!("{}.md", config.chapter_file_name)).unwrap());

    if !chapter_content.exists() {
        if config.create_missing_chapter_files {
            let mut file = File::create(&chapter_content).unwrap();
            write!(file, "# {}.md", config.chapter_file_name).unwrap();
        } else if config.ignore_missing_chapter_files {
            return None;
        } else {
            panic!("Missing chapter file: {:?}", chapter_content);
        }
    }
    Some(chapter_content)
}

/// Get all markdown files and directories in the specified directory. Ignore all other files.
fn get_markdown_files_and_directories(dir_path: &PathBuf) -> Vec<std::fs::DirEntry> {
    std::fs::read_dir(dir_path)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            let file_type = entry.file_type().unwrap();

            if file_type.is_file() {
                let path = entry.path();
                let extension = path.extension();
                // Only use .md files
                extension.is_some() && extension.unwrap() == OsStr::new("md")
            } else {
                // or directories
                file_type.is_dir()
            }
        })
        .collect()
}

/// If the chapter file exists, `config.get_chapter_name_from_file` is true and the first line of the file looks like '# <header>' use header as the chapter name.
/// Otherwise return the filename.
fn get_chapter_name(path: &Option<PathBuf>, config: &Config, filename: String) -> String {
    match path {
        Some(ref path) if config.get_chapter_name_from_file => {
            let file = File::open(path).unwrap();
            let mut reader = BufReader::new(file);

            let mut first_line = String::new();
            reader.read_line(&mut first_line).unwrap();

            first_line
                .strip_prefix("# ")
                .map_or(filename, str::to_owned)
        }
        _ => filename,
    }
}
