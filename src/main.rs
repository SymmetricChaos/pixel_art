// Everything is based on this game of life automata
//https://github.com/parasyte/pixels/tree/c2454b01abc11c007d4b9de8525195af942fef0d/examples/conway


use std::io;
use pixels::Error;
mod projects;


fn select_animation(input: &str) -> Result<(),Error> {
    match input {
        "1" => projects::sandpiles::run_piles(),
        "2" => projects::elementary::run_elementary(),
        "3" => projects::life::run_life(),
        "4" => {
            println!("These 'Binary Totalistic Automata' count the number of live cells in a nine cell neighborhood to determine the next state.");
            loop {
                println!("Please specify rule code less than 512");
                let mut text = String::new();
                io::stdin().read_line(&mut text).expect("Failed to read line");
                let n = text.trim().parse().unwrap();
                if n >= 512 {
                    continue
                }
                projects::totalistic::run_totalistic(n)?
            }
        },
        "5" => {
            println!("These 'Binary Outer Totalistic Automata' count the number of live cells in a nine cell neighborhood to determine the next state. However the rule is different depending on whether the center cell is active.");
            loop {
                println!("Please specify rule code less than 262144");
                let mut text = String::new();
                io::stdin().read_line(&mut text).expect("Failed to read line");
                let n = text.trim().parse().unwrap();
                if n >= 262144 {
                    continue
                }
                projects::outer_totalistic::run_outer_totalistic(n)?
            }

        },
        "6" => projects::critters::run_critters(),
        _ => {
            println!("unknown project");
            Ok(())
        }
    }
}

fn main() -> Result<(),Error> {
    println!("\nWelcome to my pixel animations!\nPress 'q' to quit this screen.");
    println!("\nWARNING: Totalistic and Outer Totalistic may produce flashing lights.");
    loop {
        println!("\n\nWhat would you like to see?\n\n1) Sandpiles\n2) Rule 110\n3) Life (not mine)\n4) Totalistic\n5) Outer Totalistic\n6) Critters");
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
        println!("\n\nControls for animation:\nC: clear screen\nP: pause\nR: randomize screen\nSPACE: frame by frame\nESC: close screen");
        match select_animation(v) {
            Ok(_) => continue,
            Err(e) => println!("{}",e),
        }
    }
    Ok(())
}