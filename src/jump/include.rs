use super::{gen_module_pattern, Location};
use lsp_types::Url;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types;

use crate::{consts::TREESITTER_CMAKE_LANGUAGE, utils::treehelper::PositionType};

use super::getsubdef;
use std::collections::HashMap;
use std::fs;

use std::sync::{Arc, Mutex};

use std::sync::LazyLock;

fn ismodule(tojump: &str) -> bool {
    tojump.split('.').count() == 1
}

pub(super) fn cmpinclude(localpath: &Path, subpath: &str) -> Option<Vec<Location>> {
    let target = if !ismodule(subpath) {
        let root_dir = localpath.parent().unwrap();
        root_dir.join(subpath)
    } else {
        let glob_pattern = gen_module_pattern(subpath)?;
        glob::glob(glob_pattern.as_str())
            .into_iter()
            .flatten()
            .flatten()
            .next()?
    };

    if target.exists() {
        Some(vec![Location {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
            },
            uri: Url::from_file_path(target).unwrap(),
        }])
    } else {
        None
    }
}
#[test]
fn ut_ismodule() {
    assert_eq!(ismodule("GNUInstall"), true);
    assert_eq!(ismodule("test.cmake"), false);
}

type CacheData = HashMap<PathBuf, Vec<(String, Location, String)>>;

static PACKAGE_COMPLETE_CACHE: LazyLock<Arc<Mutex<CacheData>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

pub fn scanner_include_def(
    path: &PathBuf,
    postype: PositionType,
    include_files: &mut Vec<PathBuf>,
    complete_packages: &mut Vec<String>,
    find_cmake_in_package: bool,
    is_buildin: bool,
) -> Option<Vec<(String, Location, String)>> {
    if is_buildin {
        if let Ok(cache) = PACKAGE_COMPLETE_CACHE.lock() {
            if let Some(complete_items) = cache.get(path) {
                return Some(complete_items.clone());
            }
        }
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            let mut parse = tree_sitter::Parser::new();
            parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
            let thetree = parse.parse(content.clone(), None);
            let tree = thetree.unwrap();
            let result_data = getsubdef(
                tree.root_node(),
                &content.lines().collect(),
                path,
                postype,
                None,
                include_files,
                complete_packages,
                true,
                find_cmake_in_package,
            );
            if !is_buildin {
                return result_data;
            }
            if let Some(ref content) = result_data {
                if let Ok(mut cache) = PACKAGE_COMPLETE_CACHE.lock() {
                    cache.insert(path.clone(), content.clone());
                }
            }
            result_data
        }
        Err(_) => None,
    }
}

pub fn scanner_package_defs(
    path: &PathBuf,
    postype: PositionType,
    include_files: &mut Vec<PathBuf>,
    complete_packages: &mut Vec<String>,
) -> Option<Vec<(String, Location, String)>> {
    if let Ok(cache) = PACKAGE_COMPLETE_CACHE.lock() {
        if let Some(complete_items) = cache.get(path) {
            return Some(complete_items.clone());
        }
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            let mut parse = tree_sitter::Parser::new();
            parse.set_language(&TREESITTER_CMAKE_LANGUAGE).unwrap();
            let thetree = parse.parse(content.clone(), None);
            let tree = thetree.unwrap();
            let result_data = getsubdef(
                tree.root_node(),
                &content.lines().collect(),
                path,
                postype,
                None,
                include_files,
                complete_packages,
                false,
                true,
            );
            if let Some(ref content) = result_data {
                if let Ok(mut cache) = PACKAGE_COMPLETE_CACHE.lock() {
                    cache.insert(path.clone(), content.clone());
                }
            }
            result_data
        }
        Err(_) => None,
    }
}
