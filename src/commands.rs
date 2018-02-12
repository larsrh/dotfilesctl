use config::*;
use dotfiles::*;
use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;
use paths::*;
use util::DotfilesError;

pub use config::init as init;

pub fn watch(config: PathBuf) -> Result<(), Error> {
    let config = check_config(&config)?;
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;
    info!("Watching file changes in target {:?}", config.target);
    watcher.watch(config.target.clone(), RecursiveMode::Recursive)?;
    loop {
        let event = rx.recv()?;
        match event {
            DebouncedEvent::Create(created) => {
                let relative = relative_to(config.target.as_path(), created.as_path());
                info!("File created: {:?}", relative)
            },
            _ => {}
        }
    }
}

// TODO implement thorough checking
pub fn check(config: PathBuf, _thorough: bool, repair: bool) -> Result<(), Error> {
    let config = check_config(&config)?;
    let dotfiles = load_dotfiles(&config)?;

    info!("Checking for absent content in {:?}", config.contents());
    let absent_contents = dotfiles.get_absent_files(config.contents().as_path());
    if absent_contents.is_empty() {
        info!("No absent content.")
    }
    else {
        if repair {
            warn!("Cannot fix absent content.");
        }
        let msg = format!("Absent content: {:?}", absent_contents);
        let err = DotfilesError::new(msg);
        Err(err)?
    }

    let home = config.get_home()?;
    info!("Checking for symlinks in {:?}", home);
    let symlinks = dotfiles.get_symlinks(config.contents().as_path(), home.as_path());
    for (dotfile, symlink) in &symlinks {
        match symlink.status {
            SymlinkStatus::Wrong => {
                let msg = format!("{:?} is not a symbolic link or symbolic link with wrong target, expected: {:?}", dotfile, symlink.expected);
                let err = DotfilesError::new(msg);
                Err(err)?
            },
            SymlinkStatus::Absent(ref err) => {
                let msg = format!("{:?} does not exist, expected symbolic link to {:?} ({:?})", dotfile, symlink.expected, err);
                Err(DotfilesError::new(msg))?
            },
            SymlinkStatus::Ok => ()
        }
    }
    info!("{} symlinks correct.", symlinks.len());

    save_dotfiles(&config, dotfiles)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use commands::*;
    use config::test_util::*;
    use std::os::unix::fs as unix;

    #[test]
    fn test_check_success() {
        let (dir, config) = setup_config();
        let files = vec![".test1", ".test2"];
        for f in &files {
            let path = config.contents().join(f);
            let msg = format!("{:?} can be created", path);
            File::create(path).expect(msg.as_ref());
            unix::symlink(config.contents().join(f), config.get_home().unwrap().join(f)).unwrap();
        }
        let dotfiles = Dotfiles::new(Some(files.iter().map(PathBuf::from).collect()));
        save_dotfiles(&config, dotfiles).unwrap();
        check(dir.path().join("config.toml"), false, false).unwrap();
    }
}