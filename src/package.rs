use std::fs::{self, File};
use std::io::{Read, Write};
use std::env;
use std::path::PathBuf;
use std::fmt;

use {Result, Error};

use yaml_rust::{Yaml, YamlLoader, YamlEmitter};
use yaml_rust::yaml::Hash;

const PACKFILE_HEADER: &'static [u8] = b"# vim: ft=yaml
#
# Generated by pack. DO NOT EDIT!

";

lazy_static! {
    static ref BASE_DIR: PathBuf =
        env::var("VIM_CONFIG_PATH").map(|p| PathBuf::from(p)).unwrap_or_else(|_| {
            let home = env::home_dir().expect("No home directory found");
            home.join(".vim")
        });
    static ref PACK_DIR: PathBuf = (*BASE_DIR).join("pack");
    pub static ref PACK_CONFIG_DIR: PathBuf = (*BASE_DIR).join(".pack");
    static ref PACK_FILE: PathBuf = (*PACK_CONFIG_DIR).join("packfile");
    pub static ref PACK_PLUGIN_FILE: PathBuf = (*BASE_DIR).join("plugin").join("__pack.vim");
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub category: String,
    pub opt: bool,
    /// Load this package on this command
    pub load_command: Option<String>,
}

impl Package {
    pub fn new(name: &str, category: &str, opt: bool) -> Package {
        Package {
            name: name.to_string(),
            category: category.to_string(),
            opt: opt,
            load_command: None,
        }
    }

    pub fn is_installed(&self) -> bool {
        self.path().is_dir()
    }

    pub fn set_category<T: Into<String>>(&mut self, cat: T) {
        self.category = cat.into();
    }

    pub fn set_opt(&mut self, opt: bool) {
        self.opt = opt;
    }

    pub fn set_load_command(&mut self, cmd: &str) {
        self.load_command = Some(cmd.to_string())
    }

    pub fn from_yaml(doc: &Yaml) -> Result<Package> {
        let name = doc["name"].as_str().map(|s| s.to_string()).ok_or(Error::Format)?;
        let opt = doc["opt"].as_bool().ok_or(Error::Format)?;
        let category = doc["category"].as_str().map(|s| s.to_string()).ok_or(Error::Format)?;
        let cmd = doc["on"].as_str().map(|s| s.to_string());

        Ok(Package {
            name: name,
            category: category,
            opt: opt,
            load_command: cmd,
        })
    }

    pub fn into_yaml(self) -> Yaml {
        let mut doc = Hash::new();
        doc.insert(Yaml::from_str("name"), Yaml::from_str(&self.name));
        doc.insert(Yaml::from_str("category"), Yaml::from_str(&self.category));
        doc.insert(Yaml::from_str("opt"), Yaml::Boolean(self.opt));
        if let Some(ref c) = self.load_command {
            doc.insert(Yaml::from_str("on"), Yaml::from_str(c));
        }
        Yaml::Hash(doc)
    }

    pub fn path(&self) -> PathBuf {
        let (_, repo) = self.repo();
        if self.opt {
            PACK_DIR.join(&self.category).join("opt").join(repo)
        } else {
            PACK_DIR.join(&self.category).join("start").join(repo)
        }
    }

    pub fn config_path(&self) -> PathBuf {
        let name = self.name.replace("/", "-");
        let fname = if name.ends_with(".vim") {
            name
        } else {
            format!("{}.vim", &name)
        };
        PACK_CONFIG_DIR.join(fname)
    }

    pub fn repo(&self) -> (&str, &str) {
        let mut info = self.name.splitn(2, "/");
        let user = info.next().unwrap_or("");
        let repo = info.next().unwrap_or("");
        (user, repo)
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = if self.opt { "opt" } else { "start" };
        let on = match self.load_command {
            Some(ref c) => format!("[Load on `{}`]", c),
            None => "".to_string(),
        };
        write!(f,
               "{} => pack/{}/{} {}",
               &self.name,
               &self.category,
               name,
               on)
    }
}

pub fn fetch() -> Option<Vec<Package>> {
    if !PACK_FILE.is_file() {
        return None;
    }

    let mut data = String::new();
    File::open(&*PACK_FILE)
        .expect("Fail to open packfile")
        .read_to_string(&mut data)
        .expect("Fail to read packfile");
    let docs = YamlLoader::load_from_str(&data).expect("Unexpected packfile format");

    if docs.is_empty() {
        None
    } else {
        docs[0].as_vec().map(|doc| {
            doc.iter()
                .map(|d| Package::from_yaml(d).expect("Invalid format"))
                .collect::<Vec<Package>>()
        })
    }
}

pub fn save(mut packs: Vec<Package>) -> Result<()> {
    packs.sort_by(|a, b| a.name.cmp(&b.name));
    let packs = packs.into_iter().map(|e| e.into_yaml()).collect::<Vec<Yaml>>();
    let doc = Yaml::Array(packs);
    let mut out = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out);
        emitter.dump(&doc)?;
    }
    if !PACK_CONFIG_DIR.is_dir() {
        fs::create_dir_all(&*PACK_CONFIG_DIR)?;
    }
    let mut f = File::create(&*PACK_FILE)?;
    f.write(PACKFILE_HEADER)?;
    f.write(out.as_bytes())?;
    Ok(())
}

// #[test]
// fn test_fetch() {
//     env::set_var("VIM_CONFIG_PATH", "./test");
//     let packs = fetch();
//     println!("{:?}", packs);
// }
//
// #[test]
// fn test_save() {
//     use std::fs::remove_file;
//
//     env::set_var("VIM_CONFIG_PATH", "./test/packtest");
//     let packs = vec![Package::new("test/hello", "default", true),
//                      Package::new("rust-lang/rust.vim", "rust", false)];
//     save(packs).unwrap();
//     let mut expected = String::new();
//     File::open(&*PACK_FILE).unwrap().read_to_string(&mut expected).unwrap();
//     assert_eq!(expected,
//                "---\n- \n  category: rust\n  name: \"rust-lang/rust.vim\"\n  opt: false\n- \n  \
//                 category: default\n  name: test/hello\n  opt: true");
//
//     // remove_file(&*PACK_FILE).unwrap();
// }
