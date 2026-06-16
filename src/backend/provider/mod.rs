#![allow(dead_code)]

pub mod conan;
pub mod pkgconfig;
pub mod system;
pub mod vcpkg;

use miette::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct FetchedDep {
    pub name: String,
    pub include_dirs: Vec<PathBuf>,
    pub lib_dirs: Vec<PathBuf>,
    pub libs: Vec<String>,
    pub frameworks: Vec<String>,
}

pub trait Provider {
    fn name(&self) -> &str;
    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep>;
    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep>;
}
