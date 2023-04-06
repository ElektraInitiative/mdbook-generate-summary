use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
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
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            (entry, name)
        })
        .filter(|(entry, name)| {
            if section.is_none() && name == "SUMMARY" {
                // Do not keep 'SUMMARY.md' when in src file as we are the ones generating it
                return false;
            }
            entry.file_type().unwrap().is_dir() || name != &config.chapter_file_name
        })
        .enumerate()
        .map(|(i, (entry, name))| {
            let mut section = section.cloned().unwrap_or_default();
            section.push((i + 1) as u32);

            let path = entry.path();
            let link = if entry.file_type().unwrap().is_file() {
                summary_item_for_file(path, name, config, section)
            } else {
                summary_item_for_directory(path, name, config, section)
            };

            SummaryItem::Link(link)
        })
        .collect()
}

/// Creates a summary item for the file.
fn summary_item_for_file(
    path: PathBuf,
    name: String,
    config: &Config,
    section: SectionNumber,
) -> Link {
    Link {
        name: if config.get_chapter_name_from_file {
            get_chapter_name_from_file(&path)
        } else {
            name
        },
        location: Some(path),
        nested_items: vec![],
        number: Some(section),
    }
}

/// Creates a summary item for the directory. Use the [`config.chapter_file_name`] as content.
fn summary_item_for_directory(
    path: PathBuf,
    name: String,
    config: &Config,
    section: SectionNumber,
) -> Link {
    let mut chapter_readme = path.clone();
    chapter_readme.push(PathBuf::from_str(&format!("{}.md", config.chapter_file_name)).unwrap());

    if !chapter_readme.exists() {
        if config.create_missing_chapter_files {
            let mut file = File::create(&chapter_readme).unwrap();
            write!(file, "# {}.md", config.chapter_file_name).unwrap();
        } else {
            panic!("Missing chapter file: {:?}", chapter_readme);
        }
    }

    Link {
        name,
        location: Some(chapter_readme),
        nested_items: generate_chapters(&path, Some(&section), config),
        number: Some(section),
    }
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

/// If the first line of the file looks like '# <header>' use the header as the chapter name.
/// Otherwise use the filename.
fn get_chapter_name_from_file(path: &PathBuf) -> String {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    let mut page_name = String::new();
    reader.read_line(&mut page_name).unwrap();

    match page_name.strip_prefix("# ") {
        Some(stripped) => stripped.to_owned(),
        None => page_name,
    }
}
