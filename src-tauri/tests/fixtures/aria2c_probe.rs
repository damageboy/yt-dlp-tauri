use std::{env, fs, io::{self, Write}, process::{Command, exit}, thread, time::Duration};

fn main() {
    let exe = env::current_exe().unwrap();
    let mode = fs::read_to_string(exe.parent().unwrap().join("mode")).unwrap_or_default();
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "child") {
        #[cfg(unix)]
        unsafe {
            unsafe extern "C" { fn signal(sig: i32, handler: usize) -> usize; }
            signal(15, 1);
        }
        println!("child-ready:{}", std::process::id());
        io::stdout().flush().unwrap();
        loop { thread::sleep(Duration::from_millis(50)); }
    }
    match mode.trim() {
        "sleep" => thread::sleep(Duration::from_secs(30)),
        "error" => { eprintln!("broken installation"); exit(2); }
        "empty" => {}
        "stderr" => eprintln!("aria2 version 1.37.0"),
        "parent" => {
            let mut child = Command::new(&exe).arg("child").spawn().unwrap();
            let _ = child.wait();
        }
        "capture" => {
            for arg in args { println!("{arg}"); }
        }
        _ => println!("aria2 version 1.37.0"),
    }
}
