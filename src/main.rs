use std::env;
use std::vec::Vec;
use std::collections::HashMap;
use std::path::PathBuf;
use std::error;
use std::ffi::OsString;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use clap::{Arg, App};
use regex::Regex;

#[derive(PartialEq)]
enum FilePart{
    DesktopEntry,
    None
}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

fn parse_desktop_entry(path: PathBuf) -> HashMap<String, String>{
    let mut app: HashMap<String, String> = HashMap::new();
    let file = match File::open(path){
        Ok(file) => file,
        Err(_) => return app
    };
    let reader = BufReader::new(file);

    let mut part: FilePart = FilePart::None;
    for line in reader.lines() {
        if let Ok(line) = line{
            if line.starts_with("["){
                if line.starts_with("[Desktop Entry]") && part == FilePart::None{
                    part = FilePart::DesktopEntry;
                }
                else{
                    return app;
                }
            }
            else if part == FilePart::None ||  line.is_empty() || line.starts_with('#') {
                continue;
            }
            else{
                let mut pair = line.split('=');
                let key = pair.next().expect("Expected a key").trim();
                let value = pair.next().expect("Expected a value").trim();
                app.insert(key.to_owned(), value.to_owned());
            }
        }
        else{
            return app;
        }
    }
    app
}

fn find_applications() -> Result<Vec<HashMap<String, String>>>{
    let mut apps: Vec<HashMap<String, String>> = Vec::new();
    let paths = match env::var_os("XDG_DATA_DIRS"){
        Some(path) => path,
        None => OsString::from("/usr/share")
    };

    for mut path in env::split_paths(&paths){
        path.push("applications");
        if path.exists() && path.is_dir(){
            let entries = path.read_dir()?;
            let entries = entries.filter(|e| 
                match e{ 
                    Ok(e) => match e.path().extension(){
                        Some(ext) => ext == "desktop",
                        None => false
                    },
                    &Err(_) => false
                }
            );
            
           let entries = entries.map(|e| e.expect("Desktop file should exist").path());
            entries.for_each(|entry| apps.push(parse_desktop_entry(entry)));
            
        }
    }

    Ok(apps)
}

fn print(apps: &Vec<HashMap<String, String>>, format: &str, clean_exec: bool){
    let mut vars: Vec<String> = Vec::new();
    let var_regex = Regex::new(r"%([\w\[\]]+)").unwrap();
    for cap in var_regex.captures_iter(format) {
        vars.push((&cap[1]).to_owned());
    }
    for app in apps{
        let mut text = format.to_owned();
        for var in &vars{
            let key = "%".to_string() + var;
            let value = match app.contains_key(var){
                true => &app[var],
                false => ""
            };
            if var == "Exec" && clean_exec{
                text = text.replace(&key, &var_regex.replace_all(value, ""));
            }
            else{
                text = text.replace(&key, &value);
            }
        }
        text += "\n";
        if std::io::stdout().write(text.as_bytes()).is_err(){
            return;
        }
    }
}

fn main() {

    let matches = App::new("applications")
        .version("0.0.1")
        .author("data-niklas")
        .about("display .desktop entries")
        .arg(Arg::new("FORMAT")
                 .index(1)
                 .about("The format "))
        .arg(Arg::new("clean_exec")
                .short('c')
                .about("Cleans exec"))
        .get_matches();

    let mut format = "%Name|%Exec";
    let mut clean_exec = false;

    if matches.is_present("FORMAT"){
        format = matches.value_of("FORMAT").unwrap();
    }
    if matches.is_present("clean_exec") {
        clean_exec = true;
    }


    let apps = find_applications();
    if let Ok(apps) = apps{
        print(&apps, format, clean_exec);
    }
    
}
