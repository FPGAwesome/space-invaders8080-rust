const MEMORY_SIZE: usize = 0x4000; // Total memory size 0x0000-0x3fff main memory
                                   // 0x4000- RAM mirror

pub struct Memory {
    memory: [u8; MEMORY_SIZE],
}

fn mirror_address(address: u16) -> u16 {
    // Determine the mirrored address based on the memory layout
    match address {
        0x0000..=0x1FFF => address, // ROM
        0x2000..=0x23FF => address, // RAM
        0x2400..=0x3FFF => address, // Video RAM
        0x4000..=0x7FFF => address - 0x2000, // Mirror for ROM, RAM and start of Video RAM
        _ => address - 0x2000, // Mirror for rest of Video RAM
    }
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            memory: [0; MEMORY_SIZE],
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        let mirrored_address = mirror_address(address);
        self.memory[mirrored_address as usize]
    }

    pub fn read_byte_chunk(&self, start_address: u16, end_address: u16) -> &[u8] {
        &self.memory[start_address as usize..=end_address as usize]
    }

    // special way for us to write our file to ROM
    pub fn rom_write_byte(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value;
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        let mirrored_address = mirror_address(address);
        //no write to ROM
        if address > 0x1FFF{
            self.memory[mirrored_address as usize] = value; 
        }
    }

    
}