use std::path::{Component, Path, PathBuf};

pub fn relative_to(from: &Path, to: &Path) -> PathBuf {
    fn go(buf: &mut PathBuf, from: &Path, to: &Path) {
        if to.starts_with(from) {
            buf.push(to.strip_prefix(from).unwrap());
        } else {
            buf.push("..");
            go(buf, from.parent().unwrap(), to)
        }
    }

    let mut buf = PathBuf::new();
    go(&mut buf, from, to);
    buf
}

#[allow(dead_code)]
pub fn canonicalize_light<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut buf = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                buf.push(PathBuf::from(component.as_os_str()))
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if buf.file_name().is_none() {
                    buf.push(PathBuf::from(component.as_os_str()));
                } else {
                    buf.pop();
                }
            }
        };
    }
    buf
}

#[cfg(test)]
mod tests {
    use crate::paths::*;
    use std::path::Path;

    fn assert_relative_to(from: &str, to: &str, res: &str) {
        let from = Path::new(from);
        let to = Path::new(to);
        assert_eq!(relative_to(from, to).to_string_lossy(), res)
    }

    #[test]
    fn test_relative_to() {
        assert_relative_to("/a/b/c", "/a/e", "../../e");
        assert_relative_to("/a/b/c", "/", "../../../");
        assert_relative_to("/a/b/c", "/a/b/c/d", "d");
    }

    proptest! {
        #[test]
        fn check_relative_to(ref from in "(/[a-z]+)+", ref to in "(/[a-z]+)+") {
            let from = Path::new(from);
            let to = Path::new(to);
            let res = relative_to(from, to);
            assert_eq!(canonicalize_light(from.join(res)), to);
        }
    }

    #[test]
    fn test_canonicalize_light() {
        assert_eq!(
            canonicalize_light(Path::new("/a/b/..")).as_path(),
            Path::new("/a")
        );
        assert_eq!(
            canonicalize_light(Path::new("/a/b/../..")).as_path(),
            Path::new("/")
        );
        assert_eq!(
            canonicalize_light(Path::new("../../a")).as_path(),
            Path::new("../../a")
        );
    }
}
