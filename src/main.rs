extern crate bzip2;
extern crate libc;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate structopt;
extern crate tar;
extern crate tempdir;

use std::{fs, io, process};
use std::io::Write;
use structopt::StructOpt;

#[derive(Debug, Serialize, StructOpt)]
struct Options {
    #[structopt(default_value = "firefox-latest")] product: String,
    #[structopt(default_value = "linux64")] os: String,
    #[structopt(default_value = "en-US")] lang: String,
}

fn main() {
    unsafe {
        libc::umask(0o022);
    };

    let opt = Options::from_args();

    let client = reqwest::Client::new();
    let dir =
        tempdir::TempDir::new("mozdownload-deb-tmp").expect("could not create temporary directory");

    println!("Starting download...");
    let res = client
        .get("https://download.mozilla.org/")
        .query(&opt)
        .send()
        .expect("Download request failed");

    println!("Beginning extraction into tmpdir {:?}...", dir.path());
    // the resulting response is assumed to be a .tar.bz2
    let tar_stream = bzip2::bufread::BzDecoder::new(io::BufReader::new(res));
    let mut archive = tar::Archive::new(tar_stream);

    // create necessary structure
    let pkg_name = format!("moz-{}-{}-{}", &opt.product, &opt.os, &opt.lang);
    let pkg_dir = dir.path().join(&pkg_name);
    let opt_base_dir = pkg_dir.join(format!("opt/{}", &pkg_name));
    let control_dir = pkg_dir.join("DEBIAN");

    fs::create_dir_all(&opt_base_dir)
        .and_then(|_| fs::create_dir_all(&control_dir))
        .expect("could not create package dirs");

    // we now assume that there is a single folder called firefox inside the folder, missing DEBIAN
    // metadata
    let mut control =
        fs::File::create(control_dir.join("control")).expect("could not create control file");

    control
        .write_all(
            format!(
                "Package: {}\n\
                 Version: 0.{}\n\
                 Architecture: all\n\
                 Maintainer: Auto-generated via firefox-deb\n\
                 Provides: gnome-www-browser, www-browser\n\
                 Section: web\n\
                 Priority: optional\n\
                 Description: Mozilla Firefox, built from binaries\n",
                &pkg_name, 12345
            ).as_bytes(),
        )
        .expect("could not write control file");

    archive
        .unpack(opt_base_dir)
        .expect("could not unpack tar archive");
    println!("done. Running dpkg-deb --build...");

    process::Command::new("dpkg-deb")
        .arg("--build")
        .arg(pkg_dir)
        .arg(".")
        .output()
        .expect("could not run dpkg");

    println!("All done.");
}
