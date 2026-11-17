//! Where The AllCraft PDF and Studio live on the web.

use icu_properties::props::Script;
use std::path::{Path, PathBuf};

/// The app's short name.
pub const APP: &str = "pdfcraft";

pub const DISCORD: &str = "https://github.com/thepixopik/pdfcraft/discussions";
pub const WEBSITE: &str = "https://the-allcraft.pages.dev";
pub const APP_PAGE: &str = "https://the-allcraft.pages.dev/pdf/";
pub const GITHUB: &str = "https://github.com/thepixopik/pdfcraft";

/// A link and the registry command that opens it.
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub command: &'static str,
    pub label: &'static str,
    pub url: &'static str,
    /// Lucide icon name.
    pub icon: &'static str,
}

/// In the order they are shown on Home, Help, and About.
pub const LINKS: &[Link] = &[
    Link { command: "help.discord", label: "AllCraft Community", url: DISCORD, icon: "messages-square" },
    Link { command: "help.app_page", label: "The AllCraft PDF Web", url: APP_PAGE, icon: "globe" },
    Link { command: "help.github", label: "PDFCraft on GitHub", url: GITHUB, icon: "code-xml" },
    Link { command: "help.website", label: "The AllCraft Studio", url: WEBSITE, icon: "external-link" },
];

pub fn for_command(id: &str) -> Option<&'static Link> {
    LINKS.iter().find(|l| l.command == id)
}

pub const DOCUMENT_SCHEMES: &[&str] = &["https", "http", "mailto"];
pub const MAX_DOCUMENT_URL: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockedLink {
    Empty,
    TooLong(usize),
    Hidden,
    Malformed,
    Scheme(String),
    MailFile,
    MailEncodedWord,
}

impl std::fmt::Display for BlockedLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "the address is empty"),
            Self::TooLong(n) => write!(f, "the address is {n} characters long"),
            Self::Hidden => write!(f, "the address contains hidden or invisible characters"),
            Self::Malformed => write!(f, "the address isn't a complete web or email address"),
            Self::Scheme(s) => write!(f, "it is a “{s}:” address"),
            Self::MailFile => write!(f, "it asks your email app to attach or insert a file from your computer"),
            Self::MailEncodedWord => write!(f, "it contains encoded text that some email apps turn into other instructions"),
        }
    }
}

fn hides_text(c: char) -> bool {
    (c.is_whitespace() && c != ' ')
        || c.is_control()
        || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

pub fn document_url(raw: &str) -> Result<String, BlockedLink> {
    let url = raw.trim();
    if url.is_empty() {
        return Err(BlockedLink::Empty);
    }
    let len = url.chars().count();
    if len > MAX_DOCUMENT_URL {
        return Err(BlockedLink::TooLong(len));
    }
    if url.chars().any(hides_text) {
        return Err(BlockedLink::Hidden);
    }
    let (scheme, rest) = url.split_once(':').ok_or(BlockedLink::Malformed)?;
    let well_formed = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !well_formed {
        return Err(BlockedLink::Malformed);
    }
    let scheme = scheme.to_ascii_lowercase();
    if !DOCUMENT_SCHEMES.contains(&scheme.as_str()) {
        return Err(BlockedLink::Scheme(scheme));
    }
    let complete = match scheme.as_str() {
        "mailto" => !rest.is_empty(),
        _ => authority(url).is_some_and(|a| !a.contains(' ')) && display_host(url).is_some(),
    };
    if !complete || !url::Url::parse(url).is_ok_and(|u| u.scheme() == scheme) {
        return Err(BlockedLink::Malformed);
    }
    if scheme == "mailto" {
        check_mail_parameters(rest)?;
    }
    Ok(url.to_string())
}

pub const MAX_REMOTE_DOCUMENT_PATH_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteDocumentPathError {
    Empty,
    TooLong,
    Hidden,
    NotLocal(craft_docpath::Refused),
    MissingSourcePath,
    NotPdf,
    NotAFile,
    TooLarge,
    Read(String),
}

impl std::fmt::Display for RemoteDocumentPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the file specification is empty"),
            Self::TooLong => f.write_str("the file specification is too long"),
            Self::Hidden => f.write_str("the file specification contains hidden or control characters"),
            Self::NotLocal(r) => write!(f, "only local document paths are supported: {r}"),
            Self::MissingSourcePath => f.write_str("a relative link needs a source document path"),
            Self::NotPdf => f.write_str("the linked file is not a .pdf file"),
            Self::NotAFile => f.write_str("the linked path is not a regular file"),
            Self::TooLarge => f.write_str("the linked file is too large to open"),
            Self::Read(e) => write!(f, "the linked file couldn't be read: {e}"),
        }
    }
}

