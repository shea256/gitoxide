use std::path::Path;

use git_worktree::fs;
use tempfile::{tempdir, TempDir};

fn panic_on_find<'buf>(_oid: &git_hash::oid, _buf: &'buf mut Vec<u8>) -> std::io::Result<git_object::BlobRef<'buf>> {
    unreachable!("find should nto be called")
}

#[test]
fn root_is_assumed_to_exist_and_files_in_root_do_not_create_directory() -> crate::Result {
    let dir = tempdir()?;
    let mut cache = fs::Cache::new(
        dir.path().join("non-existing-root"),
        fs::cache::State::for_checkout(false, Default::default()),
        Default::default(),
        Vec::new(),
        Default::default(),
    );
    assert_eq!(cache.num_mkdir_calls(), 0);

    let path = cache.at_path("hello", Some(false), panic_on_find)?.path();
    assert!(!path.parent().unwrap().exists(), "prefix itself is never created");
    assert_eq!(cache.num_mkdir_calls(), 0);
    Ok(())
}

#[test]
fn directory_paths_are_created_in_full() {
    let (mut cache, _tmp) = new_cache();

    for (name, is_dir) in &[
        ("dir", Some(true)),
        ("submodule", Some(true)),
        ("file", Some(false)),
        ("exe", Some(false)),
        ("link", None),
    ] {
        let path = cache
            .at_path(Path::new("dir").join(name), *is_dir, panic_on_find)
            .unwrap()
            .path();
        assert!(path.parent().unwrap().is_dir(), "dir exists");
    }

    assert_eq!(cache.num_mkdir_calls(), 3);
}

#[test]
fn existing_directories_are_fine() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    std::fs::create_dir(tmp.path().join("dir"))?;

    let path = cache.at_path("dir/file", Some(false), panic_on_find)?.path();
    assert!(path.parent().unwrap().is_dir(), "directory is still present");
    assert!(!path.exists(), "it won't create the file");
    assert_eq!(cache.num_mkdir_calls(), 1);
    Ok(())
}

#[test]
fn symlinks_or_files_in_path_are_forbidden_or_unlinked_when_forced() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    let forbidden = tmp.path().join("forbidden");
    std::fs::create_dir(&forbidden)?;
    symlink::symlink_dir(&forbidden, tmp.path().join("link-to-dir"))?;
    std::fs::write(tmp.path().join("file-in-dir"), &[])?;

    for dirname in &["file-in-dir", "link-to-dir"] {
        cache.unlink_on_collision(false);
        let relative_path = format!("{}/file", dirname);
        assert_eq!(
            cache
                .at_path(&relative_path, Some(false), panic_on_find)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );
    }
    assert_eq!(
        cache.num_mkdir_calls(),
        2,
        "it tries to create each directory once, but it's a file"
    );
    cache.reset_mkdir_calls();
    for dirname in &["link-to-dir", "file-in-dir"] {
        cache.unlink_on_collision(true);
        let relative_path = format!("{}/file", dirname);
        let path = cache.at_path(&relative_path, Some(false), panic_on_find)?.path();
        assert!(path.parent().unwrap().is_dir(), "directory was forcefully created");
        assert!(!path.exists());
    }
    assert_eq!(
        cache.num_mkdir_calls(),
        4,
        "like before, but it unlinks what's there and tries again"
    );
    Ok(())
}

fn new_cache() -> (fs::Cache<'static>, TempDir) {
    let dir = tempdir().unwrap();
    let cache = fs::Cache::new(
        dir.path(),
        fs::cache::State::for_checkout(false, Default::default()),
        Default::default(),
        Vec::new(),
        Default::default(),
    );
    (cache, dir)
}
