use std::{fs, io, path::Path};
use anyhow::{anyhow, Result};
use guard::continue_unless;

// Import folder is hard-coded to "C:\Users\Chris Petkau\Downloads".
const IMPORT_FOLDER: &str = "C:/Users/Chris Petkau/Downloads";

/// Find the most recent downloaded file with prefix "moonlander_colemak_coder_" and extension ".zip".
pub(crate) fn find_most_recent_download() -> Result<String> {
    Ok(fs::read_dir(IMPORT_FOLDER)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().into_string().ok()?;
            let time_stamp = entry.metadata().ok()?.modified().ok()?;
            if entry.file_type().ok()?.is_file() {
                Some((file_name, time_stamp))
            } else {
                None
            }
        })
        .filter_map(|(file_name, time_stamp)| {
            if file_name.starts_with("moonlander_colemak_coder_") && file_name.ends_with(".zip") {
                Some((file_name, time_stamp))
            } else {
                None
            }
        })
        .max_by_key(|(_, time_stamp)| *time_stamp)
        .ok_or_else(|| anyhow!("No .zip file found."))?
        .0)
}

/// Extract files and put them in the "temp" folder.
pub(crate) fn extract_files_to_temp(zip: &str) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(Path::new(IMPORT_FOLDER).join(&zip))?)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let file_name = file.name();
        continue_unless!(file_name.starts_with("moonlander_colemak_coder_source"));
        continue_unless!(!file_name.ends_with('/'));
        let outpath = match file.enclosed_name() {
            Some(path) => {
                let path = Path::new(
                    path.components()
                        .last()
                        .ok_or_else(|| anyhow!("Empty filename in .zip file."))?
                        .as_os_str(),
                );
                Path::new(crate::temp_folder::NAME).join(path)
            }
            None => continue,
        };
        println!(
            "Entry {} is a file. Extracting \"{}\" ({} bytes)",
            i,
            outpath.display(),
            file.size()
        );
        io::copy(&mut file, &mut fs::File::create(&outpath)?)?;
    }
    Ok(())
}
