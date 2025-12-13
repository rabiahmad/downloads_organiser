use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
use tracing::{debug, error, info};

fn current_user() -> String {
    std::env::var("USER").expect("USER environment variable not set")
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting file monitoring");

    let user = current_user();
    let downloads = PathBuf::from(format!("/home/{}/Downloads", user));
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(downloads.as_path(), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) => match event.kind {
                // Only keep Create or Modify events
                EventKind::Create(_) => {
                    info!("event: {:?}", event);

                    // Iterate each file, classify, identify destination and move file
                    for path in &event.paths {
                        // Must be a file
                        if !path.is_file() {
                            continue;
                        }

                        // Parent directory must be Downloads
                        if path.parent() != Some(downloads.as_path()) {
                            continue;
                        }

                        // If file is categorised, then handle it
                        if let Some(category) = classify_file(path) {
                            let dest_path: PathBuf = destination_path(path, category, &user);
                            match move_file(path, &dest_path) {
                                Ok(()) => debug!(
                                    from = %path.display(),
                                    to = %dest_path.display(),
                                    "File moved successfully"
                                ),
                                Err(e) => error!(
                                    from = %path.display(),
                                    to = %dest_path.display(),
                                    error = %e,
                                    "Failed to move file"
                                ),
                            }
                        }
                    }
                }
                _ => {} // ignore all other events
            },
            Err(e) => error!("watch error: {:?}", e),
        }
    }
    Ok(())
}

#[derive(Debug)]
enum Category {
    Picture,
    Video,
    Music,
    Document,
    Archive,
    Package,
}

impl Category {
    fn dest_dir(&self, user: &str) -> PathBuf {
        match self {
            Category::Picture => PathBuf::from(format!("/home/{}/Pictures/Downloads", user)),
            Category::Video => PathBuf::from(format!("/home/{}/Videos/Downloads", user)),
            Category::Music => PathBuf::from(format!("/home/{}/Music/Downloads", user)),
            Category::Document => PathBuf::from(format!("/home/{}/Documents/Downloads", user)),
            Category::Archive => PathBuf::from(format!("/home/{}/Downloads/Archives", user)),
            Category::Package => PathBuf::from(format!("/home/{}/Downloads/Packages", user)),
        }
    }
}

fn file_extension(path: &Path) -> Option<String> {
    // Get file extension from a given path
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

fn classify_file(path: &Path) -> Option<Category> {
    // Classify the file into enumerated categories based on file extension
    if !path.is_file() {
        debug!(path = %path.display(), "Not a file");
        return None;
    }

    let ext = file_extension(path)?;
    let category = match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" => Category::Picture,
        "docx" | "doc" | "md" | "txt" | "pdf" | "xlsx" => Category::Document,
        "mp3" | "flac" | "wav" => Category::Music,
        "mp4" | "mkv" | "avi" => Category::Video,
        "zip" | "tar" | "gz" => Category::Archive,
        "deb" | "rpm" | "apk" | "AppImage" | "iso" => Category::Package,
        _ => return None,
    };

    debug!(
        path = %path.display(),
        extension = ext,
        category = ?category,
        "Classified file"
    );

    Some(category)
}

fn destination_path(path: &Path, category: Category, user: &str) -> PathBuf {
    // Identify destination path of file based on the file category
    let mut dest_path = PathBuf::from(category.dest_dir(user));

    if let Some(filename) = path.file_name() {
        dest_path.push(filename);
    }

    debug!(
        current_path = %path.display(),
        destination_path = %dest_path.display(),
        "Destination path identified"
    );

    dest_path
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    // Move the file to the required destination
    info!(
        from = %from.display(),
        to = %to.display(),
        "Moving file"
    );

    // If destination directory does not exist, create it
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Move the file to the destination folder
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            // Fallback, cross-device link copy + delete instead of rename
            std::fs::copy(from, to)?;
            std::fs::remove_file(from)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
