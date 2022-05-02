use std::fs;
use std::path::{Path, PathBuf};

use self::evtx::{Evtx, Parser as EvtxParser};

pub mod evtx;

pub enum Document {
    Evtx(Evtx),
}

pub struct Documents<'a> {
    iterator: Box<dyn Iterator<Item = crate::Result<Document>> + 'a>,
}

impl<'a> Iterator for Documents<'a> {
    type Item = crate::Result<Document>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}

pub struct Unknown;
impl Iterator for Unknown {
    type Item = crate::Result<Document>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub enum Parser {
    Evtx(EvtxParser),
    Unknown,
}

pub struct Reader {
    parser: Parser,
}

impl Reader {
    pub fn load(file: &Path, load_unknown: bool, skip_errors: bool) -> crate::Result<Self> {
        // NOTE: We don't want to use libmagic because then we have to include databases etc... So
        // for now we assume that the file extensions are correct!
        match file.extension().and_then(|e| e.to_str()) {
            Some(extension) => match extension {
                "evtx" => Ok(Self {
                    parser: Parser::Evtx(EvtxParser::load(file)?),
                }),
                _ => {
                    if load_unknown {
                        if skip_errors {
                            cs_eyellowln!("file type is not currently supported - {}", extension);
                            Ok(Self {
                                parser: Parser::Unknown,
                            })
                        } else {
                            anyhow::bail!("file type is not currently supported - {}", extension)
                        }
                    } else {
                        Ok(Self {
                            parser: Parser::Unknown,
                        })
                    }
                }
            },
            None => {
                if load_unknown {
                    if let Ok(parser) = EvtxParser::load(file) {
                        return Ok(Self {
                            parser: Parser::Evtx(parser),
                        });
                    }
                    if skip_errors {
                        cs_eyellowln!("file type is not known");

                        Ok(Self {
                            parser: Parser::Unknown,
                        })
                    } else {
                        anyhow::bail!("file type is not known")
                    }
                } else {
                    Ok(Self {
                        parser: Parser::Unknown,
                    })
                }
            }
        }
    }

    pub fn documents<'a>(&'a mut self) -> Documents<'a> {
        let iterator = match &mut self.parser {
            Parser::Evtx(parser) => Box::new(
                parser
                    .parse()
                    .map(|r| r.map(|d| Document::Evtx(d)).map_err(|e| e.into())),
            ),
            Parser::Unknown => {
                Box::new(Unknown) as Box<dyn Iterator<Item = crate::Result<Document>> + 'a>
            }
        };
        Documents { iterator }
    }
}

pub fn get_files(path: &PathBuf, extension: &Option<String>) -> crate::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = vec![];
    if path.exists() {
        let metadata = fs::metadata(&path)?;
        if metadata.is_dir() {
            let directory = path.read_dir()?;
            for dir in directory {
                files.extend(get_files(&dir?.path(), &extension)?);
            }
        } else {
            if let Some(extension) = extension {
                if let Some(ext) = path.extension() {
                    if ext == extension.as_str() {
                        files.push(path.to_path_buf());
                    }
                }
            } else {
                files.push(path.to_path_buf());
            }
        }
    } else {
        anyhow::bail!("Invalid input path: {}", path.display());
    }
    Ok(files)
}