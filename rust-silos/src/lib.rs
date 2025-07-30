// Re-export phf_map macro for consumers of rust-silos
pub use phf::phf_map;
pub use phf;
use std::hash::Hash;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;


/// Error type for file and silo operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to decode file contents: {source}")]
    DecodeError {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error("File not found")]
    NotFound,
    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}


/// Metadata and contents for an embedded file.
#[derive(Debug)]
pub struct EmbedEntry {
    pub path: &'static str,
    pub contents: &'static [u8],
    pub size: usize,
    pub modified: u64,
}

/// Handle to an embedded file entry.
#[derive(Copy, Clone, Debug)]
struct EmbedFile {
    inner: &'static EmbedEntry,
}

impl EmbedFile {
    /// Returns the relative path of the embedded file.
    pub fn path(&self) -> &Path {
        Path::new(self.inner.path)
    }
}

/// Internal enum for file variants (embedded or dynamic).
#[derive(Debug, Clone)]
enum FileKind {
    Embed(EmbedFile),
    Dynamic(DynFile),
}

/// Represents a file, which may be embedded or dynamic.
#[derive(Debug, Clone)]
pub struct File {
    inner: FileKind,
}

impl File {
    /// Returns a reader for the file's contents. May return an error if the file cannot be opened.
    pub fn reader(&self) -> Result<FileReader, Error> {
        match &self.inner {
            FileKind::Embed(embed) => Ok(FileReader::Embed(Cursor::new(embed.inner.contents))),
            FileKind::Dynamic(dyn_file) => Ok(FileReader::Dynamic(std::fs::File::open(
                dyn_file.absolute_path(),
            )?)),
        }
    }

    /// Returns the relative path of the file.
    pub fn path(&self) -> &Path {
        match &self.inner {
            FileKind::Embed(embed) => embed.path(),
            FileKind::Dynamic(dyn_file) => dyn_file.path(),
        }
    }

    /// Returns true if the file is embedded in the binary.
    pub fn is_embedded(&self) -> bool {
        matches!(self.inner, FileKind::Embed(_))
    }

    /// Returns the absolute path if the file is dynamic, or None if embedded.
    pub fn absolute_path(&self) -> Option<&Path> {
        match &self.inner {
            FileKind::Embed(_) => None,
            FileKind::Dynamic(dyn_file) => Some(dyn_file.absolute_path()),
        }
    }

    /// Returns the file extension, if any.
    pub fn extension(&self) -> Option<&str> {
        self.path().extension().and_then(|s| s.to_str())
    }
}

/// Files are equal if their relative paths are equal.
impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

/// Hashes a file by its relative path.
impl Hash for File {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path().hash(state);
    }
}

impl Eq for File {}



/// Represents a set of embedded files and their root.
#[derive(Debug, Clone)]
struct EmbedSilo {
    map: &'static phf::Map<&'static str, EmbedEntry>,
    root: &'static str,
}

impl EmbedSilo {
    /// Create a new EmbedSilo from a PHF map and root path.
    pub const fn new(map: &'static phf::Map<&'static str, EmbedEntry>, root: &'static str) -> Self {
        Self { map, root }
    }

    /// Get an embedded file by its relative path.
    /// Returns None if not found.
    pub fn get_file(&self, path: &str) -> Option<EmbedFile> {
        self.map.get(path).map(|entry| EmbedFile { inner: entry })
    }

    /// Iterate over all embedded files in this silo.
    pub fn iter(&self) -> impl Iterator<Item = File> + '_ {
        self.map.values().map(|entry| File {
            inner: FileKind::Embed(EmbedFile { inner: entry }),
        })
    }
}

/// Represents a file from the filesystem (not embedded).
#[derive(Debug, Clone)]
struct DynFile {
    rel_path: Arc<str>,
    full_path: Arc<str>,
}

impl DynFile {
    /// root is the base directory where the file is located, and path is the relative path to the file.
    /// Create a new DynFile from absolute and relative paths.
    /// Both must be valid UTF-8.
    pub fn new<S: AsRef<str>>(full_path: S, rel_path: S) -> Self {
        Self {
            rel_path: Arc::from(rel_path.as_ref()),
            full_path: Arc::from(full_path.as_ref()),
        }
    }

