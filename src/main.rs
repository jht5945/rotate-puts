use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::process::exit;
use std::time::Duration;

use clap::{App, AppSettings, Arg};
use rust_util::{iff, information, util_size};

fn main() {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .long_about("Rotate standard in to log files")
        .arg(Arg::with_name("prefix")
            .long("prefix").takes_value(true).default_value("temp").help("Log file prefix"))
        .arg(Arg::with_name("suffix")
            .long("suffix").takes_value(true).default_value("log").help("Log file suffix"))
        .arg(Arg::with_name("file-size")
            .long("file-size").takes_value(true).default_value("10m").help("Single log file size"))
        .arg(Arg::with_name("file-count")
            .long("file-count").takes_value(true).default_value("10").help("Keep file count (from 0 to 1000)"))
        .setting(AppSettings::ColoredHelp);

    let arg_matchers = app.get_matches();
    let prefix = arg_matchers.value_of("prefix").unwrap().to_string();
    let suffix = arg_matchers.value_of("suffix").unwrap().to_string();
    let file_size = arg_matchers.value_of("file-size").unwrap();
    let file_size = util_size::parse_size(file_size).unwrap_or_else(|_| 10 * 1024 * 1028) as usize;
    let file_count = arg_matchers.value_of("file-count").unwrap();
    let file_count = file_count.parse().unwrap_or_else(|_| 10);
    let file_count = match file_count {
        i if i < 0 => {
            0
        }
        i if i > 1000 => {
            1000
        }
        i => i as i32,
    };

    information!("Prefix: {}, suffix: {}, file size: {}, file count: {}",
        prefix, suffix, util_size::get_display_size(file_size as i64), file_count
    );
    let (sender, receiver) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut file_index = 0;
        let mut written_len = 0;

        let file_name = make_new_file_name(&prefix, &suffix, file_count, &mut file_index);
        let mut out_file = BufWriter::new(File::create(&file_name)
            .expect(&format!("Create file failed: {}", file_name)));

        let mut write_buffer = Vec::with_capacity(1024 * 8);
        loop {
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Err(_e) => {
                    if write_buffer.len() > 0 {
                        written_len += write_buffer.len();
                        out_file.write(&write_buffer).ok();
                        write_buffer.clear();
                    }
                }
                Ok(buff) => {
                    if buff.len() == 0 {
                        out_file.write(&write_buffer).ok();
                        out_file.flush().ok();
                        exit(0);
                    }
                    write_buffer.extend_from_slice(&buff);
                    if write_buffer.len() > 1024 {
                        let mut pos_of_n = -1_isize;
                        write_buffer.iter().enumerate().for_each(|(i, c)| {
                            if *c == b'\n' { pos_of_n = i as isize; }
                        });
                        if pos_of_n > -1 {
                            let left_buffer = write_buffer.split_off(pos_of_n as usize + 1);
                            written_len += write_buffer.len();
                            out_file.write(&write_buffer).ok();
                            write_buffer = left_buffer;
                            out_file.flush().ok();

                            if written_len >= file_size {
                                written_len = 0;
                                let file_name = make_new_file_name(&prefix, &suffix, file_count, &mut file_index);
                                out_file = BufWriter::new(File::create(&file_name)
                                    .expect(&format!("Create file failed: {}", file_name)));
                            }
                        }
                    }
                }
            }
        }
    });

    let mut std_in = std::io::stdin();
    let mut buff = [0_u8; 128];
    loop {
        match std_in.read(&mut buff) {
            Ok(len) if len == 0 => {
                sender.send(Vec::new()).ok();
            }
            Ok(len) => {
                let mut vec = Vec::with_capacity(len);
                for b in &buff[0..len] {
                    vec.push(*b);
                }
                if let Err(e) = sender.send(vec) {
                    eprintln!("[ERROR] Send error: {}", e);
                }
            }
            Err(e) => eprintln!("[ERROR] Send error: {}", e),
        }
    }
}

fn make_new_file_name(prefix: &str, suffix: &str, file_count: i32, index: &mut i32) -> String {
    let i = *index;
    *index = i + 1;

    let pending_rm = generate_file_name(prefix, suffix, i - file_count);
    if let Ok(_) = std::fs::metadata(&pending_rm) {
        println!("[INFO] Remove log file: {}", &pending_rm);
        std::fs::remove_file(&pending_rm).ok();
    }

    let file_name = generate_file_name(prefix, suffix, i);
    println!("[INFO] New log file: {}", &file_name);
    file_name
}

fn generate_file_name(prefix: &str, suffix: &str, index: i32) -> String {
    format!("{}_{:03}{}{}", prefix, index, iff!(suffix.is_empty(), "", "."), suffix)
}
