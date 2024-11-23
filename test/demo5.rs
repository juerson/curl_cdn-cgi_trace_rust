use std::{ collections::HashSet, fs::File, io::{ self, BufRead, Write }, path::Path };

fn read_text_file<P>(filename: P) -> io::Result<Vec<String>> where P: AsRef<Path> {
    let file = match File::open(&filename) {
        Ok(file) => file,
        Err(e) => {
            let filename_str = filename.as_ref().to_str().unwrap_or("<invalid path>");
            println!("打开{:?}文件失败，错误原因是:{}", filename_str, e);
            print!("按Enter键退出程序！");
            io::stdout().flush().expect("Failed to flush stdout");
            let _ = io::stdin().read_line(&mut String::new());
            std::process::exit(1);
        }
    };
    let buf = io::BufReader::new(file);
    let mut unique_lines = HashSet::new();

    for line in buf.lines() {
        let line = line?;
        let trimmed_line = line.trim();
        if !trimmed_line.is_empty() {
            unique_lines.insert(trimmed_line.to_string());
        }
    }

    let mut unique_lines_vec: Vec<String> = unique_lines.into_iter().collect();
    unique_lines_vec.sort();

    Ok(unique_lines_vec)
}

fn main() -> io::Result<()> {
    let filename = "ips-v4.txt";
    match read_text_file(filename) {
        Ok(lines) => {
            println!("{:?}", lines);
        }
        Err(error) => {
            eprintln!("Error reading file: {:?}", error);
        }
    }

    Ok(())
}
