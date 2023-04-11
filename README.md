# mdbook-generate-summary
A [preprocessor](https://rust-lang.github.io/mdBook/for_developers/preprocessors.html) for [mdbook](https://github.com/rust-lang/mdBook) to automatically generate the summary from the source directory.

## Setup
To install run:
```
cargo install --git https://github.com/kitzbergerg/mdbook-generate-summary mdbook-generate-summary
```

Add the following to the end of your book.toml:
```
[preprocessor.generate-summary]
```

Now run `mdbook serve --open`. Note that the `SUMMARY.md` file is required for mdbook to start, its contents however are ignored.

## Configuration
Using the default configuration this preprocessor will not make changes to your filesystem (Note however that mdbook might make changes).

| Option                        | Type    | Description                                                                                                                                                   | Default Value |
|-------------------------------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------|
| get_chapter_name_from_file    | bool    | Use the first line of the file and parse '# \<chapter_name>' if set.                                                                                          | false         |
| chapter_file_name             | String  | The file to use for chapters with children. Do not include the file extension as it will be '.md' anyways.                                                    | "README"      |
| create_missing_chapter_files  | bool    | Creates empty files with name chapter_file_name if it is missing in a directory.                                                                              | false         |
| ignore_missing_chapter_files  | bool    | If create_missing_chapter_files is false, but the file is missing, the implementation panics by default. Set this to true to instead ignore the missing file. | false         |

### Example:
```
[preprocessor.generate-summary]
get_chapter_name_from_file = true
ignore_missing_chapter_files = true
```
