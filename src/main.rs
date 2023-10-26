mod disassemble;
mod memory;
mod state8080;
mod debugger;

use std::fs::File;
use std::io::{Read,Write};
use std::time::{Instant, Duration};
use std::thread::{self, Thread};

use disassemble::process_instruction;

use debugger::parse_command;

use minifb::{Window, WindowOptions, Key};

use queues::*;

use crate::state8080::State8080;

const WIDTH: usize = 256;
const HEIGHT: usize = 224; //224
const FRAME_TIME: Duration = Duration::from_nanos(16666667); // 60 Hz frame time
const FRAME_TIME120: Duration = Duration::from_nanos(8333333); // 120 Hz frame time
const DEBUG: bool = false;

// This will read in a hexdump file, parse each opcode, and write out to
// a file
fn parse_file(infile: &'static str, outfile: &'static str) {

    // Open file 'infile' for reading
    let mut file = match File::open(infile) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file: {}", err);
            return;
        }
    };

    // store our vector of opcodes
    let mut buffer = Vec::new();

    // read the file into our byte buffer
    match file.read_to_end(&mut buffer) {
        Ok(_) => {
            // As this is now, it will simply
            // dump all the hex to a file
            //let hex_string = hex::encode(&buffer);

            // create our output file
            let mut outfile = match File::create(outfile) {
                Ok(outfile) => outfile,
                Err(err) => {
                    println!("Error creating file: {}", err);
                    return;
                }
            };

            //disassemble relevant hex into the human-readable instruction
            let mut i = 0;
            while i < buffer.len() {

                let opcode = buffer[i];

                let next_bytes = if i + 2 < buffer.len() {
                    &buffer[i + 1..=i + 2]
                } else if i < buffer.len() {
                    &buffer[i + 1..]
                } else {
                    &[0, 0] // Default to [0, 0] when there are no more bytes left
                };

                let (opbytes,operation)=process_instruction(opcode,next_bytes);
                let _ = outfile.write_all(format!("PC {:04X}: ",i).as_bytes());
                let _ = outfile.write_all(operation.as_bytes());
                let _ = outfile.write_all(b"\n");

                i=i+opbytes; // Skip any bytes we've used as direct data
            }

            let _ = outfile.flush();
        }
        Err(err) => {
            println!("Error reading file: {}", err);
        }
    }
}

