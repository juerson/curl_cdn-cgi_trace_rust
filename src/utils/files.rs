use std::{
    fs::{self, File},
    io::{self, BufRead, Write},
};

// 将结果写入文件
pub fn write_to_file(data: &[String], file_name: &str) -> Result<(), io::Error> {
    let mut file = File::create(file_name)?;
    for ip in data {
        writeln!(file, "{}", ip)?;
    }
    Ok(())
}

// 按行读取文件的内容(读取当前目录中第一个txt文件，并排除output.txt文件)
pub fn read_text_file() -> Result<Vec<String>, io::Error> {
    let current_dir = ".";
    let txt_files: Vec<String> = fs::read_dir(current_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            // 判断文件扩展名是".txt"，并且文件名不为"output.txt"
            path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("txt")
                && path.file_name().and_then(|s| s.to_str()) != Some("output.txt")
        })
        .filter_map(|entry| entry.path().to_str().map(|s| s.to_string()))
        .collect();

    if txt_files.is_empty() {
        println!("当前目录没有找到合适的TXT文件，按Enter键退出程序！");
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }
    // 只选择第一个txt文件的路径
    let file_path = &txt_files[0];
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("打开{}文件失败，错误原因是:{}", file_path, e);
            print!("按Enter键退出程序！");
            io::stdout().flush().expect("Failed to flush stdout");
            let _ = io::stdin().read_line(&mut String::new());
            std::process::exit(1);
        }
    };

    let ips: Vec<String> = io::BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok())
        .collect();

    if ips.is_empty() {
        print!("{}文件是空的，按Enter键退出程序！", file_path);
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }

    Ok(ips)
}

// 按行读取文件的内容（传入具体的txt文件）
// pub fn read_text_file(file_path: &str) -> Result<Vec<String>, io::Error> {
//     let file = match File::open(file_path) {
//         Ok(file) => file,
//         Err(e) => {
//             println!("打开{}文件失败，错误原因是:{}", file_path, e);
//             print!("按Enter键退出程序！");
//             io::stdout().flush().expect("Failed to flush stdout");
//             let _ = io::stdin().read_line(&mut String::new());
//             std::process::exit(1);
//         }
//     };
//     let ips: Vec<String> = io::BufReader::new(file)
//         .lines()
//         .filter_map(|line| line.ok())
//         .collect();
//     if ips.is_empty() {
//         print!("{}文件是空的，按Enter键退出程序！", file_path);
//         io::stdout().flush().expect("Failed to flush stdout");
//         let _ = io::stdin().read_line(&mut String::new());
//         std::process::exit(1);
//     }
//     Ok(ips)
// }
