#[derive(Clone, Copy)]
pub struct ContentIter<'a> {
    content: &'a [u8],
    address: usize,
    bit_address: usize
}

macro_rules! read_number {
    ($name: tt, $type: ident::$method: ident, $bytes: expr) => {
        #[allow(dead_code)]
        pub fn $name(&mut self, reading: &str) -> Result<$type, String> {
            let mut buffer: [u8; $bytes] = [0; $bytes];
            self
                .next_slice_of($bytes, reading)?
                .iter()
                .enumerate()
                .for_each(|(i, val)| buffer[i] = *val);
            Ok($type::$method(buffer))
        }
        
    }
}

impl<'a> ContentIter<'a> {
    pub fn new(contents: &'a [u8]) -> Self {
        Self {
            content: contents,
            address: 0x0,
            bit_address: 0x0
        }
    }

    #[allow(dead_code)]
    pub fn get_address(&self) -> usize {
        self.address
    }

    #[allow(dead_code)]
    pub fn read_utf8_str(&mut self, n: usize, reading: &str) -> Result<&'a str, String> {
        let bytes = self.next_slice_of(n, reading)?;
        let result = std::str::from_utf8(bytes)
            .map_err(|err| format!("{err} at byte 0x{:0>2x}", self.get_address()))?;
        Ok(result)
    }
    read_number!(read_u8_be, u8::from_be_bytes, 1);
    read_number!(read_u8_le, u8::from_le_bytes, 1);
    
    read_number!(read_u16_be, u16::from_be_bytes, 2);
    read_number!(read_u16_le, u16::from_le_bytes, 2);

    read_number!(read_u32_be, u32::from_be_bytes, 4);
    read_number!(read_u32_le, u32::from_le_bytes, 4);
    
    read_number!(read_u64_be, u64::from_be_bytes, 8);
    read_number!(read_u64_le, u64::from_le_bytes, 8);


    #[allow(dead_code)]
    pub fn next_slice_of(&mut self, n: usize, reading: &str) -> Result<&'a [u8], String> {
        let new_address = self.address.checked_add(n).ok_or(format!("Failed with overflow to add {n} to usize {}", self.address))?;
        let result = self.content.get(self.address..new_address);
        self.address = new_address;
        result.ok_or(format!(
            "Could not read {n} bytes for \"{reading}\" at address: 0x{:0>2x}. Content vector is smaller (size: {} bytes)",
            self.address-n, 
            self.content
                .get(self.address - n..)
                .unwrap_or_else(|| &[])
                .len()
        ))
    }

    #[allow(dead_code)]
    pub fn skip_remaining_bits(&mut self) -> Result<Vec<u8>, String> {
        let remaining = 8 - self.bit_address;
        let res = self.next_bit_slice_of(remaining, &format!("Skipping {remaining} bits to byte {}", self.address ));
        self.bit_address = 0;
        self.address += 1;
        res
    } 

    #[allow(dead_code)]
    pub fn next_bit_slice_of(&mut self, n: usize, reading: &str) -> Result<Vec<u8>, String> {
        let mut result = vec![];
        for _ in 0..n {

            let get_byte = |n| self.content.get(n).ok_or(format!(
                "Could not read byte for \"{reading}\" at address: 0x{:0>2x}. The contents size is (size: {} bytes)",
                n,
                self.content.len()
            ));

            let (bit, new_address) = get_byte(self.address)?.checked_shr(self.bit_address as u32).ok_or("Failed to shift value".to_string()).map_or_else(
            |_| -> Result<(u8, usize), String> {
                self.address += 1;
                Ok((get_byte(self.address)? >> 0x0, 0x1))
            },
            |val| -> Result<(u8, usize), String> {
                Ok((val, self.bit_address + 1))
            })?;

            result.push(bit & 1);
            self.bit_address = new_address;
        }

        
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn next_bit(&mut self, reading: &str) -> Result<u8, String> {
        self.next_bit_slice_of(1, reading).map(|slice| slice[0])
    }

    #[allow(dead_code)]
    pub fn prev_slice_of(&mut self, n: usize, reading: &str) -> Result<&'a [u8], String> {
        let new_address = self.address.checked_sub(n).ok_or(format!("Failed with overflow to substract {n} to usize {}", self.address))?;
        let result = self.content.get(new_address..self.address);
        self.address = new_address;
        result.ok_or(format!(
            "Could not read previous {n} bytes for \"{reading}\" at address 0x{:0>2x}. Content vector is smaller (size: {} bytes)",
            self.address+n,
            self.content
                .get(..self.address + n)
                .unwrap_or_else(|| &[])
                .len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_bits() {
        let slice = &[0b11000101u8, 32, 0b11100110, 0b00101100];
        let mut iterator = ContentIter::new(&slice[..]);
    
        let first_3_bits = iterator.next_bit("1 First bits");
        assert_eq!(first_3_bits, Ok(1));
        
        let first_3_bits = iterator.next_bit_slice_of(2, "2 First bits");
        assert_eq!(first_3_bits, Ok(vec![0,1]));
        
        let first_3_bits = iterator.skip_remaining_bits();
        assert_eq!(first_3_bits, Ok(vec![0,0,0,1,1]));

        let read_u8 = iterator.read_u8_be("U8 test");
        assert_eq!(read_u8, Ok(32));
        
        let first_3_bits = iterator.next_bit_slice_of(8, "8 first bits");
        assert_eq!(first_3_bits, Ok(vec![0,1,1,0,0,1,1,1]));

        let first_3_bits = iterator.next_bit_slice_of(8, "8 first bits");
        assert_eq!(first_3_bits, Ok(vec![0,0,1,1,0,1,0,0]));
        
        let first_3_bits = iterator.next_bit_slice_of(1, "1 first bits");
        assert_eq!(first_3_bits, Err("Could not read byte for \"1 first bits\" at address: 0x04. The contents size is (size: 4 bytes)".to_string()));
    }
}