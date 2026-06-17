use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use image::DynamicImage;
use ratatui::{
    layout::{Rect, Size},
    Frame,
};
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
    FontSize, Resize, StatefulImage,
};

/// Cap on bytes pulled from a remote image to bound memory and download time
const MAX_REMOTE_BYTES: u64 = 32 * 1024 * 1024;
const REMOTE_TIMEOUT: Duration = Duration::from_secs(10);

struct Loaded {
    image: DynamicImage,
    protocol: StatefulProtocol,
}

/// Loads and renders inline markdown images via the terminal graphics protocol
/// supported() reports whether the terminal has a native protocol
pub struct ImageManager {
    picker: Picker,
    supported: bool,
    base_dir: PathBuf,
    // None marks a target that failed to load so we don't retry every frame
    cache: HashMap<String, Option<Loaded>>,
}

impl ImageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let (picker, supported) = match Picker::from_query_stdio() {
            Ok(picker) => {
                let supported = picker.protocol_type() != ProtocolType::Halfblocks;
                (picker, supported)
            }
            Err(_) => (Picker::halfblocks(), false),
        };
        Self {
            picker,
            supported,
            base_dir,
            cache: HashMap::new(),
        }
    }

    pub fn supported(&self) -> bool {
        self.supported
    }

    pub fn set_base_dir(&mut self, base_dir: PathBuf) {
        if base_dir != self.base_dir {
            self.base_dir = base_dir;
            self.cache.clear();
        }
    }

    fn font_size(&self) -> FontSize {
        let font = self.picker.font_size();
        FontSize::new(font.width.max(1), font.height.max(1))
    }

    fn ensure_loaded(&mut self, target: &str) {
        if self.cache.contains_key(target) {
            return;
        }
        let loaded = self.load(target);
        self.cache.insert(target.to_string(), loaded);
    }

    fn load(&self, target: &str) -> Option<Loaded> {
        if target.is_empty() {
            return None;
        }
        let bytes = if target.contains("://") {
            fetch_remote(target)?
        } else {
            let path = Path::new(target);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.base_dir.join(path)
            };
            fs::read(resolved).ok()?
        };
        let image = image::load_from_memory(&bytes).ok()?;
        let protocol = self.picker.new_resize_protocol(image.clone());
        Some(Loaded { image, protocol })
    }

    /// Cell size the image fits into within avail_cols x max_rows preserving
    /// aspect ratio - None when the image can't be loaded
    pub fn fit_size(&mut self, target: &str, avail_cols: u16, max_rows: u16) -> Option<Size> {
        self.ensure_loaded(target);
        let font = self.font_size();
        let available = Size::new(avail_cols.max(1), max_rows.max(1));
        let loaded = self.cache.get(target)?.as_ref()?;
        let size = Resize::Fit(None).size_for(&loaded.image, font, available);
        Some(Size::new(size.width.max(1), size.height.max(1)))
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, target: &str, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.ensure_loaded(target);
        if let Some(Some(loaded)) = self.cache.get_mut(target) {
            let widget = StatefulImage::default().resize(Resize::Fit(None));
            frame.render_stateful_widget(widget, area, &mut loaded.protocol);
        }
    }
}

/// Fetch a remote image caching the bytes on disk so later runs skip the network, it returns the image bytes or None on any failure
fn fetch_remote(url: &str) -> Option<Vec<u8>> {
    let cache_path = remote_cache_path(url);
    if let Ok(bytes) = fs::read(&cache_path) {
        return Some(bytes);
    }
    let response = ureq::get(url).timeout(REMOTE_TIMEOUT).call().ok()?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_REMOTE_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&cache_path, &bytes);
    Some(bytes)
}

/// On-disk cache location for a remote image keyed by a hash of its URL
fn remote_cache_path(url: &str) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let name = format!("{:016x}", hasher.finish());
    cache_root().join("poneglyph").join("images").join(name)
}

fn cache_root() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".cache");
        }
    }
    std::env::temp_dir()
}
