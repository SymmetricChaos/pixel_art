use std::io;
use pixels::Error;
mod projects;


fn select_animation(input: &str) -> Result<(),Error> {
    match input {
        "1" => projects::sandpiles::run_piles(),
        "2" => projects::elementary::run_elementary(),
        _ => {
            println!("unknown project");
            Ok(())
        }
    }
}

fn main() -> Result<(),Error> {
    println!("\nWelcome to my pixel animation project!");
    loop {
        println!("\n\nWhat would you like to see?\n\n1) Sandpiles\n2) Rule 110");
        let mut val = String::new();
        io::stdin().read_line(&mut val).expect("Failed to read line");

        let v = val.trim();
        
        if v == "q" || v == "quit" {
            break
        }

        if !v.chars().all(char::is_numeric) {
            println!("\nERROR: Must input an integer or a valid command.");
            continue
        }
        println!("\n\nShowing {}",v);
        select_animation(v)?
    }
    Ok(())
}