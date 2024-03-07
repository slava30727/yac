use std::{ffi::{CStr, OsStr, OsString}, path::Path};



fn raw_c_str_to_os_string(raw: &CStr) -> OsString {
    #[allow(useless_ptr_null_checks)]
    if raw.as_ptr().is_null() {
        return OsString::new();
    }
    
    raw.to_str()
        .expect("failed to convert CStr to str")
        .into()
}



#[repr(C)]
pub struct VecCStr {
    ptr: *mut &'static CStr,
    cap: usize,
    len: usize,
}

impl From<&VecCStr> for Vec<OsString> {
    fn from(value: &VecCStr) -> Self {
        (0..value.len).map(|i| unsafe {
            let c_str = value.ptr.add(i).read();
            raw_c_str_to_os_string(c_str)
        }).collect()
    }
}



#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Build {
    pub executable_name: OsString,
    pub src_files: Vec<OsString>,
    pub link_directories: Vec<OsString>,
    pub enabled_flags: Vec<OsString>,
}

impl<Api: BuildApi> From<&CBuild<Api>> for Build {
    fn from(value: &CBuild<Api>) -> Self {
        Self {
            executable_name: raw_c_str_to_os_string(value.build.executable_name),
            src_files: (&value.build.src_files).into(),
            link_directories: (&value.build.link_directories).into(),
            enabled_flags: (&value.build.enables_flags).into(),
        }
    }
}



pub struct CBuild<Api: BuildApi> {
    api: Api,
    build: RawBuild,
}

impl<Api: BuildApi> CBuild<Api> {
    pub fn new(api: Api) -> Self {
        Self { build: unsafe { api.new() }, api }
    }

    pub fn build(&mut self) {
        self.api.build(&mut self.build);
    }
}

impl<Api: BuildApi> Drop for CBuild<Api> {
    fn drop(&mut self) {
        unsafe { self.api.free(&mut self.build) };
    }
}



#[repr(C)]
pub struct RawBuild {
    executable_name: &'static CStr,
    src_files: VecCStr,
    link_directories: VecCStr,
    enables_flags: VecCStr,
}



#[allow(clippy::missing_safety_doc, clippy::new_ret_no_self, clippy::wrong_self_convention)]
pub trait BuildApi {
    fn build(&self, build: &mut RawBuild);
    unsafe fn new(&self) -> RawBuild;
    unsafe fn free(&self, build: &mut RawBuild);
}



type BuilderBuildFn = unsafe extern "C" fn(*mut RawBuild);
#[allow(improper_ctypes_definitions)]
type BuilderNewFn = unsafe extern "C" fn() -> RawBuild;
type BuilderFreeFn = unsafe extern "C" fn(*mut RawBuild);

pub struct Builder {
    lib: libloading::Library,
    build_fn: libloading::Symbol<'static, BuilderBuildFn>,
    new_fn: libloading::Symbol<'static, BuilderNewFn>,
    free_fn: libloading::Symbol<'static, BuilderFreeFn>,
}

impl Builder {
    pub fn compile(build: impl AsRef<Path>, out: impl AsRef<Path>) {
        use std::process::Command;

        let src = r"D:\Svyatoslav\Programs\yac\src";

        let cfiles = walkdir::WalkDir::new(src)
            .into_iter()
            .flatten()
            .map(|entry| entry.into_path())
            .filter_map(|path|
                (path.extension()? == OsStr::new("c")).then_some(path)
            );

        Command::new("gcc")
            .arg("-shared")
            .arg("-o")
            .arg(out.as_ref())
            .args(cfiles)
            .arg(build.as_ref())
            .arg("-g")
            .args(["-I", r"D:\svyatoslav\programs\yac\include"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn new(lib_path: impl AsRef<Path>) -> Result<Self, BuilderCreationError> {
        use libloading::{Library, Symbol};

        unsafe {
            let lib = Library::new(lib_path.as_ref())?;

            /// Transmuting to cast out the 'lib lifetime to 'static.
            /// It is safe, because lib lives as long as func.

            let func = std::mem::transmute(
                lib.get::<BuilderBuildFn>(b"build")?,
            );

            let new_fn = std::mem::transmute(
                lib.get::<BuilderNewFn>(b"Build_new")?,
            );

            let free_fn = std::mem::transmute(
                lib.get::<BuilderFreeFn>(b"Build_free")?,
            );

            Ok(Self { lib, build_fn: func, new_fn, free_fn })
        }
    }
}

impl BuildApi for Builder {
    fn build(&self, build: &mut RawBuild) {
        unsafe { (self.build_fn)(build as *mut _) }
    }

    unsafe fn new(&self) -> RawBuild {
        (self.new_fn)()
    }

    unsafe fn free(&self, build: &mut RawBuild) {
        (self.free_fn)(build as *mut _)
    }
}



#[derive(Debug, thiserror::Error)]
pub enum BuilderCreationError {
    #[error(transparent)]
    SharedLibraryLoadError(#[from] libloading::Error),
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_test() {
        Builder::compile("test_prj/build.c", "test_prj/build.dll");
    }

    #[test]
    fn run_build() {
        let builder = Builder::new("test_prj/build.dll").unwrap();

        let mut build = CBuild::new(builder);

        build.api.build(&mut build.build);

        println!("{:?}", Build::from(&build));
    }
}