// Read the file into a byte vector
fn read_file(infile: &'static str) -> Result<Vec<u8>, std::io::Error> {
    // Open file 'infile' for reading
    let mut file = File::open(infile)?;

    // Store the vector of opcodes
    let mut buffer = Vec::new();

    // Read the file into our byte buffer
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

fn dump_bytes_to_file(bytes: &[u8], file_path: &str) {
    let mut file = File::create(file_path).expect("Failed to create file.");
    file.write_all(bytes).expect("Failed to write file");
}
fn main() {

    //profiling code
    // Variables for measuring elapsed time
    let mut start_time = Instant::now();
    let mut instruction_count = 0;

    // Attempt at some timing code for emulator precision
    let target_frequency: f64 = 2000000.0; // 2 MHz target frequency
    let cycles_per_frame: f64 = target_frequency / 60.0; // Assuming 60 frames per second

    // Create a window
    let mut window = Window::new(
        "Simple Graphics Example",
        //swapped em
        HEIGHT,
        WIDTH,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut swapInterrupt = false;

    parse_file("invaders", "invaders.8080"); // for disassembly

    // for actual emulation
    let bytes = match read_file("invaders") {
        Ok(bytes) => {
            bytes
        }
        Err(err) => {
            println!("Error reading file: {}", err);
            return;
        }
    };

    // first TODO is going to be writing the space invaders game
    // to memory
    let mut emu8080 = state8080::State8080::default();

    for (address, byte) in bytes.iter().enumerate() {
        emu8080.write_rom_mem(address as u16, *byte);
    }

    let mut last_frame_time = Instant::now();
    let mut lastInterrupt = Instant::now(); // kick off a timer for interrupts
    let mut intCounter = Duration::default();

    println!("Starting debug loop, enter 'help' to display debug commands.");
    //let mut last_instructions: Vec<String> = vec![];
    let mut q: Queue<String> = queue![];
    let mut total_cycles=0;

    loop {
        let mut ret_code=0;
        let frame_start_time = Instant::now();

        
        last_frame_time = Instant::now();

        // EMULATION BLOCK
        // Emulate instructions for the current frame
        let mut cycles_executed: f64 = 0.0;
        while cycles_executed < cycles_per_frame {
            
            // Emulate an instruction
            let (_, mut diss) = disassemble::process_instruction(emu8080.read_mem(emu8080.get_pc()), emu8080.read_mem_chunk(emu8080.get_pc()+1, emu8080.get_pc()+2));
            diss = format!("{:04X}: {}, Frame cycles thus far: {}",emu8080.get_pc(),diss,cycles_executed);
            q.add(diss);
            if q.size() > 1000 {
                q.remove();
            }
            let a = state8080::emulate_8080_op(&mut emu8080) as f64;
            instruction_count+=a as i32;
            cycles_executed += a;
            total_cycles+=a as i32;

            // if emu8080.get_pc()==0x09EE {
            //     break;
            // }
            let elapsed = lastInterrupt.elapsed();
            // Accumulate the frame time
            intCounter = elapsed;
            // Check timers and handle their interrupts if necessary
            if emu8080.interrupt_enabled() && total_cycles > 16667{
                lastInterrupt = Instant::now();
                //intCounter -= FRAME_TIME120;
                total_cycles=0;

                if swapInterrupt {
                    state8080::generate_interrupt(&mut emu8080, 2);
                } else {
                    state8080::generate_interrupt(&mut emu8080, 1);
                }

                swapInterrupt = !swapInterrupt;
            }
        }
        // if emu8080.get_pc()==0x09EE {
        //     break;
        // }

        draw_screen(&mut emu8080, &mut window);

        // Sleep to maintain the target frequency
        let target_frame_time = Duration::from_secs_f64(1.0 / 60.0);//Duration::from_secs_f64(1.0 / target_frequency);
        let target_time = frame_start_time + target_frame_time;
        loop {
            if Instant::now() >= target_time {
                break;
            }
        }
        
        // Check if a second has passed
        if start_time.elapsed() >= Duration::from_secs(1) {
            // Calculate the MHz based on the instruction count
            let mhz = instruction_count as f64 / 1_000_000.0;
            
            // Print the MHz value
            println!("Current speed: {:.2} MHz", mhz);
            
            // Reset the timer and instruction count
            start_time = Instant::now();
            instruction_count = 0;
        }
                  
    }

    let mut file = File::create("instruction_dump_last1000.txt").unwrap();

    for i in 0..1000  {
        let string = q.remove().unwrap();
        file.write_all(string.as_bytes()).unwrap();
        file.write_all(b"\n").unwrap();
    }
}


fn draw_screen(state: &mut State8080, window: &mut Window) {
    let vram_chunk=state.read_mem_chunk(0x2400, 0x3FFF);
    let mut buffidx = 0;

    // Create a buffer to store the pixel data
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    
    // First loop for the first half of vram_chunk
    for v in vram_chunk {
        for i in 0..8 {
            let bit = 0b00000001 << i;
            buffer[buffidx] = if bit & v != 0 { 0xFFFFFFFF } else { 0xFF000000 };
            buffidx += 1;
        }
    }

    // Interpret the input 1D buffer as a 2D grid (a vector of vectors).
    // The chunks_exact() method splits our buffer into chunks of "WIDTH"-length, effectively giving us the rows of our grid.
    // .map(|chunk| chunk.to_vec()) simply converts each chunk (or row) into a vector.
    // .collect() finally gathers these vectors of rows into a single vector.
    let buffer_grid: Vec<Vec<u32>> = buffer
    .chunks_exact(WIDTH)
    .map(|chunk| chunk.to_vec())
    .collect();

    // Rotate the 2D grid counterclockwise.
    // (0..WIDTH).rev() generates a reversed range of column indices.
    // The .flat_map call then takes each reversed column index and goes through each row at that column index.
    // This iterates bottom to top, so we're effectively taking columns from the right of the original image and appending them to the new image, achieving a 90 degree counterclockwise rotation.
    // .collect() then gathers these into a single flat vector.
    let rotated_grid: Vec<u32> = (0..WIDTH).rev()
    .flat_map(|x| buffer_grid.iter().map(move |row| row[x]))
    .collect();

    //cleaner to put this here I guess

    // Update the window with the buffer contents
    //width and height had to be swapped, this cause so many issues. I hate 1D bitmaps <3
    window.update_with_buffer(&rotated_grid, HEIGHT,WIDTH ).unwrap();

    window.get_keys_pressed(minifb::KeyRepeat::No).iter().for_each(|key| match key {
        Key::C => {
            let coin_counter = state.port.io_ports.entry(1).or_insert(0);
            *coin_counter |= 1; // Set bit 0
        }, // do whatever io port thing
        _ => (),
    });

    window.get_keys_released().iter().for_each(|key|
        match key {
            Key::C => { 
                let coin_counter = state.port.io_ports.entry(1).or_insert(0);
                *coin_counter &= !1; // Clear bit 0
            }
            _ => (),
        }
    );

    window.get_keys_pressed(minifb::KeyRepeat::No).iter().for_each(|key| match key {
        Key::Enter => {
            let coin_counter = state.port.io_ports.entry(1).or_insert(0);
            *coin_counter |= 4; // Set bit 0
        }, // do whatever io port thing
        _ => (),
    });

    window.get_keys_released().iter().for_each(|key|
        match key {
            Key::Enter => { 
                let coin_counter = state.port.io_ports.entry(1).or_insert(0);
                *coin_counter &= !4; // Clear bit 0
            }
            _ => (),
        }
    );

    window.get_keys_pressed(minifb::KeyRepeat::No).iter().for_each(|key| match key {
        Key::A => {
            let coin_counter = state.port.io_ports.entry(1).or_insert(0);
            *coin_counter |= 0x20; // Set bit 0
        }, // do whatever io port thing
        _ => (),
    });

    window.get_keys_released().iter().for_each(|key|
        match key {
            Key::A => { 
                let coin_counter = state.port.io_ports.entry(1).or_insert(0);
                *coin_counter &= !0x20; // Clear bit 0
            }
            _ => (),
        }
    );

    window.get_keys_pressed(minifb::KeyRepeat::No).iter().for_each(|key| match key {
        Key::D => {
            let coin_counter = state.port.io_ports.entry(1).or_insert(0);
            *coin_counter |= 0x40; // Set bit 0
        }, // do whatever io port thing
        _ => (),
    });

    window.get_keys_released().iter().for_each(|key|
        match key {
            Key::D => { 
                let coin_counter = state.port.io_ports.entry(1).or_insert(0);
                *coin_counter &= !0x40; // Clear bit 0
            }
            _ => (),
        }
    );

    window.get_keys_pressed(minifb::KeyRepeat::No).iter().for_each(|key| match key {
        Key::Space => {
            let coin_counter = state.port.io_ports.entry(1).or_insert(0);
            *coin_counter |= 0x10; // Set bit 0
        }, // do whatever io port thing
        _ => (),
    });

    window.get_keys_released().iter().for_each(|key|
        match key {
            Key::Space => { 
                let coin_counter = state.port.io_ports.entry(1).or_insert(0);
                *coin_counter &= !0x10; // Clear bit 0
            }
            _ => (),
        }
    );
}