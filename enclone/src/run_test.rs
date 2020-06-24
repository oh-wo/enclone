// Copyright (c) 2020 10X Genomics, Inc. All rights reserved.

use crate::misc3::parse_bsv;
use ansi_escape::*;
use enclone_core::testlist::*;
use io_utils::*;
use itertools::Itertools;
use std::cmp::min;
use std::fs::read_to_string;
use std::io::Write;
use std::process::Command;
use string_utils::*;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Run an enclone test.

pub fn run_test(
    enclone: &str,     // name of the enclone executable
    it: usize,         // test number
    test: &str,        // arguments for the test
    testname: &str,    // test category e.g. "test" or "ext_test"
    ok: &mut bool,     // true if test passes
    logx: &mut String, // logging from test
    out: &mut String,  // stdout of test
) {
    let mut test = test.replace("\n", "");
    for _ in 0..3 {
        test = test.replace("  ", " ");
    }
    let mut expect_null = false;
    let mut expect_fail = false;
    let mut expect_ok = false;
    let mut set_in_stone = false;
    let mut no_pre = false;
    let mut nforce = false;
    let mut ncores = false;
    if test.contains(" EXPECT_NULL") {
        test = test.replace(" EXPECT_NULL", "");
        expect_null = true;
    }
    if test.contains(" EXPECT_FAIL") {
        test = test.replace(" EXPECT_FAIL", "");
        expect_fail = true;
    }
    if test.contains(" EXPECT_OK") {
        test = test.replace(" EXPECT_OK", "");
        expect_ok = true;
    }
    if test.contains(" SET_IN_STONE") {
        test = test.replace(" SET_IN_STONE", "");
        set_in_stone = true;
    }
    if test.contains(" NO_PRE") {
        test = test.replace(" NO_PRE", "");
        no_pre = true;
    }
    if test.contains(" NFORCE") {
        test = test.replace(" NFORCE", "");
        nforce = true;
    }
    if test.contains(" NCORES") {
        test = test.replace(" NCORES", "");
        ncores = true;
    }
    test = test.replace("{TEST_FILES_VERSION}", &format!("{}", TEST_FILES_VERSION));
    let mut log = Vec::<u8>::new();
    let out_file = format!("test/inputs/outputs/enclone_{}{}_output", testname, it + 1);
    let mut pre_arg = format!("PRE=test/inputs/version{}", TEST_FILES_VERSION);
    let mut local_pre_arg = format!(
        "PRE=enclone_main/test/inputs/version{},enclone_main",
        TEST_FILES_VERSION
    );
    if no_pre {
        pre_arg = String::new();
        local_pre_arg = String::new();
    }
    if !path_exists(&out_file) && !expect_fail && !expect_ok {
        fwriteln!(log, "\nYou need to create the output file {}.\n", out_file);
        fwriteln!(
            log,
            "Do this by executing the following command from \
             the top level of the enclone repo:\n"
        );
        emit_bold_escape(&mut log);
        fwriteln!(
            log,
            "enclone {} {} > enclone_main/test/inputs/outputs/enclone_{}{}_output; \
             git add enclone_main/test/inputs/outputs/enclone_{}{}_output\n",
            local_pre_arg,
            test,
            testname,
            it + 1,
            testname,
            it + 1
        );
        emit_end_escape(&mut log);
        *logx = stringme(&log);
    } else {
        let mut old = String::new();
        if !expect_fail && !expect_ok {
            old = read_to_string(&out_file).unwrap();
        }
        let args = parse_bsv(&test);

        // Form the command and execute it.

        let mut new = Command::new(&enclone);
        let mut new = new.arg(&args[0]);
        if !no_pre {
            new = new.arg(&pre_arg);
        }
        for i in 1..args.len() {
            new = new.arg(&args[i]);
        }
        if !nforce {
            new = new.arg("FORCE_EXTERNAL")
        }
        if !ncores {
            // Cap number of cores at 24.  Surprisingly, for testing on a 64-core
            // server, this significantly reduces wallclock.  And substituting either
            // 16 or 64 is slower.  Slower at the time of testing!  As we add tests or
            // change the algorithms, this may change.
            new = new.arg("MAX_CORES=24")
        }
        // dubious use of expect:
        let new = new
            .output()
            .expect(&format!("failed to execute enclone for test{}", it + 1));
        let new_err = strme(&new.stderr).split('\n').collect::<Vec<&str>>();
        let new2 = stringme(&new.stdout);
        *out = new2.clone();

        // Process tests that were supposed to fail or supposed to succeed.

        if expect_fail || expect_ok {
            *ok = false;
            if new.status.code().is_none() {
                fwriteln!(log, "\nCommand for subtest {} failed.", it + 1);
                fwriteln!(
                    log,
                    "Something really funky happened, status code unavailable.\n"
                );
            } else {
                let status = new.status.code().unwrap();
                if expect_fail {
                    if status == 0 {
                        fwriteln!(log, "\nCommand for subtest {} failed.", it + 1);
                        fwriteln!(
                            log,
                            "That test was supposed to have failed, but instead \
                             succeeded.\n"
                        );
                    } else if status != 1 {
                        fwriteln!(log, "\nCommand for subtest {} failed.", it + 1);
                        fwriteln!(
                            log,
                            "That test was supposed to have failed with exit status 1,\n\
                             but instead failed with exit status {}.\n",
                            status
                        );
                    } else {
                        *ok = true;
                    }
                } else {
                    if status != 0 {
                        fwriteln!(log, "\nCommand for subtest {} failed.", it + 1);
                        fwrite!(
                            log,
                            "That test was supposed to have succeeded, but instead \
                             failed, with stderr = {}",
                            new_err.iter().format("\n")
                        );
                    } else {
                        *ok = true;
                    }
                }
            }
            *logx = stringme(&log);

        // Process tests that yield the expected stdout.
        } else if old == new2 {
            *ok = true;
            if old.len() <= 1 && !expect_null {
                fwriteln!(
                    log,
                    "\nWarning: old output for subtest {} has {} bytes.\n",
                    it + 1,
                    old.len()
                );
            }
            if new.stderr.len() > 0 {
                fwriteln!(log, "Command for subtest {} failed.\n", it + 1);
                fwriteln!(log, "stderr has {} bytes:", new.stderr.len());
                fwrite!(log, "{}", strme(&new.stderr));
                *ok = false;
            }
            *logx = stringme(&log);

        // Process tests that yield unexpected stdout.
        } else {
            fwriteln!(log, "\nSubtest {}: old and new differ", it + 1);
            fwriteln!(
                log,
                "old has u8 length {} and new has u8 length {}",
                old.len(),
                new2.len()
            );
            let mut oldc = Vec::<char>::new();
            let mut newc = Vec::<char>::new();
            for c in old.chars() {
                oldc.push(c);
            }
            for c in new2.chars() {
                newc.push(c);
            }
            fwriteln!(
                log,
                "old has char length {} and new has char length {}",
                oldc.len(),
                newc.len()
            );
            for i in 0..min(oldc.len(), newc.len()) {
                if oldc[i] != newc[i] {
                    fwriteln!(
                        log,
                        "the first difference is at character {}: old = \"{}\", \
                         new = \"{}\"\n",
                        i,
                        oldc[i],
                        newc[i]
                    );
                    break;
                }
            }
            fwrite!(log, "old:\n{}", old);
            fwrite!(log, "new:\n{}", new2);
            if new_err.len() != 1 || new_err[0].len() != 0 {
                fwriteln!(log, "stderr has {} lines:", new_err.len());
                for i in 0..new_err.len() {
                    fwriteln!(log, "{}", new_err[i]);
                }
            }
            // let f = format!(
            //     "test/inputs/version{}/{}/outs/all_contig_annotations.json.lz4",
            //         version, args[0].after("=") );
            // if !path_exists(&f) {
            //     println!( "Perhaps you forgot to lz4 compress the json file.\n" );
            //     std::process::exit(1);
            // }
            // println!( "The size of {} is {} bytes.", f, fs::metadata(&f).unwrap().len() );

            fwriteln!(
                log,
                "enclone subtest {} failed.  If you are happy with the new output, \
                 you can replace the\noutput by executing the following command from \
                 the top level of the enclone repo (essential):\n",
                it + 1
            );
            if set_in_stone {
                fwriteln!(
                    log,
                    "🔴 However, the output of this test was not supposed to have changed.\n\
                     🔴 Please be extremely careful if you change it.\n",
                );
            }
            emit_bold_escape(&mut log);
            fwriteln!(
                log,
                "enclone {} {} \
                 > enclone_main/test/inputs/outputs/enclone_{}{}_output\n",
                local_pre_arg,
                test,
                testname,
                it + 1
            );
            emit_end_escape(&mut log);
            fwrite!(log, "and then committing the changed file.  ");
            fwriteln!(
                log,
                "You can then retest using:\n\n\
                 cargo test -p enclone enclone  -- --nocapture"
            );
            if new2.len() > 0 {
                fwriteln!(log, "");
                *logx = stringme(&log);
            } else if old != new2 {
                fwriteln!(log, "old != new");
                *logx = stringme(&log);
            }
        }
    }
}