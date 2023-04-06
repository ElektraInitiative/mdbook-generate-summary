use std::{ffi::OsStr, fs::OpenOptions, io::ErrorKind, path::PathBuf, str::FromStr, vec};

use anyhow::Error;
use mdbook::{
    book::{Book, Link, SectionNumber, Summary, SummaryItem},
    preprocess::{Preprocessor, PreprocessorContext},
    MDBook,
};

/// A no-op preprocessor.
pub struct GenerateSummary;

impl GenerateSummary {
    pub fn new() -> GenerateSummary {
        GenerateSummary
    }
}

impl Preprocessor for GenerateSummary {
    fn name(&self) -> &str {
        "generate-summary-preprocessor"
    }

    fn run(&self, ctx: &PreprocessorContext, _: Book) -> Result<Book, Error> {
        // In testing we want to tell the preprocessor to blow up by setting a
        // particular config value
        if let Some(nop_cfg) = ctx.config.get_preprocessor(self.name()) {
            if nop_cfg.contains_key("blow-up") {
                anyhow::bail!("Boom!!1!");
            }
        }

        let book_dir = &ctx.root.join(&ctx.config.book.src);

        // Try to delete SUMMARY.md. Panics if file exists, but could not be deleted
        let path_to_summary = book_dir.join("SUMMARY.md");
        if let Err(e) = std::fs::remove_file(&path_to_summary) {
            if e.kind() != ErrorKind::NotFound {
                panic!("{}", e);
            }
        }

        // Create summary using books src directory
        let summary = Summary {
            title: Option::None,
            prefix_chapters: vec![],
            numbered_chapters: generate_chapters(book_dir, &SectionNumber::default()),
            suffix_chapters: vec![],
        };

        // Create empty SUMMARY.md
        let _ = OpenOptions::new().create(true).open(&path_to_summary);

        Ok(MDBook::load_with_config_and_summary(&ctx.root, ctx.config.clone(), summary)?.book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn generate_chapters(dir_path: &PathBuf, section: &SectionNumber) -> Vec<SummaryItem> {
    let mut entries = std::fs::read_dir(dir_path)
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
        .collect::<Vec<_>>();

    // Sort by filename
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            let path = entry.path();
            let entryname = path.file_name().unwrap().to_str().unwrap().to_owned();

            let mut section = section.clone();
            section.push(i as u32);

            let (location, nested_items) = if entry.file_type().unwrap().is_file() {
                // README.md files are used by chapter headings
                if entryname.as_str() == "README.md" {
                    return None;
                }

                (path.clone(), vec![])
            } else {
                let mut chapter_readme = path.clone();
                chapter_readme.push(PathBuf::from_str("README.md").unwrap());

                if !chapter_readme.exists() {
                    panic!("Missing chapter file: {:?}", chapter_readme);
                }

                (chapter_readme, generate_chapters(&path, &section))
            };

            let link = Link {
                name: entryname,
                location: Some(location),
                nested_items,
                number: Some(section),
            };
            Some(SummaryItem::Link(link))
        })
        .collect()
}