    /// Returns the relative path of the file.
    pub fn path(&self) -> &Path {
        Path::new(&*self.rel_path)
    }

    /// Returns the absolute path of the file.
    pub fn absolute_path(&self) -> &Path {
        Path::new(&*self.full_path)
    }
}

/// Get a dynamic file by its relative path. Returns None if not found or not a file.
fn get_file_for_root(root: &str, path: &str) -> Option<DynFile> {
    let pathbuff = Path::new(&*root).join(path);
    if pathbuff.is_file() {            
        Some(DynFile::new(Arc::from(pathbuff.to_str()?), Arc::from(path)))
    } else {
        None
    }
}

/// Iterate over all files in the dynamic silo.
fn iter_root(root: &str) -> impl Iterator<Item = File> {
    let root_path = PathBuf::from(&*root);
    walkdir::WalkDir::new(&root_path)
        .into_iter()
        .filter_map(move |entry| {
            let entry = entry.ok()?;
            if entry.file_type().is_file() {
                let relative_path = entry.path().strip_prefix(&root_path).ok()?;
                Some(File {
                    inner: FileKind::Dynamic(DynFile::new(
                        Arc::from(entry.path().to_str()?),
                        Arc::from(relative_path.to_str()?),
                    )),
                })
            } else {
                None
            }
        })
}

/// Represents a set of dynamic (filesystem) files rooted at a directory.
#[derive(Debug, Clone)]
struct DynamicSilo {
    root: Arc<str>,
}

impl DynamicSilo {
    /// Creates a new DynamicSilo from a dynamic root path.
    /// The root path must be valid UTF-8.
    pub fn new(root: &str) -> Self {
        Self { root: Arc::from(root) }
    }

    /// Gets a dynamic file by its relative path.
    /// Returns `None` if the file is not found or is not a valid file.
    pub fn get_file(&self, path: &str) -> Option<DynFile> {
        get_file_for_root(self.root.as_ref(), path)
    }

    /// Iterates over all files in the dynamic silo.
    /// Returns an iterator of `File` objects representing the files.
    pub fn iter(&self) -> impl Iterator<Item = File> {
        iter_root(self.root.as_ref())
    }
}

/// Represents a set of static (filesystem) files rooted at a directory.
/// Static silos are backed by a fixed directory path.
#[derive(Debug, Clone)]
struct StaticSilo {
    root: &'static str,
}

impl StaticSilo {
    /// Creates a new StaticSilo from a static root path.
    pub const fn new(root: &'static str) -> Self {
        Self { root }
    }

    /// Gets a static file by its relative path.
    /// Returns `None` if the file is not found or is not a valid file.
    pub fn get_file(&self, path: &str) -> Option<DynFile> {
        get_file_for_root(self.root, path)
    }

    /// Iterates over all files in the static silo.
    /// Returns an iterator of `File` objects representing the files.
    pub fn iter(&self) -> impl Iterator<Item = File> {
        iter_root(self.root)
    }
}

/// Internal enum for silo variants (embedded or dynamic).
#[derive(Debug, Clone)]
enum InnerSilo {
    Embed(EmbedSilo),
    Static(StaticSilo),
    Dynamic(DynamicSilo),
}

/// Represents a root directory, which may be embedded or dynamic.
#[derive(Debug, Clone)]
pub struct Silo {
    inner: InnerSilo,
}

impl Silo {

    #[doc(hidden)]
    /// Creates a Silo from an embedded PHF map and root path.
    pub const fn from_embedded(phf_map: &'static phf::Map<&'static str, EmbedEntry>, root: &'static str) -> Self {
        Self {
            inner: InnerSilo::Embed(EmbedSilo::new(phf_map, root)),
        }
    }

    #[doc(hidden)]
    /// Creates a Silo from a static path (dynamic root).
    pub const fn from_static(path: &'static str) -> Self {
        Self {
            inner: InnerSilo::Static(StaticSilo::new(path)),
        }
    }

    /// Creates a Silo from a dynamic path (dynamic root).
    pub fn new(path: &str) -> Self {
        Self {
            inner: InnerSilo::Dynamic(DynamicSilo::new(path)),
        }
    }

