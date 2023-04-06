use std::{ffi::OsStr, fs::OpenOptions, io::Write, path::PathBuf, str::FromStr, vec};

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

        let summary = Summary {
            title: Option::None,
            prefix_chapters: vec![],
            numbered_chapters: generate_chapters(
                &ctx.root.join(&ctx.config.book.src),
                &SectionNumber::default(),
            ),
            suffix_chapters: vec![],
        };

        Ok(MDBook::load_with_config_and_summary(&ctx.root, ctx.config.clone(), summary)?.book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer != "not-supported"
    }
}

fn generate_chapters(path: &PathBuf, section: &SectionNumber) -> Vec<SummaryItem> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("mdbook.log")
        .unwrap();
    writeln!(file, "{:?}", path).unwrap();

    std::fs::read_dir(path)
        .unwrap()
        .into_iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            let entry = entry.ok().unwrap();
            let path = entry.path();
            let entryname = path.file_name().unwrap().to_str().unwrap().to_owned();

            let mut section = section.clone();
            section.push((i as u32) + 1);

            if entry.file_type().unwrap().is_file() {
                // Only use .md files
                if entry.path().extension()? != OsStr::new("md") {
                    return None;
                }
                // README.md files are used by chapter headings
                if entryname.as_str() == "README.md" {
                    return None;
                }

                let link = Link {
                    name: entryname,
                    location: Some(path.clone()),
                    nested_items: vec![],
                    number: Some(section),
                };
                return Some(SummaryItem::Link(link));
            }

            if entry.file_type().unwrap().is_dir() {
                let mut chapter_readme = path.clone();
                chapter_readme.push(PathBuf::from_str("README.md").unwrap());

                if !chapter_readme.exists() {
                    panic!("Missing chapter file: {:?}", chapter_readme);
                }

                let link = Link {
                    name: entryname,
                    location: Some(chapter_readme),
                    nested_items: generate_chapters(&path, &section),
                    number: Some(section),
                };
                return Some(SummaryItem::Link(link));
            }

            Option::None
        })
        .collect()
}