pub fn remote_document_path_is_absolute(raw: &str) -> bool {
    Path::new(raw).is_absolute()
}

pub fn resolve_remote_document_path(source: Option<&Path>, raw: &str) -> Result<PathBuf, RemoteDocumentPathError> {
    if raw.is_empty() || raw.trim().is_empty() {
        return Err(RemoteDocumentPathError::Empty);
    }
    if raw.len() > MAX_REMOTE_DOCUMENT_PATH_BYTES {
        return Err(RemoteDocumentPathError::TooLong);
    }
    if raw.chars().any(hides_text) {
        return Err(RemoteDocumentPathError::Hidden);
    }
    if let Some((scheme, _)) = raw.split_once(':') {
        let is_scheme = scheme.len() > 1
            && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if is_scheme {
            return Err(RemoteDocumentPathError::NotLocal(craft_docpath::Refused::Url));
        }
    }
    craft_docpath::resolve_absolute(raw, source).map_err(|r| match r {
        craft_docpath::Refused::Invalid if !source.is_some_and(Path::is_absolute) && craft_docpath::resolve(raw, source).is_ok() => {
            RemoteDocumentPathError::MissingSourcePath
        }
        r => RemoteDocumentPathError::NotLocal(r),
    })
}

pub const MAX_REMOTE_DOCUMENT_BYTES: u64 = 1 << 30;