    /// Converts the Silo to a dynamic Silo if it is currently embedded.
    /// Returns `self` unchanged if the Silo is already dynamic or static.
    pub fn into_dynamic(self) -> Self {
        match self.inner {
            InnerSilo::Embed(emb_silo) => Self::from_static(&*emb_silo.root),
            InnerSilo::Static(_) => self,
            InnerSilo::Dynamic(_) => self,
        }
    }

    /// Automatically converts the Silo to a dynamic directory in debug mode (`cfg!(debug_assertions)`).
    /// In release mode, returns `self` unchanged.
    /// This is a no-op if the Silo is not embedded.
    pub fn auto_dynamic(self) -> Self {
        if cfg!(debug_assertions) {
            self.into_dynamic()
        } else {
            self
        }
    }

    /// Returns `true` if this Silo is dynamic (filesystem-backed).
    pub fn is_dynamic(&self) -> bool {
        matches!(self.inner, InnerSilo::Static(_) | InnerSilo::Dynamic(_))
    }

    /// Returns `true` if this Silo is embedded in the binary.
    pub fn is_embedded(&self) -> bool {
        matches!(self.inner, InnerSilo::Embed(_))
    }

    /// Gets a file by its relative path from this Silo.
    /// Returns `None` if the file is not found.
    pub fn get_file(&self, path: &str) -> Option<File> {
        match &self.inner {
            InnerSilo::Embed(embed) => embed.get_file(path).map(|f| File {
                inner: FileKind::Embed(f),
            }),
            InnerSilo::Static(dyn_silo) => dyn_silo.get_file(path).map(|f| File {
                inner: FileKind::Dynamic(f),
            }),
            InnerSilo::Dynamic(dyn_silo) => dyn_silo.get_file(path).map(|f| File {
                inner: FileKind::Dynamic(f),
            }),
        }
    }

    /// Iterates over all files in this Silo.
    /// Returns a boxed iterator of `File` objects representing the files.
    pub fn iter(&self) -> Box<dyn Iterator<Item = File> + '_> {
        match &self.inner {
            InnerSilo::Embed(embd) => Box::new(embd.iter()),
            InnerSilo::Static(dynm) => Box::new(dynm.iter()),
            InnerSilo::Dynamic(dynm) => Box::new(dynm.iter()),
        }
    }
}



/// Represents a set of root directories, supporting overlay and override semantics.
/// Later directories in the set can override files from earlier ones with the same relative path.
#[derive(Debug, Clone)]
pub struct SiloSet {
    /// The list of root directories, in order of increasing precedence.
    pub silos: Vec<Silo>,
}

impl SiloSet {
    /// Creates a new SiloSet from the given list of directories.
    /// The order of directories determines override precedence.
    /// Create a new SiloSet from a list of Silos. Order determines override precedence.
    pub fn new(dirs: Vec<Silo>) -> Self {
        Self { silos: dirs }
    }


    /// Returns the file with the given name, searching roots in reverse order.
    /// Files in later roots override those in earlier roots if the relative path matches.
    /// Get a file by name, searching Silos in reverse order (highest precedence first).
    pub fn get_file(&self, name: &str) -> Option<File> {
        for silo in self.silos.iter().rev() {
            if let Some(file) = silo.get_file(name) {
                return Some(file);
            }
        }
        None
    }

    /// Recursively walks all files in all root directories.
    /// Files with the same relative path from different roots are all included.
    /// Iterate all files in all Silos, including duplicates.
    pub fn iter(&self) -> impl Iterator<Item = File> + '_ {
        self.silos.iter().rev().flat_map(|silo| silo.iter())
    }

    /// Recursively walks all files, yielding only the highest-precedence file for each relative path.
    /// This implements the override behaviour: later roots take precedence over earlier ones.
    /// Iterate all files, yielding only the highest-precedence file for each path.
    pub fn iter_override(&self) -> impl Iterator<Item = File> + '_ {
        let mut history = std::collections::HashSet::new();
        self.iter().filter(move |file| history.insert(file.clone()) )
    }
}


/// Reader for file contents, either embedded or dynamic.
pub enum FileReader {
    Embed(std::io::Cursor<&'static [u8]>),
    Dynamic(std::fs::File),
}

/// Implements std::io::Read for FileReader.
impl std::io::Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            FileReader::Embed(c) => c.read(buf),
            FileReader::Dynamic(f) => f.read(buf),
        }
    }
}
