// Copyright (c) 2020 10X Genomics, Inc. All rights reserved.

// Build html files by generating and inserting other html files.

use enclone::html::insert_html;
use enclone::misc3::parse_bsv;
use enclone_core::testlist::SITE_EXAMPLES;
use io_utils::*;
use pretty_trace::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process::Command;
use string_utils::*;

fn main() {
    PrettyTrace::new().on();

    let mut results = Vec::<(usize, String)>::new();
    for i in 0..SITE_EXAMPLES.len() {
        results.push((i, String::new()));
    }
    results.par_iter_mut().for_each(|r| {
        let i = r.0;
        let test = SITE_EXAMPLES[i].1;
        let args = parse_bsv(&test);
        let new = Command::new("target/debug/enclone")
            .args(&args)
            .arg("MAX_CORES=24")
            .output()
            .expect(&format!("failed to execute build_html"));
        r.1 = stringme(&new.stdout);
    });
    for i in 0..SITE_EXAMPLES.len() {
        let example_name = SITE_EXAMPLES[i].0;
        let out_file = format!("pages/auto/{}.html", example_name);
        let mut f = open_for_write_new![&out_file];
        fwrite!(&mut f, "{}", results[i].1);
    }

    insert_html("pages/index.html.src", "index.html", false);
    insert_html("pages/expanded.html.src", "pages/auto/expanded.html", false);
}