pub fn read_remote_document(path: &Path) -> Result<Vec<u8>, RemoteDocumentPathError> {
    use std::io::Read;
    let is_pdf = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return Err(RemoteDocumentPathError::NotPdf);
    }
    let meta = std::fs::metadata(path).map_err(|e| RemoteDocumentPathError::Read(e.to_string()))?;
    if !meta.is_file() {
        return Err(RemoteDocumentPathError::NotAFile);
    }
    if meta.len() > MAX_REMOTE_DOCUMENT_BYTES {
        return Err(RemoteDocumentPathError::TooLarge);
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(if cfg!(target_os = "linux") { 0o4000 } else { 0x4 });
    }
    let file = options.open(path).map_err(|e| RemoteDocumentPathError::Read(e.to_string()))?;
    let meta = file.metadata().map_err(|e| RemoteDocumentPathError::Read(e.to_string()))?;
    if !meta.is_file() {
        return Err(RemoteDocumentPathError::NotAFile);
    }
    if meta.len() > MAX_REMOTE_DOCUMENT_BYTES {
        return Err(RemoteDocumentPathError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.take(MAX_REMOTE_DOCUMENT_BYTES.saturating_add(1)).read_to_end(&mut bytes).map_err(|e| RemoteDocumentPathError::Read(e.to_string()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > MAX_REMOTE_DOCUMENT_BYTES) {
        return Err(RemoteDocumentPathError::TooLarge);
    }
    Ok(bytes)
}

const MAIL_FILE_PARAMETERS: &[&str] = &["attach", "insert"];
const MAIL_DECODE_LAYERS: usize = 8;

fn check_mail_parameters(rest: &str) -> Result<(), BlockedLink> {
    let mut text = rest.to_string();
    for _ in 0..MAIL_DECODE_LAYERS {
        check_mail_layer(&text)?;
        let decoded = String::from_utf8_lossy(&percent_decode(text.as_bytes())).into_owned();
        if decoded == text {
            return Ok(());
        }
        text = decoded;
    }
    check_mail_layer(&text)
}

fn check_mail_layer(text: &str) -> Result<(), BlockedLink> {
    if text.find("=?").is_some_and(|i| text.get(i + 2..).is_some_and(|after| after.contains("?="))) {
        return Err(BlockedLink::MailEncodedWord);
    }
    if text.split('\\').skip(1).any(|after| after.starts_with(|c: char| c.is_ascii_digit() || matches!(c, 'x' | 'X'))) {
        return Err(BlockedLink::MailEncodedWord);
    }
    let lower = text.to_ascii_lowercase();
    let is_word = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-');
    let mut previous = None;
    for (i, c) in lower.char_indices() {
        let starts_word = is_word(c) && !previous.is_some_and(is_word);
        previous = Some(c);
        if !starts_word {
            continue;
        }
        let Some(word) = lower.get(i..) else { continue };
        let name_end = word.find(|c: char| !is_word(c)).unwrap_or(word.len());
        let (name, after) = word.split_at(name_end);
        let asks = MAIL_FILE_PARAMETERS.iter().any(|p| name.starts_with(p));
        if asks && after.trim_start_matches([' ', '\t', '+', '\'', '"']).starts_with('=') {
            return Err(BlockedLink::MailFile);
        }
    }
    Ok(())
}

fn percent_decode(s: &[u8]) -> Vec<u8> {
    let hex = |b: Option<&u8>| b.and_then(|b| char::from(*b).to_digit(16)).and_then(|d| u8::try_from(d).ok());
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while let Some(&b) = s.get(i) {
        if b == b'%'
            && let (Some(high), Some(low)) = (hex(s.get(i + 1)), hex(s.get(i + 2)))
        {
            out.push((high << 4) | low);
            i += 3;
        } else {
            out.push(b);
            i += 1;
        }
    }
    out
}

fn authority(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_once(':')?;
    if !(scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")) {
        return None;
    }
    rest.strip_prefix("//")?.split(['/', '\\', '?', '#']).next()
}

pub fn host(url: &str) -> Option<&str> {
    let authority = authority(url)?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if host_port.starts_with('[') {
        host_port.find(']').and_then(|i| host_port.get(..=i)).unwrap_or_default()
    } else {
        host_port.split(':').next().unwrap_or_default()
    };
    (!host.is_empty()).then_some(host)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDisplay {
    pub ascii: String,
    pub international: bool,
    pub mixed_scripts: bool,
}

pub fn display_host(url: &str) -> Option<HostDisplay> {
    host(url)?;
    let parsed = url::Url::parse(url).ok()?;
    let ascii = match parsed.host()? {
        url::Host::Domain(d) => d.to_string(),
        url::Host::Ipv4(a) => a.to_string(),
        url::Host::Ipv6(a) => format!("[{a}]"),
    };
    if !ascii.split('.').any(|label| label.starts_with("xn--")) {
        return Some(HostDisplay { ascii, international: false, mixed_scripts: false });
    }
    let (unicode, valid) = idna::domain_to_unicode(&ascii);
    let mixed_scripts = valid.is_err() || unicode.split('.').any(mixes_scripts);
    Some(HostDisplay { ascii, international: true, mixed_scripts })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Writing {
    Script(Script),
    Japanese,
    Korean,
    Chinese,
}

fn writing_systems(c: char) -> Option<Vec<Writing>> {
    let scripts = icu_properties::script::ScriptWithExtensions::new().get_script_extensions_val(c);
    let mut out = Vec::new();
    for s in scripts.iter() {
        if s == Script::Common || s == Script::Inherited {
            return None;
        }
        out.push(Writing::Script(s));
        match s {
            Script::Han => out.extend([Writing::Japanese, Writing::Korean, Writing::Chinese]),
            Script::Hiragana | Script::Katakana => out.push(Writing::Japanese),
            Script::Hangul => out.push(Writing::Korean),
            Script::Bopomofo => out.push(Writing::Chinese),
            _ => {}
        }
    }
    Some(out)
}

fn shared_writing(label: &str, skip: impl Fn(&[Writing]) -> bool) -> Option<Vec<Writing>> {
    let mut shared: Option<Vec<Writing>> = None;
    for systems in label.chars().filter_map(writing_systems) {
        if skip(&systems) {
            continue;
        }
        shared = Some(match shared {
            None => systems,
            Some(s) => s.into_iter().filter(|w| systems.contains(w)).collect(),
        });
    }
    shared
}

fn mixes_scripts(label: &str) -> bool {
    if shared_writing(label, |_| false).is_none_or(|s| !s.is_empty()) {
        return false;
    }
    let latin = Writing::Script(Script::Latin);
    let rest = shared_writing(label, |s| s.contains(&latin)).unwrap_or_default();
    !rest.iter().any(|w| matches!(w, Writing::Japanese | Writing::Korean | Writing::Chinese))
}

#[cfg(test)]
mod tests;
