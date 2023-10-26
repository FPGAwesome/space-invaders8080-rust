use std::io::{self,BufRead, Write};
use crate::state8080::{State8080, self};

//return a command to run and an optional secondary argument
pub fn parse_command(emu8080: &mut State8080) -> i32 {
    //TODO: Make this a 'manual' debugger mode
    print!(">>>");
    io::stdout().flush().unwrap(); // Flush the output buffer because we don't have a \n

    //println!("Next opcode to run {:02X}", emu8080.read_mem(emu8080.get_pc()));
    let mut input = String::new();

    // Read user input
    io::stdin().lock().read_line(&mut input).unwrap();

    // Trim leading/trailing whitespaces and convert to lowercase
    let input = input.trim().to_lowercase();

    let mut iter = input.trim().split_whitespace();

    if let Some(cmd) = iter.next() {
        match cmd {
            "quit" => return -1,

            // run for n instructions, return 1 for run
            "run" => {
                if let Some(arg) = iter.next() {
                    // Handle the "run" command with the specified argument
                    // Add your code here to handle the argument as desired
                    println!("Running program for {} lines", arg);
                    let runcmd = arg.parse::<i32>().unwrap_or(0);

                    for _ in 1..runcmd {
                        state8080::emulate_8080_op(emu8080);
                    }
                    // Return the desired integer value
                    return 0;
                } else {
                    println!("Missing argument for 'run' command");
                    // Return an error code or handle the missing argument case as desired
                    return 0;
                }
            }
            // Run until some condition is met
            "cnd" => {
                if let Some(arg) = iter.next() {
                    // Split the argument into parts using the logic operator as the separator
                    let parts: Vec<&str> = arg.splitn(2, |c| c == '=' || c == '<' || c == '>').collect();
                    if parts.len() == 2 {
                        let register = parts[0].trim().chars().next().expect("string is empty");
                        let condition = parts[1].trim();
                        let value: u8 = match condition.parse() {
                            Ok(value) => value,
                            Err(_) => {
                                println!("Invalid condition value: {}", condition);
                                return 0;
                            }
                        };
    
                        // Perform the desired comparison based on the register and condition
                        
                        while State8080::get_reg(emu8080, register) != value {
                            state8080::emulate_8080_op(emu8080);
                        }
                            
    
                        return 0;
                    } else {
                        println!("Invalid condition format: {}", arg);
                        return 0;
                    }
                } else {
                    println!("Missing argument for 'cnd' command");
                    return 0;
                }
            }
            "status" => {
                state8080::print_state(emu8080);
                //return 1 to do nothing
                return 1;
            }
            "help" => {
                println!("Available commands:");
                println!("quit - Quit the program");
                println!("run <n> - Run the program for n instructions");
                println!("status - Display current register/system status");
                println!("help - Display information about the commands");
                // Return 1 to indicate successful execution of the "help" command
                return 1;
            }
            _ => {
                println!("Unknown command: {}", cmd);
                // Return an error code or handle the unknown command case as desired
                return 1;
            }
        }
    }

    // default case, step and do nothing
    0
}